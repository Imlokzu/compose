//! Compose's shared service: the desktop app holds a public URL, never a provider key.
use crate::{
    panels::label,
    workspace::{Popup, Workspace},
};
use document::NodeKind;
use gpui::{prelude::*, *};
impl Workspace {
    pub fn load_background_service(&mut self) {
        match image_tools::configured_service_url() {
            Ok(endpoint) => self.background_service = endpoint,
            Err(_) => self.set_status(
                "Background removal is not configured correctly in this build",
                true,
            ),
        }
    }
    pub fn open_background_settings(&mut self) {
        self.close_popup();
        self.popup = Some(Popup::Background);
    }
    pub fn background_settings_view(&self, cx: &Context<Self>) -> Div {
        div().flex().flex_col().gap(px(12.))
            .child(label("Remove image backgrounds through Compose. No account or API key needed.").line_height(px(17.)))
            .child(label("Clicking Remove background uploads this image to the Compose service. It returns a transparent PNG up to 800 px. Shared usage limits apply; Undo keeps your original.").line_height(px(17.)))
            .when(self.background_service.is_none(),|d|d.child(div().text_color(rgb(0x9a563e)).line_height(px(17.)).child("The shared service has not been connected in this build yet. You can keep editing while it is being set up.")))
            .child(self.button("background-close","Got it",Some("check"),true,false,cx,|s,_,_|s.close_popup()))
    }
    pub fn remove_image_background(&mut self, cx: &mut gpui::Context<Self>) {
        if self.preview || self.selection_locked() || self.cutout_target.is_some() || self.busy {
            return;
        }
        if self.background_service.is_none() {
            self.open_background_settings();
            cx.notify();
            return;
        }
        let Some(node) = self.editor.selected_node() else {
            return;
        };
        let NodeKind::Image { asset } = &node.kind else {
            return;
        };
        let (id, asset, source) = (node.id, asset.clone(), node.image_source.clone());
        let Some(bytes) = self.editor.document.assets.get(&asset).cloned() else {
            return;
        };
        self.close_popup();
        self.end_edit();
        self.cutout_generation = self.cutout_generation.wrapping_add(1);
        let generation = self.cutout_generation;
        let endpoint = self.background_service.clone().unwrap();
        self.cutout_target = Some(id);
        self.set_status(
            "Starting background removal · the free service may need a moment…",
            false,
        );
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let bytes = image_tools::remove_shared(&bytes, &endpoint)?;
                    let (image, w, h) = Workspace::decode_image(&bytes)?;
                    Ok::<_, anyhow::Error>((bytes, image, w, h))
                })
                .await;
            this.update(cx, |s, cx| {
                if s.cutout_generation != generation {
                    return;
                }
                s.cutout_target = None;
                let unchanged = s
                    .editor
                    .document
                    .node(id)
                    .is_some_and(|n| matches!(&n.kind,NodeKind::Image{asset:a}if *a==asset));
                if !unchanged || s.editor.is_locked(id) || s.preview {
                    s.set_status(
                        "Background removal cancelled because the image changed",
                        false,
                    );
                    cx.notify();
                    return;
                }
                match result {
                    Ok((bytes, image, w, h)) => {
                        let previous = s.image_picker.replace_target;
                        s.image_picker.replace_target = Some(id);
                        let result =
                            s.apply_picker_image(bytes, image, w, h, String::new(), source);
                        s.image_picker.replace_target = previous;
                        match result {
                            Ok(()) => s.set_status(
                                "Background removed · free preview resolution · Undo restores the original",
                                false,
                            ),
                            Err(e) => s.set_status(format!("Could not use cutout: {e:#}"), true),
                        }
                    }
                    Err(e) => s.set_status(format!("Could not remove background: {e:#}"), true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
