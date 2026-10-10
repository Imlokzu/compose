use crate::workspace::{Gesture, Tool, Workspace};
use document::{Node, NodeKind, Point};
use editor::ObjectClipboard;
use gpui::ClipboardItem;
use std::sync::atomic::{AtomicU64, Ordering};
use text::RichText;

pub struct LocalClipboard {
    token: String,
    object: ObjectClipboard,
    pastes: u32,
}
static NEXT_CLIP: AtomicU64 = AtomicU64::new(1);
const RICH_PREFIX: &str = "compose-rich-text-v1:";

impl Workspace {
    /// Produce a real OS clipboard item; keeping this separate also lets native QA
    /// exercise copy/cut/paste without overwriting the user's system clipboard.
    pub fn copy_item(&mut self, cut: bool) -> Option<ClipboardItem> {
        if self.preview || self.pending.is_some() || (self.popup.is_some() && self.field.is_none())
        {
            return None;
        }
        if self.field.is_some() || self.editing {
            if cut && self.selection_locked() && self.field.is_none() {
                return None;
            }
            let r = self.active_range();
            if r.is_empty() {
                return None;
            }
            let item = if self.field.is_some() {
                ClipboardItem::new_string(self.active_content()[r.clone()].to_string())
            } else {
                let rich = self.text()?.slice(r.clone());
                let metadata = format!("{RICH_PREFIX}{}", serde_json::to_string(&rich).ok()?);
                ClipboardItem::new_string_with_metadata(rich.content, metadata)
            };
            if cut {
                self.typing = None;
                self.replace_input(r, "");
                self.typing = None;
            }
            return Some(item);
        }
        if cut && self.selection_locked() {
            return None;
        }
        let object = self.editor.copy_selected()?;
        let count = object.nodes.len();
        let mut text = Vec::new();
        fn gather(node: &Node, out: &mut Vec<String>) {
            if let NodeKind::Text(t) = &node.kind {
                out.push(t.content.clone());
            }
            for child in &node.children {
                gather(child, out);
            }
        }
        for node in &object.nodes {
            gather(node, &mut text);
        }
        let fallback = if text.is_empty() {
            object
                .nodes
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            text.join("\n")
        };
        let token = format!(
            "compose-object-v2:{}:{}",
            std::process::id(),
            NEXT_CLIP.fetch_add(1, Ordering::Relaxed)
        );
        self.object_clipboard = Some(LocalClipboard {
            token: token.clone(),
            object,
            pastes: 0,
        });
        if cut {
            self.gesture = Gesture::None;
            self.guides.clear();
            self.editor.delete_selected();
        }
        self.typing = None;
        self.set_status(
            format!(
                "{count} {} {}",
                if count == 1 { "layer" } else { "layers" },
                if cut { "cut" } else { "copied" }
            ),
            false,
        );
        Some(ClipboardItem::new_string_with_metadata(fallback, token))
    }

    pub fn paste_item(&mut self, item: &ClipboardItem) {
        if self.preview || self.pending.is_some() || (self.popup.is_some() && self.field.is_none())
        {
            return;
        }
        if self.field.is_some() {
            if let Some(value) = item.text() {
                self.replace_input(self.active_range(), &value);
            }
            return;
        }
        self.gesture = Gesture::None;
        self.guides.clear();
        let rich = item
            .metadata()
            .and_then(|m| m.strip_prefix(RICH_PREFIX))
            .filter(|m| m.len() <= 16 * 1024 * 1024)
            .and_then(|m| serde_json::from_str::<RichText>(m).ok())
            .filter(|t| t.validate() && item.text().as_deref() == Some(&t.content));
        if self.editing {
            if self.selection_locked() {
                return;
            }
            self.typing = None;
            if let Some(rich) = rich {
                self.editor.checkpoint();
                let r = self.text().unwrap().range(self.range());
                let end = r.start + rich.content.len();
                self.text_mut().unwrap().replace_rich(r, &rich);
                self.anchor = end;
                self.caret = end;
                self.marked = None;
                self.insertion_style = None;
                self.caret_visible = true;
                self.measure();
                if let Some(l) = self.layout() {
                    let h = l.height.max(24.);
                    self.editor.selected_node_mut().unwrap().frame.height = h;
                }
            } else if let Some(value) = item.text() {
                self.replace_input(self.range(), &value.replace("\r\n", "\n"));
            }
            self.typing = None;
            return;
        }
        if let Some(clip) = &mut self.object_clipboard
            && item.metadata() == Some(&clip.token)
        {
            let step = (clip.pastes + 1).min(1000);
            let offset = 16. * step as f32;
            match self.editor.paste_object(
                &clip.object,
                Point {
                    x: offset,
                    y: offset,
                },
            ) {
                Ok(_) => {
                    clip.pastes = step;
                    self.tool = Tool::Select;
                    self.typing = None;
                    self.measure();
                    let count = self.editor.selection_count();
                    self.set_status(
                        format!(
                            "{count} {} pasted · drag to place",
                            if count == 1 { "layer" } else { "layers" }
                        ),
                        false,
                    );
                }
                Err(e) => self.set_status(format!("Could not paste: {e}"), true),
            }
            return;
        }
        if let Some(value) = item.text().filter(|s| !s.is_empty()) {
            let rich = rich.unwrap_or_else(|| RichText::new(value.replace("\r\n", "\n")));
            self.insert_at(Tool::Text, Point { x: 64., y: 120. });
            *self.text_mut().unwrap() = rich;
            self.caret = self.text().unwrap().content.len();
            self.anchor = self.caret;
            self.measure();
            self.set_status("Text pasted", false);
        }
    }
}
