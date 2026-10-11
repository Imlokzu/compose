use crate::workspace::{Popup, Workspace};
use document::{ImageSource, NodeKind, Rect};
use gpui::*;
use image_search::{Provider, SearchClient, SearchImage};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageTab {
    Computer,
    Search,
}
#[derive(Clone)]
pub struct ImageResult {
    pub item: SearchImage,
    pub thumbnail: Option<Arc<RenderImage>>,
    pub thumbnail_bytes: Option<Arc<Vec<u8>>>,
    pub thumbnail_failed: bool,
}
pub struct ImagePicker {
    pub insert_point: Option<document::Point>,
    pub provider: Provider,
    pub replace_target: Option<document::NodeId>,
    pub tab: ImageTab,
    pub query: String,
    pub searched: String,
    pub results: Vec<ImageResult>,
    pub selected: Option<u64>,
    pub loading: bool,
    pub inserting: bool,
    pub error: Option<String>,
    pub next: Option<u64>,
    pub generation: u64,
    pub cancel: Arc<AtomicBool>,
}
impl Default for ImagePicker {
    fn default() -> Self {
        Self {
            insert_point: None,
            provider: Provider::Web,
            replace_target: None,
            tab: ImageTab::Computer,
            query: String::new(),
            searched: String::new(),
            results: vec![],
            selected: None,
            loading: false,
            inserting: false,
            error: None,
            next: None,
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl ImagePicker {
    pub fn invalidate(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.generation = self.generation.wrapping_add(1);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.loading = false;
        self.inserting = false;
    }
    pub fn selected_result(&self) -> Option<&ImageResult> {
        self.results
            .iter()
            .find(|r| Some(r.item.id) == self.selected)
    }
    pub fn accepts(&self, generation: u64) -> bool {
        self.generation == generation && !self.cancel.load(Ordering::Relaxed)
    }
}
impl Workspace {
    pub fn open_image_picker(&mut self, cx: &mut Context<Self>) {
        self.image_picker.insert_point = None;
        self.close_popup();
        self.image_picker.invalidate();
        self.image_picker.replace_target = None;
        self.end_edit();
        self.popup = Some(Popup::Images);
        self.image_picker.error = None;
        if self.image_picker.tab == ImageTab::Search {
            self.load_image_thumbnails(self.image_picker.generation, cx);
        }
    }
    pub fn open_image_replacement(&mut self, cx: &mut Context<Self>) {
        if self.preview || self.selection_locked() {
            return;
        }
        let Some(node) = self.editor.selected_node() else {
            return;
        };
        if !matches!(node.kind, NodeKind::Image { .. }) {
            return;
        }
        let id = node.id;
        self.open_image_picker(cx);
        self.image_picker.replace_target = Some(id);
    }
    pub fn set_image_tab(&mut self, tab: ImageTab, cx: &mut Context<Self>) {
        self.commit_field();
        self.image_picker.invalidate();
        self.image_picker.tab = tab;
        self.image_picker.error = None;
        if tab == ImageTab::Search {
            self.load_image_thumbnails(self.image_picker.generation, cx);
        }
    }
    pub fn set_image_provider(&mut self, provider: Provider, cx: &mut Context<Self>) {
        if self.image_picker.provider == provider || self.image_picker.inserting {
            return;
        }
        self.commit_field();
        self.image_picker.invalidate();
        self.image_picker.provider = provider;
        self.image_picker.results.clear();
        self.image_picker.selected = None;
        self.image_picker.searched.clear();
        self.image_picker.next = None;
        self.image_picker.error = None;
        if !self.image_picker.query.trim().is_empty() {
            self.search_images(false, cx);
        }
        cx.notify();
    }
    pub fn search_images(&mut self, more: bool, cx: &mut Context<Self>) {
        self.commit_field();
        if self.popup != Some(Popup::Images) || self.image_picker.inserting {
            return;
        }
        let query = if more {
            self.image_picker.searched.clone()
        } else {
            self.image_picker.query.trim().to_string()
        };
        let offset = if more {
            let Some(next) = self.image_picker.next else {
                return;
            };
            next
        } else {
            0
        };
        let provider = self.image_picker.provider;
        if let Err(e) = image_search::provider_search_url(provider, &query, offset) {
            self.image_picker.error = Some(e.to_string());
            cx.notify();
            return;
        }
        self.image_picker.invalidate();
        let generation = self.image_picker.generation;
        let cancel = self.image_picker.cancel.clone();
        if !more {
            self.image_picker.results.clear();
            self.image_picker.selected = None;
            self.image_picker.next = None;
        }
        self.image_picker.loading = true;
        self.image_picker.error = None;
        self.image_picker.searched = query.clone();
        let task = cx.background_executor().spawn(async move {
            if cancel.load(Ordering::Relaxed) {
                return Err(anyhow::anyhow!("Search cancelled"));
            }
            SearchClient::with_provider(provider)?.search(&query, offset)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |s, cx| {
                if s.popup != Some(Popup::Images) || !s.image_picker.accepts(generation) {
                    return;
                }
                s.image_picker.loading = false;
                match result {
                    Ok(page) => {
                        s.image_picker.next = page.next;
                        let mut seen: std::collections::HashSet<_> =
                            s.image_picker.results.iter().map(|r| r.item.id).collect();
                        for item in page.images {
                            if seen.insert(item.id) {
                                s.image_picker.results.push(ImageResult {
                                    item,
                                    thumbnail: None,
                                    thumbnail_bytes: None,
                                    thumbnail_failed: false,
                                });
                            }
                        }
                        s.load_image_thumbnails(generation, cx);
                    }
                    Err(e) => s.image_picker.error = Some(format!("{e:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn load_image_thumbnails(&mut self, generation: u64, cx: &mut Context<Self>) {
        let items: Vec<_> = self
            .image_picker
            .results
            .iter()
            .filter(|r| r.thumbnail.is_none() && !r.thumbnail_failed)
            .map(|r| r.item.clone())
            .collect();
        let cancel = self.image_picker.cancel.clone();
        cx.spawn(async move |this, cx| {
            // Four at a time: small bounded downloads, no UI-thread image decoding.
            for batch in items.chunks(4) {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let tasks: Vec<_> = batch
                    .iter()
                    .cloned()
                    .map(|item| {
                        let cancel = cancel.clone();
                        cx.background_executor().spawn(async move {
                            let result = (|| -> anyhow::Result<_> {
                                anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Cancelled");
                                let bytes = SearchClient::new()?.thumbnail(&item)?;
                                let reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
                                    .with_guessed_format()?;
                                let (w, h) = reader.into_dimensions()?;
                                anyhow::ensure!(
                                    u64::from(w) * u64::from(h) <= 2_000_000,
                                    "Preview is too large"
                                );
                                let image = Self::decode_image(&bytes)?.0;
                                Ok((image, Arc::new(bytes)))
                            })();
                            (item.id, result)
                        })
                    })
                    .collect();
                for task in tasks {
                    let (id, result) = task.await;
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    if this
                        .update(cx, |s, cx| {
                            if s.popup != Some(Popup::Images) || !s.image_picker.accepts(generation)
                            {
                                return;
                            }
                            if let Some(r) =
                                s.image_picker.results.iter_mut().find(|r| r.item.id == id)
                            {
                                match result {
                                    Ok((image, bytes)) => {
                                        r.thumbnail = Some(image);
                                        r.thumbnail_bytes = Some(bytes);
                                    }
                                    Err(_) => r.thumbnail_failed = true,
                                }
                            }
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                }
            }
        })
        .detach();
    }
    pub fn insert_search_image(&mut self, cx: &mut Context<Self>) {
        self.insert_search_image_with_cutout(false, cx);
    }
    pub fn insert_search_image_with_cutout(&mut self, cutout: bool, cx: &mut Context<Self>) {
        if cutout && self.background_service.is_none() {
            self.open_background_settings();
            cx.notify();
            return;
        }
        let endpoint = self.background_service.clone();
        if self.image_picker.inserting || self.popup != Some(Popup::Images) {
            return;
        }
        let Some(result) = self.image_picker.selected_result() else {
            return;
        };
        let item = result.item.clone();
        self.commit_field();
        self.image_picker.invalidate();
        let generation = self.image_picker.generation;
        let cancel = self.image_picker.cancel.clone();
        let page_id = self.editor.document.pages[self.editor.page].id;
        self.image_picker.inserting = true;
        self.image_picker.error = None;
        let task = cx.background_executor().spawn(async move {
            let result = (|| -> anyhow::Result<_> {
                anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Cancelled");
                let mut bytes = SearchClient::new()?.original(&item)?;
                anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Cancelled");
                if cutout {
                    bytes = image_tools::remove_shared(&bytes, endpoint.as_deref().unwrap())?;
                    anyhow::ensure!(!cancel.load(Ordering::Relaxed), "Cancelled");
                }
                let (image, w, h) = Self::decode_image(&bytes)?;
                Ok((bytes, image, w, h))
            })();
            (item, result)
        });
        cx.spawn(async move |this, cx| {
            let (item, result) = task.await;
            this.update(cx, |s, cx| {
                if s.popup != Some(Popup::Images)
                    || !s.image_picker.accepts(generation)
                    || s.editor.document.pages[s.editor.page].id != page_id
                {
                    return;
                }
                s.image_picker.inserting = false;
                match result {
                    Ok((bytes, image, w, h)) => {
                        let source = ImageSource {
                            provider: item.provider.name().into(),
                            page_url: item.source_url,
                            original_url: item.original_url,
                            author: item.author,
                            license: item.license,
                            license_url: item.license_url,
                            credit: item.credit,
                        };
                        match s.apply_picker_image(bytes, image, w, h, item.title, Some(source)) {
                            Ok(()) => {
                                s.close_popup();
                                if cutout {
                                    s.set_status(
                                        "Image inserted without background · free preview resolution",
                                        false,
                                    );
                                }
                            }
                            Err(e) => {
                                s.image_picker.error =
                                    Some(format!("Could not use this image: {e:#}"))
                            }
                        }
                    }
                    Err(e) => {
                        s.image_picker.error = Some(format!("Could not insert this image: {e:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    pub fn insert_search_preview(&mut self, cx: &mut Context<Self>) {
        if self.image_picker.inserting || self.popup != Some(Popup::Images) {
            return;
        }
        let Some(result) = self.image_picker.selected_result() else {
            return;
        };
        let Some(bytes) = result.thumbnail_bytes.as_ref() else {
            return;
        };
        let item = result.item.clone();
        let bytes = bytes.as_ref().clone();
        let Some(image) = result.thumbnail.clone() else {
            return;
        };
        let size = image.size(0);
        let (w, h) = (size.width.0 as u32, size.height.0 as u32);
        let source = ImageSource {
            provider: item.provider.name().into(),
            page_url: item.source_url,
            original_url: item.thumbnail_url,
            author: item.author,
            license: item.license,
            license_url: item.license_url,
            credit: format!("Search preview · original image: {}", item.original_url),
        };
        match self.apply_picker_image(bytes, image, w, h, item.title, Some(source)) {
            Ok(()) => {
                self.close_popup();
                self.set_status("Search preview inserted · lower resolution", false);
            }
            Err(e) => self.image_picker.error = Some(e.to_string()),
        }
        cx.notify();
    }
    pub fn load_local_image(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.image_picker.invalidate();
        let generation = self.image_picker.generation;
        self.image_picker.inserting = true;
        self.image_picker.error = None;
        let task = cx.background_executor().spawn(async move {
            (|| -> anyhow::Result<_> {
                anyhow::ensure!(
                    std::fs::metadata(&path)?.len() <= 64 * 1024 * 1024,
                    "Image exceeds 64 MB"
                );
                let bytes = std::fs::read(&path)?;
                let (image, w, h) = Self::decode_image(&bytes)?;
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                Ok((bytes, image, w, h, name))
            })()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |s, cx| {
                if s.popup != Some(Popup::Images) || !s.image_picker.accepts(generation) {
                    return;
                }
                s.image_picker.inserting = false;
                match result {
                    Ok((bytes, image, w, h, name)) => {
                        match s.apply_picker_image(bytes, image, w, h, name, None) {
                            Ok(()) => s.close_popup(),
                            Err(e) => {
                                s.image_picker.error =
                                    Some(format!("Could not use this image: {e:#}"))
                            }
                        }
                    }
                    Err(e) => s.image_picker.error = Some(format!("Could not insert image: {e:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    pub fn insert_decoded_image(
        &mut self,
        bytes: Vec<u8>,
        image: Arc<RenderImage>,
        w: u32,
        h: u32,
        name: String,
        source: Option<ImageSource>,
    ) {
        use sha2::{Digest, Sha256};
        let key = format!("{:x}", Sha256::digest(&bytes));
        let scale = (360. / w as f32).min(360. / h as f32).min(1.);
        let (width, height) = (w as f32 * scale, h as f32 * scale);
        self.end_edit();
        self.editor.checkpoint();
        self.editor
            .document
            .assets
            .insert(key.clone(), Arc::new(bytes));
        let id = self.editor.document.insert(
            self.editor.page,
            NodeKind::Image { asset: key.clone() },
            {
                let p = self
                    .image_picker
                    .insert_point
                    .unwrap_or(document::Point { x: 64., y: 120. });
                let page = &self.editor.document.pages[self.editor.page];
                Rect::new(
                    p.x.clamp(0., (page.width - width).max(0.)),
                    p.y.clamp(0., (page.height - height).max(0.)),
                    width,
                    height,
                )
            },
        );
        self.editor.select_only(Some(id));
        let radius = if self.word_mode() { 0. } else { 12. };
        let node = self.editor.document.node_mut(id).unwrap();
        node.name = name;
        node.radius = radius;
        node.image_source = source;
        self.images.insert(key, image);
        self.measure();
    }
    pub fn apply_picker_image(
        &mut self,
        bytes: Vec<u8>,
        image: Arc<RenderImage>,
        w: u32,
        h: u32,
        name: String,
        source: Option<ImageSource>,
    ) -> anyhow::Result<()> {
        let Some(id) = self.image_picker.replace_target else {
            self.insert_decoded_image(bytes, image, w, h, name, source);
            self.set_status("Image inserted · saved with your document", false);
            return Ok(());
        };
        anyhow::ensure!(
            !self.preview && !self.editor.is_locked(id),
            "Image is locked"
        );
        let node = self
            .editor
            .document
            .node(id)
            .ok_or_else(|| anyhow::anyhow!("Image no longer exists"))?;
        anyhow::ensure!(
            matches!(node.kind, NodeKind::Image { .. }),
            "Layer is no longer an image"
        );
        anyhow::ensure!(w > 0 && h > 0, "Invalid image dimensions");
        let mut frame = node.frame;
        if node.image_sizing == document::ImageSizing::Scale {
            frame.height = frame.width * h as f32 / w as f32;
        }
        anyhow::ensure!(
            frame.valid(),
            "Replacement dimensions are outside the page limits"
        );
        use sha2::{Digest, Sha256};
        let key = format!("{:x}", Sha256::digest(&bytes));
        let total = self
            .editor
            .document
            .assets
            .iter()
            .filter(|(existing, _)| **existing != key)
            .map(|(_, b)| b.len() as u64)
            .sum::<u64>()
            + bytes.len() as u64;
        anyhow::ensure!(
            total <= 256 * 1024 * 1024,
            "Images exceed the 256 MB document limit"
        );
        self.editor.checkpoint();
        self.editor
            .document
            .assets
            .insert(key.clone(), Arc::new(bytes));
        let node = self.editor.document.node_mut(id).unwrap();
        node.kind = NodeKind::Image { asset: key.clone() };
        node.frame = frame;
        node.image_source = source;
        self.editor.select_only(Some(id));
        self.images.insert(key, image);
        self.set_status("Image replaced · position and style preserved", false);
        Ok(())
    }
    pub fn image_source_details(&self) -> Option<&ImageSource> {
        self.editor.selected_node()?.image_source.as_ref()
    }
}
#[cfg(test)]
mod tests {
    use super::ImagePicker;
    use std::sync::atomic::Ordering;
    #[test]
    fn closing_and_new_search_reject_old_results() {
        let mut p = ImagePicker::default();
        let old = p.generation;
        let cancel = p.cancel.clone();
        p.loading = true;
        p.inserting = true;
        p.invalidate();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(!p.accepts(old));
        assert!(p.accepts(p.generation));
        assert!(!p.loading && !p.inserting);
    }
}
