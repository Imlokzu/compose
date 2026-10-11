use crate::{
    panels::label,
    workspace::{Field, Popup, Workspace},
};
use document::NodeKind;
use gpui::{prelude::*, *};
pub struct LinkEdit {
    pub id: u64,
    pub range: std::ops::Range<usize>,
    pub content: String,
}
impl Workspace {
    pub fn open_link_editor(&mut self) {
        if self.preview || self.selection_locked() {
            return;
        }
        self.close_popup();
        let Some(t) = self.text() else {
            return;
        };
        if t.content.is_empty() {
            self.set_status("Select some text before adding a link", false);
            return;
        }
        let mut range = if self.editing {
            t.range(self.range())
        } else {
            0..t.content.len()
        };
        if range.is_empty() {
            if let Some((linked, _)) =
                t.link_range(self.caret.min(t.content.len().saturating_sub(1)))
            {
                range = linked;
            } else {
                self.set_status(
                    "Highlight the words you want to link, then press Ctrl/⌘K",
                    false,
                );
                return;
            }
        }
        let url = t.style_at(range.start).link.clone().unwrap_or_default();
        self.link_edit = Some(LinkEdit {
            id: self.editor.selected().unwrap(),
            range: range.clone(),
            content: t.content.clone(),
        });
        self.link_url = url.clone();
        self.popup = Some(Popup::Link);
        self.text_popup_anchor = self.text_toolbar_bounds();
        self.begin_field(Field::LinkUrl, url);
    }
    pub fn apply_text_link(&mut self, remove: bool) {
        self.commit_field();
        let Some(edit) = &self.link_edit else {
            return;
        };
        if self.editor.is_locked(edit.id) {
            return;
        }
        let valid = self
            .editor
            .document
            .node(edit.id)
            .is_some_and(|n| matches!(&n.kind,NodeKind::Text(t)if t.content==edit.content));
        if !valid {
            self.set_status(
                "Text changed while the link was open. Select it again.",
                true,
            );
            return;
        }
        let url = if remove {
            None
        } else {
            match text::links::normalize(&self.link_url) {
                Ok(url) => Some(url),
                Err(e) => {
                    self.set_status(e, true);
                    self.begin_field(Field::LinkUrl, self.link_url.clone());
                    return;
                }
            }
        };
        let (id, range) = (edit.id, edit.range.clone());
        self.editor.checkpoint();
        if let NodeKind::Text(t) = &mut self.editor.document.node_mut(id).unwrap().kind {
            t.apply_link(range.clone(), url);
        }
        self.editor.select_only(Some(id));
        self.editing = true;
        self.anchor = range.start;
        self.caret = range.end;
        self.close_popup();
        self.measure();
        self.set_status(
            if remove {
                "Link removed"
            } else {
                "Link added · Ctrl/⌘-click to open · click in Preview"
            },
            false,
        );
    }
    pub fn link_popup_view(&self, cx: &Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(label(
                "Link the selected words to a web page or email address.",
            ))
            .child(self.field_view(Field::LinkUrl, "", self.link_url.clone(), cx))
            .child(self.capture_control(
                "link-apply",
                self.button(
                    "link-apply",
                    "Apply link",
                    Some("link"),
                    true,
                    false,
                    cx,
                    |s, _, _| s.apply_text_link(false),
                ),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(self.capture_control(
                        "link-remove",
                        self.button(
                            "link-remove",
                            "Remove",
                            Some("close"),
                            false,
                            false,
                            cx,
                            |s, _, _| s.apply_text_link(true),
                        ),
                        cx,
                    ))
                    .child(self.button(
                        "link-open",
                        "Open",
                        Some("open"),
                        false,
                        text::links::normalize(&self.link_url).is_err(),
                        cx,
                        |s, _, cx| {
                            s.commit_field();
                            if let Ok(url) = text::links::normalize(&s.link_url) {
                                cx.open_url(&url);
                            }
                        },
                    )),
            )
            .child(
                label("Ctrl/⌘-click opens a link while editing. Preview uses a normal click.")
                    .line_height(px(16.)),
            )
    }
    pub fn link_at_screen(&self, position: Point<Pixels>) -> Option<String> {
        let p = self.to_doc(position);
        let id = self.editor.hit_test_context(p, true)?;
        let n = self.editor.document.node(id)?;
        let NodeKind::Text(t) = &n.kind else {
            return None;
        };
        let r = self.absolute_frame(id)?;
        let layout = &self.layouts.get(&id)?.2;
        let index = layout.hit_test(p.x - r.x, p.y - r.y);
        let (range, url) = t.link_range(index)?;
        if !layout.selection(range).iter().any(|rect| {
            rect.contains(document::Point {
                x: p.x - r.x,
                y: p.y - r.y,
            })
        }) {
            return None;
        }
        text::links::valid(&url).then_some(url)
    }
}
