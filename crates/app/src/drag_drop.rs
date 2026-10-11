use crate::{
    panels::LINE,
    workspace::{Gesture, Tool, Workspace},
};
use document::{NodeId, Point as DocPoint, Rect};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct LayerDrag {
    pub id: NodeId,
    pub epoch: u64,
    pub page: NodeId,
    pub name: String,
}
#[derive(Clone, Copy)]
pub enum InsertKind {
    Tool(Tool),
    Component(editor::Component),
    Image,
}
#[derive(Clone, Copy)]
pub struct InsertDrag {
    pub kind: InsertKind,
    pub epoch: u64,
}
pub struct DragLabel {
    pub name: String,
    pub position: Point<Pixels>,
}
impl Render for DragLabel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .ml(-self.position.x)
            .mt(-self.position.y)
            .px(px(12.))
            .py(px(8.))
            .bg(white())
            .border_1()
            .border_color(rgb(LINE))
            .rounded(px(6.))
            .shadow_md()
            .text_size(px(12.))
            .child(self.name.clone())
    }
}
impl InsertKind {
    fn name(self) -> &'static str {
        match self {
            Self::Tool(Tool::Text) => "Text",
            Self::Tool(Tool::Shape) => "Shape",
            Self::Tool(Tool::Frame) => "Frame",
            Self::Tool(Tool::Select) => "Select",
            Self::Component(c) => c.name(),
            Self::Image => "Image",
        }
    }
}
impl Workspace {
    pub fn draggable_insert(
        &self,
        button: Stateful<Div>,
        kind: InsertKind,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        if self.word_mode()
            && matches!(
                kind,
                InsertKind::Tool(Tool::Shape | Tool::Frame) | InsertKind::Component(_)
            )
        {
            return button;
        }
        let entity = cx.entity();
        let payload = InsertDrag {
            kind,
            epoch: self.drag_epoch,
        };
        button.on_drag(payload, move |value, position, _, cx| {
            entity.update(cx, |s, _| {
                s.end_edit();
                s.gesture = Gesture::None;
            });
            cx.new(|_| DragLabel {
                name: value.kind.name().into(),
                position,
            })
        })
    }
    pub fn drop_insert(
        &mut self,
        payload: &InsertDrag,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.preview || self.busy || self.pending.is_some() || self.drag_epoch != payload.epoch {
            return;
        }
        let p = self.to_doc(position);
        let page = &self.editor.document.pages[self.editor.page];
        if !Rect::new(0., 0., page.width, page.height).contains(p) {
            self.set_status("Drop inside the page to insert", false);
            return;
        }
        self.close_popup();
        self.end_edit();
        self.gesture = Gesture::None;
        match payload.kind {
            InsertKind::Tool(tool) => self.insert_at(tool, p),
            InsertKind::Component(c) if !self.word_mode() => {
                self.editor.insert_component(c, p.x, p.y);
                self.assets_tab = false;
            }
            InsertKind::Image => {
                self.open_image_picker(cx);
                self.image_picker.insert_point = Some(p);
            }
            _ => {
                self.set_status("This element needs Compose mode", true);
                return;
            }
        }
        self.measure();
        cx.notify();
    }
    pub fn drop_layer_row(
        &mut self,
        payload: &LayerDrag,
        target: NodeId,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.preview
            || self.busy
            || self.pending.is_some()
            || payload.epoch != self.drag_epoch
            || payload.page != self.editor.document.pages[self.editor.page].id
        {
            return;
        }
        let Some(bounds) = self.layer_bounds.get(&target) else {
            return;
        };
        let above = position.y < bounds.center().y;
        let before = if self.word_mode() { above } else { !above };
        self.close_popup();
        self.end_edit();
        self.gesture = Gesture::None;
        if self.editor.drop_layer(payload.id, target, before) {
            self.measure();
            self.set_status(
                if self.word_mode() {
                    "Reading order updated"
                } else {
                    "Layer order updated"
                },
                false,
            );
        } else {
            self.set_status(
                "Cannot move a locked layer or place a group inside itself",
                false,
            );
        }
        cx.notify();
    }
    pub fn import_dropped_files(
        &mut self,
        paths: &ExternalPaths,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.preview || self.busy || self.pending.is_some() {
            return;
        }
        let files = paths.paths().to_vec();
        if files.len() == 1
            && files[0]
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("compose"))
        {
            self.request(crate::workspace::Pending::Open(files[0].clone()), cx);
            return;
        }
        if files.is_empty() || files.len() > 16 {
            self.set_status("Drop up to 16 image files at once", true);
            return;
        }
        let p = self.to_doc(position);
        let page = &self.editor.document.pages[self.editor.page];
        if !Rect::new(0., 0., page.width, page.height).contains(p) {
            self.set_status("Drop images inside the page", false);
            return;
        }
        self.close_popup();
        self.end_edit();
        self.gesture = Gesture::None;
        let epoch = self.drag_epoch;
        self.busy = true;
        self.set_status("Reading dropped images…", false);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    files
                        .into_iter()
                        .map(|path| {
                            anyhow::ensure!(
                                std::fs::metadata(&path)?.len() <= 64 * 1024 * 1024,
                                "Image exceeds 64 MB"
                            );
                            let bytes = std::fs::read(&path)?;
                            let (image, w, h) = Workspace::decode_image(&bytes)?;
                            Ok::<_, anyhow::Error>((
                                bytes,
                                image,
                                w,
                                h,
                                path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned(),
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .await;
            this.update(cx, |s, cx| {
                s.busy = false;
                if s.drag_epoch != epoch {
                    return;
                }
                match result {
                    Ok(images) => {
                        let total = s
                            .editor
                            .document
                            .assets
                            .values()
                            .map(|b| b.len() as u64)
                            .sum::<u64>()
                            + images.iter().map(|i| i.0.len() as u64).sum::<u64>();
                        if total > 256 * 1024 * 1024 {
                            s.set_status("Images exceed the 256 MB document limit", true);
                            cx.notify();
                            return;
                        }
                        let count = images.len();
                        s.editor.begin_batch();
                        for (i, (bytes, image, w, h, name)) in images.into_iter().enumerate() {
                            s.image_picker.insert_point = Some(DocPoint {
                                x: p.x + i as f32 * 20.,
                                y: p.y + i as f32 * 20.,
                            });
                            s.insert_decoded_image(bytes, image, w, h, name, None);
                        }
                        s.editor.end_batch();
                        s.image_picker.insert_point = None;
                        s.measure();
                        s.set_status(
                            format!(
                                "{count} image{} dropped onto the page",
                                if count == 1 { "" } else { "s" }
                            ),
                            false,
                        );
                    }
                    Err(e) => s.set_status(format!("Could not import dropped files: {e:#}"), true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
