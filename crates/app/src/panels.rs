use crate::workspace::{ColorTarget, Field, Pending, Popup, Tool, Workspace};
use document::{Color, NodeKind};
use gpui::{prelude::*, *};
use std::borrow::Cow;
pub const INK: u32 = 0x252a25;
pub const MUTED: u32 = 0x7a8078;
pub const LINE: u32 = 0xe3e6df;
pub fn color(c: Color) -> Hsla {
    rgba((c.r as u32) << 24 | (c.g as u32) << 16 | (c.b as u32) << 8 | c.a as u32).into()
}
pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "select" => include_bytes!("../../../assets/icons/select.svg"),
            "text" => include_bytes!("../../../assets/icons/text.svg"),
            "shape" => include_bytes!("../../../assets/icons/shape.svg"),
            "frame" => include_bytes!("../../../assets/icons/frame.svg"),
            "image" => include_bytes!("../../../assets/icons/image.svg"),
            "undo" => include_bytes!("../../../assets/icons/undo.svg"),
            "redo" => include_bytes!("../../../assets/icons/redo.svg"),
            "plus" => include_bytes!("../../../assets/icons/plus.svg"),
            "save" => include_bytes!("../../../assets/icons/save.svg"),
            "open" => include_bytes!("../../../assets/icons/open.svg"),
            "up" => include_bytes!("../../../assets/icons/up.svg"),
            "down" => include_bytes!("../../../assets/icons/down.svg"),
            "duplicate" => include_bytes!("../../../assets/icons/duplicate.svg"),
            "trash" => include_bytes!("../../../assets/icons/trash.svg"),
            "left" => include_bytes!("../../../assets/icons/left.svg"),
            "center" => include_bytes!("../../../assets/icons/center.svg"),
            "right" => include_bytes!("../../../assets/icons/right.svg"),
            "page" => include_bytes!("../../../assets/icons/page.svg"),
            "logo" => include_bytes!("../../../assets/icons/logo.svg"),
            "components" => include_bytes!("../../../assets/icons/components.svg"),
            "layers" => include_bytes!("../../../assets/icons/layers.svg"),
            "eye" => include_bytes!("../../../assets/icons/eye.svg"),
            "eye-off" => include_bytes!("../../../assets/icons/eye-off.svg"),
            "lock" => include_bytes!("../../../assets/icons/lock.svg"),
            "unlock" => include_bytes!("../../../assets/icons/unlock.svg"),
            "chevron-down" => include_bytes!("../../../assets/icons/chevron-down.svg"),
            "close" => include_bytes!("../../../assets/icons/close.svg"),
            "check" => include_bytes!("../../../assets/icons/check.svg"),
            "settings" => include_bytes!("../../../assets/icons/settings.svg"),
            "preview" => include_bytes!("../../../assets/icons/preview.svg"),
            "export" => include_bytes!("../../../assets/icons/export.svg"),
            "print" => include_bytes!("../../../assets/icons/print.svg"),
            "group" => include_bytes!("../../../assets/icons/group.svg"),
            "ungroup" => include_bytes!("../../../assets/icons/ungroup.svg"),
            "align-left" => include_bytes!("../../../assets/icons/align-left.svg"),
            "align-center-x" => include_bytes!("../../../assets/icons/align-center-x.svg"),
            "align-right" => include_bytes!("../../../assets/icons/align-right.svg"),
            "align-top" => include_bytes!("../../../assets/icons/align-top.svg"),
            "align-center-y" => include_bytes!("../../../assets/icons/align-center-y.svg"),
            "align-bottom" => include_bytes!("../../../assets/icons/align-bottom.svg"),
            "distribute-x" => include_bytes!("../../../assets/icons/distribute-x.svg"),
            "distribute-y" => include_bytes!("../../../assets/icons/distribute-y.svg"),
            "rotate" => include_bytes!("../../../assets/icons/rotate.svg"),
            "stroke" => include_bytes!("../../../assets/icons/stroke.svg"),
            "shadow" => include_bytes!("../../../assets/icons/shadow.svg"),
            "typeface" => include_bytes!("../../../assets/icons/typeface.svg"),
            "clear-style" => include_bytes!("../../../assets/icons/clear-style.svg"),
            "search" => include_bytes!("../../../assets/icons/search.svg"),
            "palette" => include_bytes!("../../../assets/icons/palette.svg"),

            "minus" => include_bytes!("../../../assets/icons/minus.svg"),
            "ratio" => include_bytes!("../../../assets/icons/ratio.svg"),
            "stretch" => include_bytes!("../../../assets/icons/stretch.svg"),
            "corners" => include_bytes!("../../../assets/icons/corners.svg"),

            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
pub(crate) fn icon(name: &'static str) -> Svg {
    svg().path(name).size(px(17.)).text_color(rgb(INK))
}
pub(crate) fn label(s: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(11.))
        .text_color(rgb(MUTED))
        .child(s.into())
}
fn separator() -> Div {
    div().w(px(1.)).h(px(20.)).mx(px(6.)).bg(rgb(LINE))
}
impl Workspace {
    // Keep identity, content, state, and the handler together at each call site.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn button(
        &self,
        id: &'static str,
        label_text: impl Into<SharedString>,
        icon_name: Option<&'static str>,
        active: bool,
        disabled: bool,
        cx: &Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        let text: SharedString = label_text.into();
        let hint: SharedString = match id {
            "preview" => "Preview page",
            "export-menu" => "Export & print · ⇧⌘E",
            "print-document" => "Print document · ⌘P",
            "selection-left" => "Align left edges",
            "selection-center-x" => "Align horizontal centers",
            "selection-right" => "Align right edges",
            "selection-top" => "Align top edges",
            "selection-center-y" => "Align vertical centers",
            "selection-bottom" => "Align bottom edges",
            "selection-group" => "Group selected siblings · ⌘G",
            "selection-ungroup" => "Ungroup · ⇧⌘G",
            "components-menu" => "Components & assets",
            "open" => "Open document · ⌘O",
            "page-setup" => "Page setup",
            "close-popover" => "Close",
            "tool-select" => "Select · V",
            "tool-text" => "Text & selection · T",
            "tool-shape" => "Rectangle · R",
            "tool-frame" => "Frame · F",
            "tool-image" => "Insert image",
            "undo" => "Undo · ⌘Z",
            "redo" => "Redo · ⇧⌘Z",
            "send-back" => "Send backward",
            "bring-forward" => "Bring forward",
            "duplicate" => "Duplicate · ⌘D",
            "delete" => "Delete",
            "image-smaller" => "Scale down by 10%",
            "image-bigger" => "Scale up by 10%",
            "align-left" => "Align text left",
            "align-center" => "Center text",
            "align-right" => "Align text right",
            "float-font" => "Choose font",
            "float-weight" => "Font weight",
            "float-sizes" => "Font size presets",
            "float-styles" => "Text styles",
            "float-color" => "Text color",
            "float-reset" => "Clear selected text formatting",
            _ => text.as_ref(),
        }
        .to_string()
        .into();
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .h(px(30.))
            .min_w(px(30.))
            .px(px(7.))
            .rounded(px(5.))
            .text_size(px(12.))
            .bg(if active { rgb(0xe6efff) } else { rgb(0xffffff) })
            .text_color(if disabled {
                rgb(0xb6bbb3)
            } else if active {
                rgb(0x2878ed)
            } else {
                rgb(INK)
            })
            .when(!disabled, |d| {
                d.cursor_pointer()
                    .hover(|d| d.bg(rgb(if id == "save" { 0x454b43 } else { 0xf0f2ed })))
            })
            .when_some(icon_name, |d, name| {
                d.child(
                    icon(name)
                        .text_color(rgb(if active { 0x2878ed } else { INK }))
                        .when(disabled, |d| d.opacity(0.35)),
                )
            })
            .when(!text.is_empty(), |d| d.child(text))
            .when(!hint.is_empty(), |d| {
                d.tooltip(move |_, cx| cx.new(|_| Hint(hint.clone())).into())
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                if !disabled {
                    action(this, window, cx);
                    this.focus.focus(window);
                    cx.notify();
                }
            }))
    }
    pub(crate) fn section(&self, title: &str, content: impl IntoElement) -> Div {
        div()
            .px(px(16.))
            .py(px(12.))
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .mb(px(10.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title.to_string()),
            )
            .child(content)
    }
    pub(crate) fn field_view(
        &self,
        field: Field,
        title: &'static str,
        value: String,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        self.field_view_at(field, title, value, "inspector", cx)
    }
    pub(crate) fn field_view_at(
        &self,
        field: Field,
        title: &'static str,
        value: String,
        source: &'static str,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let active = self
            .field
            .as_ref()
            .is_some_and(|f| f.field == field && f.source == source);
        let entity = cx.entity();
        let display = value.clone();
        div()
            .id(SharedString::from(format!(
                "field-{source}-{}",
                field as u8
            )))
            .flex()
            .items_center()
            .gap(px(6.))
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .h(px(30.))
            .px(px(8.))
            .rounded(px(6.))
            .border_1()
            .border_color(if active { rgb(0x2878ed) } else { rgb(LINE) })
            .bg(rgb(0xf8f9f6))
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, _, w, cx| {
                    s.begin_field(field, value.clone());
                    s.field.as_mut().unwrap().source = source;
                    s.focus.focus(w);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(label(title).w(px(if title.is_empty() { 0. } else { 15. })))
            .child(
                canvas(
                    move |_, _, _| (),
                    move |bounds, (), w, cx| {
                        entity.update(cx, |s, cx| {
                            s.paint_field(field, source, &display, bounds, w, cx)
                        });
                    },
                )
                .flex_1()
                .h(px(20.)),
            )
    }
    fn toolbar(&self, cx: &Context<Self>) -> Div {
        if self.preview {
            return div()
                .h(px(49.))
                .px(px(18.))
                .flex()
                .items_center()
                .gap(px(10.))
                .bg(white())
                .border_b_1()
                .border_color(rgb(LINE))
                .child(icon("logo"))
                .child(self.editor.document.name.clone())
                .child(div().flex_1())
                .child(label("Preview"))
                .child(self.button(
                    "export-menu",
                    "Export",
                    Some("export"),
                    self.popup == Some(Popup::Export),
                    self.busy,
                    cx,
                    |s, _, _| s.toggle_popup(Popup::Export),
                ))
                .child(self.button(
                    "leave-preview",
                    "Back to editor",
                    Some("close"),
                    false,
                    false,
                    cx,
                    |s, _, _| s.toggle_preview(),
                ));
        }
        div()
            .h(px(49.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px(px(16.))
            .border_b_1()
            .border_color(rgb(LINE))
            .bg(rgb(0xffffff))
            .child(
                div()
                    .w(px(154.))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .child(icon("logo").size(px(22.)))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Compose"),
                    ),
            )
            .child(self.button(
                "tool-select",
                "",
                Some("select"),
                self.tool == Tool::Select,
                false,
                cx,
                |s, _, _| {
                    s.commit_field();
                    s.end_edit();
                    s.tool = Tool::Select
                },
            ))
            .child(self.button(
                "tool-text",
                "",
                Some("text"),
                self.tool == Tool::Text,
                false,
                cx,
                |s, _, _| {
                    s.commit_field();
                    s.end_edit();
                    s.tool = Tool::Text;
                    s.set_status("Click on the page to insert text", false)
                },
            ))
            .child(self.button(
                "tool-shape",
                "",
                Some("shape"),
                self.tool == Tool::Shape,
                false,
                cx,
                |s, _, _| {
                    s.end_edit();
                    s.tool = Tool::Shape;
                },
            ))
            .child(self.button(
                "tool-frame",
                "",
                Some("frame"),
                self.tool == Tool::Frame,
                false,
                cx,
                |s, _, _| {
                    s.end_edit();
                    s.tool = Tool::Frame;
                },
            ))
            .child(self.button(
                "tool-image",
                "",
                Some("image"),
                false,
                self.busy,
                cx,
                |s, _, cx| s.open_image_picker(cx),
            ))
            .child(self.button(
                "components-menu",
                "",
                Some("components"),
                self.assets_tab,
                false,
                cx,
                |s, _, _| {
                    s.assets_tab = !s.assets_tab;
                },
            ))
            .child(separator())
            .child(self.button(
                "undo",
                "",
                Some("undo"),
                false,
                !self.editor.can_undo(),
                cx,
                |s, _, _| s.undo(false),
            ))
            .child(self.button(
                "redo",
                "",
                Some("redo"),
                false,
                !self.editor.can_redo(),
                cx,
                |s, _, _| s.undo(true),
            ))
            .child(div().flex_1())
            .child(div().w(px(154.)).child(self.field_view(
                Field::DocumentName,
                "",
                self.editor.document.name.clone(),
                cx,
            )))
            .child(div().flex_1())
            .child(
                self.button("zoom-minus", "−", None, false, false, cx, |s, _, _| {
                    s.zoom_by(1. / 1.15)
                }),
            )
            .child(self.button(
                "zoom-fit",
                format!("{}%", (self.zoom * 100.).round()),
                None,
                false,
                false,
                cx,
                |s, _, _| s.fit(),
            ))
            .child(
                self.button("zoom-plus", "+", None, false, false, cx, |s, _, _| {
                    s.zoom_by(1.15)
                }),
            )
            .child(separator())
            .child(self.button(
                "preview",
                "",
                Some("preview"),
                false,
                false,
                cx,
                |s, _, _| s.toggle_preview(),
            ))
            .child(self.button(
                "open",
                "",
                Some("open"),
                false,
                self.busy,
                cx,
                |s, _, cx| s.choose_file(false, cx),
            ))
            .child(
                self.button("save", "Save", None, false, self.busy, cx, |s, _, cx| {
                    s.save(false, cx)
                })
                .ml(px(7.))
                .bg(rgb(0x252a25))
                .text_color(rgb(0xffffff))
                .child(div().w(px(1.))),
            )
            .child(self.button(
                "export-menu",
                "Export",
                Some("export"),
                self.popup == Some(Popup::Export),
                self.busy,
                cx,
                |s, _, _| s.toggle_popup(Popup::Export),
            ))
    }
    fn layers(&self, cx: &Context<Self>) -> Div {
        let page = &self.editor.document.pages[self.editor.page];
        let tabs_entity = cx.entity();
        let mut rows = div().flex().flex_col().gap(px(2.));
        fn add(
            app: &Workspace,
            nodes: &[document::Node],
            depth: usize,
            rows: &mut Div,
            cx: &Context<Workspace>,
        ) {
            for n in nodes.iter().rev() {
                let id = n.id;
                let text = matches!(n.kind, NodeKind::Text(_));
                let ico = if n.is_group {
                    "group"
                } else {
                    match &n.kind {
                        NodeKind::Text(_) => "text",
                        NodeKind::Image { .. } => "image",
                        NodeKind::Shape => "shape",
                        NodeKind::Frame => "frame",
                    }
                };
                let selected = app.editor.is_selected(id);
                let row = div()
                    .id(("layer", id))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(px(32.))
                    .pl(px(14. + depth as f32 * 14.))
                    .pr(px(8.))
                    .rounded(px(4.))
                    .bg(if selected {
                        rgb(0xe6efff)
                    } else {
                        rgb(0xffffff)
                    })
                    .text_color(if selected { rgb(0x2878ed) } else { rgb(INK) })
                    .cursor_pointer()
                    .hover(|d| d.bg(rgb(0xf0f2ed)))
                    .when(!n.children.is_empty(), |d| {
                        d.child(
                            div()
                                .id(("toggle-node", id))
                                .size(px(14.))
                                .cursor_pointer()
                                .child(
                                    icon(if app.collapsed_nodes.contains(&id) {
                                        "up"
                                    } else {
                                        "down"
                                    })
                                    .size(px(12.)),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |s, _, _, cx| {
                                        if !s.collapsed_nodes.remove(&id) {
                                            s.collapsed_nodes.insert(id);
                                        }
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                ),
                        )
                    })
                    .child(icon(ico).size(px(15.)))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(n.name.clone())
                            .when(!n.visible, |d| d.opacity(0.4)),
                    )
                    .child(
                        div()
                            .id(("visibility", id))
                            .size(px(18.))
                            .flex_shrink_0()
                            .cursor_pointer()
                            .child(icon(if n.visible { "eye" } else { "eye-off" }).size(px(13.)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |s, _, _, cx| {
                                    s.commit_field();
                                    s.end_edit();
                                    s.editor.toggle_visibility(id);
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .id(("lock", id))
                            .size(px(18.))
                            .flex_shrink_0()
                            .cursor_pointer()
                            .child(
                                icon(if n.locked { "lock" } else { "unlock" })
                                    .size(px(13.))
                                    .opacity(if n.locked { 1. } else { 0.35 }),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |s, _, _, cx| {
                                    s.commit_field();
                                    s.end_edit();
                                    s.editor.toggle_lock(id);
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            ),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |app, e: &MouseDownEvent, window, cx| {
                            app.close_popup();
                            app.commit_field();
                            app.end_edit();
                            if e.modifiers.shift && e.click_count == 1 {
                                app.editor.toggle_selected(id);
                            } else {
                                app.editor.select_only(Some(id));
                            }
                            if e.click_count == 2 && !app.selection_locked() {
                                let value = app.editor.selected_node().unwrap().name.clone();
                                app.begin_field(Field::Name, value);
                            } else if text {
                                app.set_status("Double-click text on the page to edit", false);
                            }
                            app.focus.focus(window);
                            cx.notify();
                        }),
                    );
                let entity = cx.entity();
                *rows = std::mem::replace(rows, div()).child(
                    div()
                        .on_children_prepainted(move |bounds, _, cx| {
                            if let Some(bound) = bounds.first() {
                                entity.update(cx, |s, _| {
                                    s.layer_bounds.insert(id, *bound);
                                });
                            }
                        })
                        .child(row),
                );
                if !app.collapsed_nodes.contains(&id) {
                    add(app, &n.children, depth + 1, rows, cx);
                }
            }
        }
        if !self.collapsed {
            add(self, &page.nodes, 0, &mut rows, cx)
        }
        let mut pages = div().flex().flex_col().gap(px(3.));
        if self.show_pages {
            for (i, p) in self.editor.document.pages.iter().enumerate() {
                pages = pages.child(
                    div()
                        .id(("page", i))
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(4.))
                        .bg(if i == self.editor.page {
                            rgb(0xe6efff)
                        } else {
                            rgb(0xffffff)
                        })
                        .child(p.name.clone())
                        .cursor_pointer()
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.commit_field();
                            s.end_edit();
                            s.editor.page = i;
                            s.editor.select_only(None);
                            s.fitted = false;
                            s.show_pages = false;
                            cx.notify();
                        })),
                );
            }
        }
        div()
            .w(px(220.))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .border_r_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .px(px(14.))
                    .py(px(12.))
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(self.button(
                                "pages",
                                page.name.clone(),
                                Some("page"),
                                false,
                                false,
                                cx,
                                |s, _, _| s.show_pages = !s.show_pages,
                            ))
                            .child(self.button(
                                "add-page",
                                "",
                                Some("plus"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.commit_field();
                                    s.end_edit();
                                    s.editor.checkpoint();
                                    let id = s.editor.document.next_id;
                                    s.editor.document.next_id += 1;
                                    let index = s.editor.document.pages.len();
                                    s.editor.document.pages.push(document::Page {
                                        id,
                                        name: format!("Page {}", index + 1),
                                        width: 595.28,
                                        height: 841.89,
                                        nodes: vec![],
                                    });
                                    s.editor.page = index;
                                    s.editor.select_only(None);
                                    s.fitted = false;
                                },
                            )),
                    )
                    .when(self.show_pages, |d| d.child(pages)),
            )
            .child(
                div()
                    .px(px(12.))
                    .py(px(10.))
                    .flex()
                    .gap(px(5.))
                    .child(
                        div()
                            .flex_1()
                            .on_children_prepainted(move |bounds, _, cx| {
                                if let Some(bound) = bounds.first() {
                                    tabs_entity.update(cx, |s, _| {
                                        s.selection_control_bounds.insert("layers-tab", *bound);
                                    });
                                }
                            })
                            .child(
                                self.button(
                                    "layers-tab",
                                    "Layers",
                                    Some("layers"),
                                    !self.assets_tab,
                                    false,
                                    cx,
                                    |s, _, _| s.assets_tab = false,
                                )
                                .w_full(),
                            ),
                    )
                    .child(
                        self.button(
                            "assets-tab",
                            "Assets",
                            Some("components"),
                            self.assets_tab,
                            false,
                            cx,
                            |s, _, _| s.assets_tab = true,
                        )
                        .flex_1(),
                    ),
            )
            .when(self.assets_tab, |d| {
                d.child(
                    div()
                        .id("asset-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(self.assets_panel(cx)),
                )
            })
            .child(
                div()
                    .id("layer-scroll")
                    .when(self.assets_tab, |d| d.hidden())
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(8.))
                    .child(
                        div()
                            .id("page-collapse")
                            .h(px(30.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .px(px(8.))
                            .cursor_pointer()
                            .child(
                                icon(if self.collapsed { "right" } else { "down" }).size(px(12.)),
                            )
                            .child(icon("page").size(px(14.)))
                            .child(page.name.clone())
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.collapsed = !s.collapsed;
                                cx.notify();
                            })),
                    )
                    .child(rows)
                    .when(page.nodes.is_empty(), |d| {
                        d.child(
                            div()
                                .px(px(16.))
                                .py(px(20.))
                                .text_size(px(12.))
                                .text_color(rgb(MUTED))
                                .child("Your page starts here.\nAdd text, images, or shapes."),
                        )
                    }),
            )
            .child(
                div()
                    .px(px(14.))
                    .py(px(14.))
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .mb(px(9.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Insert"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(4.))
                            .child(self.button(
                                "insert-text",
                                "Text",
                                Some("text"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.insert_at(Tool::Text, document::Point { x: 48., y: 64. })
                                },
                            ))
                            .child(self.button(
                                "insert-image",
                                "Image",
                                Some("image"),
                                false,
                                self.busy,
                                cx,
                                |s, _, cx| s.open_image_picker(cx),
                            )),
                    )
                    .child(
                        div()
                            .mt(px(5.))
                            .flex()
                            .gap(px(4.))
                            .child(self.button(
                                "insert-shape",
                                "Shape",
                                Some("shape"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.insert_at(Tool::Shape, document::Point { x: 64., y: 120. })
                                },
                            ))
                            .child(self.button(
                                "insert-frame",
                                "Frame",
                                Some("frame"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.insert_at(Tool::Frame, document::Point { x: 64., y: 120. })
                                },
                            )),
                    ),
            )
    }
    pub(crate) fn text_controls(&self, floating: bool, cx: &Context<Self>) -> Div {
        let style = self.active_style();
        let face = renderer::FONT_CHOICES
            .iter()
            .find(|f| f.family == style.family);
        let can_bold = face.is_none_or(|f| f.weights.iter().any(|w| w.0 >= 700.));
        let can_italic = face.is_none_or(|f| f.italic);
        div()
            .flex()
            .items_center()
            .gap(px(3.))
            .child(
                self.button(
                    if floating { "float-bold" } else { "bold" },
                    "B",
                    None,
                    self.format_values().iter().all(|s| s.weight >= 600.),
                    !can_bold,
                    cx,
                    |s, _, _| {
                        let b = !s.format_values().iter().all(|v| v.weight >= 600.);
                        s.format(|v| v.weight = if b { 700. } else { 400. })
                    },
                )
                .font_weight(FontWeight::BOLD),
            )
            .child(
                self.button(
                    if floating { "float-italic" } else { "italic" },
                    "I",
                    None,
                    self.format_values().iter().all(|s| s.italic),
                    !can_italic,
                    cx,
                    |s, _, _| {
                        let i = !s.format_values().iter().all(|v| v.italic);
                        s.format(|v| v.italic = i)
                    },
                )
                .italic(),
            )
            .child(self.button(
                if floating {
                    "float-underline"
                } else {
                    "underline"
                },
                "U",
                None,
                self.format_values().iter().all(|s| s.underline),
                false,
                cx,
                |s, _, _| {
                    let u = !s.format_values().iter().all(|v| v.underline);
                    s.format(|v| v.underline = u)
                },
            ))
    }
    fn inspector(&self, cx: &Context<Self>) -> Stateful<Div> {
        let mut panel = div()
            .id("inspector-scroll")
            .w(px(272.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .bg(rgb(0xffffff))
            .border_l_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .h(px(44.))
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Design")
                    .child(div().flex_1())
                    .child(self.button(
                        "page-setup",
                        "",
                        Some("settings"),
                        false,
                        false,
                        cx,
                        |s, _, _| s.toggle_popup(Popup::Pages),
                    )),
            );
        if self.editor.selection_count() > 1
            || self.editor.selected_node().is_some_and(|n| n.is_group)
        {
            return panel.child(self.selection_inspector(cx));
        }
        let Some(node) = self.editor.selected_node() else {
            return panel.child(self.section("Page",div().flex().flex_col().gap(px(12.))
                .child(div().flex().justify_between().child(label("Format")).child(format!("{:.0} × {:.0} pt",self.editor.document.pages[self.editor.page].width,self.editor.document.pages[self.editor.page].height)))
                .child(div().flex().justify_between().child(label("Size")).child(if self.editor.document.pages[self.editor.page].width>self.editor.document.pages[self.editor.page].height{"Landscape"}else{"Portrait"}))))
                .child(div().p(px(16.)).text_color(rgb(MUTED)).line_height(px(19.)).child("Select an element to design it. Double-click text to select a word, or use the Text tool to highlight a passage."))
                .child(self.section("Quick keys",div().flex().flex_col().gap(px(9.)).child("V   Select & move").child("T   Write & select text").child("R   Rectangle").child("Space + drag   Pan").child("⌘ + scroll   Zoom")))
                .child(div().p(px(16.)).child(self.button("new-doc","New document",Some("plus"),false,false,cx,|s,_,cx|s.request(Pending::New,cx))));
        };
        if self.selection_locked() {
            return panel.child(
                self.section(
                    "Locked layer",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(label("Unlock this layer or its parent to edit."))
                        .child(self.button(
                            "unlock-layer",
                            "Unlock",
                            Some("lock"),
                            false,
                            false,
                            cx,
                            |s, _, _| {
                                let mut id = s.editor.selected().unwrap();
                                while !s.editor.document.node(id).unwrap().locked {
                                    let Some(parent) = s.editor.parent_of(id) else {
                                        return;
                                    };
                                    id = parent;
                                }
                                s.editor.toggle_lock(id);
                            },
                        )),
                ),
            );
        }
        let is_image = matches!(node.kind, NodeKind::Image { .. });
        let is_text = matches!(node.kind, NodeKind::Text(_));
        panel = panel.child(self.section(
            match node.kind {
                NodeKind::Text(_) => "Text",
                NodeKind::Image { .. } => "Image",
                NodeKind::Shape => "Rectangle",
                NodeKind::Frame => "Frame",
            },
            self.field_view(Field::Name, "", node.name.clone(), cx),
        ));
        if let NodeKind::Text(t) = &node.kind {
            panel = panel.child(
                self.section(
                    "Typography",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(label(self.formatting_scope()).text_color(rgb(0x2878ed)))
                        .child(self.typography_choices(cx))
                        .child(self.text_controls(false, cx))
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .child(div().flex_1().child(label("Font size").mb(px(5.))).child(
                                    self.field_view(
                                        Field::Size,
                                        "pt",
                                        self.style_value(Field::Size),
                                        cx,
                                    ),
                                ))
                                .child(
                                    div().flex_1().child(label("Line height").mb(px(5.))).child(
                                        self.field_view(
                                            Field::LineHeight,
                                            "×",
                                            self.style_value(Field::LineHeight),
                                            cx,
                                        ),
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(label("Letter spacing").w(px(87.)))
                                .child(self.field_view(
                                    Field::Spacing,
                                    "pt",
                                    self.style_value(Field::Spacing),
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(4.))
                                .child(self.button(
                                    "align-left",
                                    "",
                                    Some("left"),
                                    t.align == text::TextAlign::Left,
                                    false,
                                    cx,
                                    |s, _, _| {
                                        s.commit_field();
                                        s.editor.checkpoint();
                                        if let Some(t) = s.text_mut() {
                                            t.align = text::TextAlign::Left
                                        }
                                    },
                                ))
                                .child(self.button(
                                    "align-center",
                                    "",
                                    Some("center"),
                                    t.align == text::TextAlign::Center,
                                    false,
                                    cx,
                                    |s, _, _| {
                                        s.commit_field();
                                        s.editor.checkpoint();
                                        if let Some(t) = s.text_mut() {
                                            t.align = text::TextAlign::Center
                                        }
                                    },
                                ))
                                .child(self.button(
                                    "align-right",
                                    "",
                                    Some("right"),
                                    t.align == text::TextAlign::Right,
                                    false,
                                    cx,
                                    |s, _, _| {
                                        s.commit_field();
                                        s.editor.checkpoint();
                                        if let Some(t) = s.text_mut() {
                                            t.align = text::TextAlign::Right
                                        }
                                    },
                                )),
                        ),
                ),
            );
        }
        if is_image {
            panel = panel.child(self.section(
                "Image content",
                div().flex().flex_col().gap(px(8.))
                    .child(self.button("replace-image", "Replace image…", Some("image"), false, false, cx,
                        |s, _, cx| s.open_image_replacement(cx)))
                    .child(label(if node.image_sizing == document::ImageSizing::Scale {
                        "Keeps position, width and style. Height follows the new proportions."
                    } else {
                        "Keeps position, size and style."
                    })),
            ));
            let scale = node.image_sizing == document::ImageSizing::Scale;
            panel = panel.child(
                self.section(
                    "Resize image",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(9.))
                        .child(
                            div()
                                .flex()
                                .gap(px(4.))
                                .p(px(3.))
                                .rounded(px(7.))
                                .bg(rgb(0xf1f3ee))
                                .child(
                                    self.button(
                                        "image-scale",
                                        "Scale",
                                        Some("ratio"),
                                        scale,
                                        false,
                                        cx,
                                        |s, _, _| s.set_image_sizing(document::ImageSizing::Scale),
                                    )
                                    .flex_1(),
                                )
                                .child(
                                    self.button(
                                        "image-stretch",
                                        "Stretch",
                                        Some("stretch"),
                                        !scale,
                                        false,
                                        cx,
                                        |s, _, _| {
                                            s.set_image_sizing(document::ImageSizing::Stretch)
                                        },
                                    )
                                    .flex_1(),
                                ),
                        )
                        .child(label(if scale {
                            "Proportions locked"
                        } else {
                            "Width and height are independent"
                        }))
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .child(
                                    self.button(
                                        "image-smaller",
                                        "Smaller",
                                        Some("minus"),
                                        false,
                                        false,
                                        cx,
                                        |s, _, _| s.scale_image(0.9),
                                    )
                                    .flex_1()
                                    .border_1()
                                    .border_color(rgb(LINE)),
                                )
                                .child(
                                    self.button(
                                        "image-bigger",
                                        "Bigger",
                                        Some("plus"),
                                        false,
                                        false,
                                        cx,
                                        |s, _, _| s.scale_image(1.1),
                                    )
                                    .flex_1()
                                    .border_1()
                                    .border_color(rgb(LINE)),
                                ),
                        ),
                ),
            );
        }
        if is_image {
            panel = panel.child(self.image_source_panel(cx));
        }
        let r = node.frame;
        panel = panel.child(
            self.section(
                "Position & size",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(self.field_view(Field::X, "X", format!("{:.1}", r.x), cx))
                            .child(self.field_view(Field::Y, "Y", format!("{:.1}", r.y), cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(self.field_view(Field::W, "W", format!("{:.1}", r.width), cx))
                            .child(self.field_view(Field::H, "H", format!("{:.1}", r.height), cx)),
                    ),
            ),
        );
        if !is_text {
            let max = r.width.min(r.height) / 2.;
            let radius = node.radius.min(max);
            panel = panel.child(
                self.section(
                    "Corners",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(icon("corners"))
                                .child(label("Radius"))
                                .child(self.field_view(
                                    Field::Radius,
                                    "pt",
                                    format!("{radius:.1}"),
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(4.))
                                .child(
                                    self.button(
                                        "radius-square",
                                        "Square",
                                        None,
                                        radius < 0.01,
                                        false,
                                        cx,
                                        |s, _, _| s.set_radius(0.),
                                    )
                                    .flex_1(),
                                )
                                .child(
                                    self.button(
                                        "radius-soft",
                                        "Soft",
                                        None,
                                        (radius - 12.).abs() < 0.01,
                                        false,
                                        cx,
                                        |s, _, _| s.set_radius(12.),
                                    )
                                    .flex_1(),
                                )
                                .child(
                                    self.button(
                                        "radius-round",
                                        "Round",
                                        None,
                                        (radius - max).abs() < 0.01,
                                        false,
                                        cx,
                                        |s, _, _| {
                                            if let Some(n) = s.editor.selected_node() {
                                                s.set_radius(n.frame.width.min(n.frame.height) / 2.)
                                            }
                                        },
                                    )
                                    .flex_1(),
                                ),
                        )
                        .child(label("Or drag the round handle on the canvas")),
                ),
            );
        }
        let mut appearance = div().flex().flex_col().gap(px(10.));
        if !is_image {
            let mut colors = div().flex().gap(px(9.));
            let active = if is_text {
                self.active_style().color
            } else {
                node.fill
            };
            for (index, c) in [
                Color::rgb(35, 38, 36),
                Color::rgb(114, 130, 101),
                Color::rgb(211, 222, 201),
                Color::rgb(40, 120, 237),
                Color::rgb(206, 142, 112),
                Color::rgb(255, 254, 251),
            ]
            .into_iter()
            .enumerate()
            {
                colors = colors.child(
                    div()
                        .id(("color", index))
                        .size(px(25.))
                        .rounded(px(13.))
                        .p(px(3.))
                        .border_1()
                        .border_color(if c == active {
                            rgb(0x2878ed)
                        } else {
                            rgb(0xe1e5dc)
                        })
                        .cursor_pointer()
                        .child(div().size_full().rounded(px(10.)).bg(color(c)))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.commit_field();
                            if s.text().is_some() {
                                s.format(|v| v.color = c)
                            } else {
                                s.editor.checkpoint();
                                if let Some(n) = s.editor.selected_node_mut() {
                                    n.fill = c;
                                }
                            }
                            cx.notify();
                        })),
                );
            }
            appearance = appearance
                .child(self.color_row(ColorTarget::Fill, cx))
                .child(colors);
        }
        appearance = appearance.child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(label("Opacity").w(px(80.)))
                .child(self.field_view(
                    Field::Opacity,
                    "%",
                    format!("{:.0}", node.opacity * 100.),
                    cx,
                )),
        );
        panel = panel.child(self.section(
            if is_image {
                "Appearance"
            } else {
                "Color & opacity"
            },
            appearance,
        ));
        if !is_text {
            panel = panel.child(self.effects_panel(cx));
        }
        panel = panel.child(
            self.section(
                "Arrange",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .gap(px(3.))
                            .child(
                                self.button(
                                    "object-left",
                                    "Left",
                                    None,
                                    false,
                                    false,
                                    cx,
                                    |s, _, _| s.align_object(0),
                                )
                                .flex_1(),
                            )
                            .child(
                                self.button(
                                    "object-center",
                                    "Center",
                                    None,
                                    false,
                                    false,
                                    cx,
                                    |s, _, _| s.align_object(1),
                                )
                                .flex_1(),
                            )
                            .child(
                                self.button(
                                    "object-right",
                                    "Right",
                                    None,
                                    false,
                                    false,
                                    cx,
                                    |s, _, _| s.align_object(2),
                                )
                                .flex_1(),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(self.button(
                                "send-back",
                                "",
                                Some("down"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.commit_field();
                                    s.end_edit();
                                    s.editor.reorder_selected(false)
                                },
                            ))
                            .child(self.button(
                                "bring-forward",
                                "",
                                Some("up"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.commit_field();
                                    s.end_edit();
                                    s.editor.reorder_selected(true)
                                },
                            ))
                            .child(div().flex_1())
                            .child(self.button(
                                "duplicate",
                                "",
                                Some("duplicate"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.commit_field();
                                    s.end_edit();
                                    s.editor.duplicate_selected()
                                },
                            ))
                            .child(self.button(
                                "delete",
                                "",
                                Some("trash"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.field = None;
                                    s.end_edit();
                                    s.editor.delete_selected()
                                },
                            )),
                    ),
            ),
        );
        panel.child(self.parent_controls(cx))
    }
}
impl Workspace {
    fn parent_controls(&self, cx: &Context<Self>) -> Div {
        let Some(id) = self.editor.selected() else {
            return div();
        };
        let current = self.editor.parent_of(id);
        let mut parents = Vec::new();
        fn collect(nodes: &[document::Node], out: &mut Vec<(u64, String)>) {
            for n in nodes {
                if matches!(n.kind, NodeKind::Frame) {
                    out.push((n.id, n.name.clone()));
                }
                collect(&n.children, out)
            }
        }
        collect(
            &self.editor.document.pages[self.editor.page].nodes,
            &mut parents,
        );
        if parents.is_empty() {
            return div();
        }
        let mut list = div().flex().flex_col().gap(px(3.));
        for (parent, name) in std::iter::once((None, "Page".to_string())).chain(
            parents
                .into_iter()
                .filter(|(p, _)| *p != id)
                .map(|(p, n)| (Some(p), n)),
        ) {
            list = list.child(
                div()
                    .id(("parent-choice", parent.unwrap_or(0)))
                    .px(px(8.))
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .rounded(px(4.))
                    .bg(if current == parent {
                        rgb(0xe6efff)
                    } else {
                        rgb(0xf8f9f6)
                    })
                    .child(name)
                    .cursor_pointer()
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.commit_field();
                        s.end_edit();
                        if parent.is_some_and(|p| s.editor.is_locked(p)) {
                            s.set_status(
                                "Unlock the destination frame before moving a layer into it",
                                true,
                            );
                        } else if !s.editor.reparent_selected(parent)
                            && s.editor.parent_of(id) != parent
                        {
                            s.set_status("A frame cannot contain itself", true);
                        }
                        cx.notify();
                    })),
            );
        }
        self.section("Parent", list)
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let mut center = div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .bg(rgb(0xecedeb))
            .child(
                div()
                    .id("canvas")
                    .size_full()
                    .cursor(self.canvas_cursor())
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
                    .on_mouse_down(MouseButton::Middle, cx.listener(Self::mouse_down))
                    .on_scroll_wheel(cx.listener(Self::scroll))
                    .child(
                        canvas(
                            move |_, _, _| (),
                            move |bounds, (), window, cx| {
                                entity.update(cx, |app, cx| app.paint_canvas(bounds, window, cx));
                            },
                        )
                        .size_full(),
                    ),
            );
        if let Some(toolbar) = self.text_selection_toolbar(cx) {
            center = center.child(toolbar);
        }
        let mut root = div()
            .id("compose")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .text_color(rgb(INK))
            .text_size(px(12.))
            .font_family(".SystemUIFont")
            .track_focus(&self.focus)
            .on_action(cx.listener(|s, _: &crate::NewDocument, _, cx| s.request(Pending::New, cx)))
            .on_action(cx.listener(|s, _: &crate::OpenDocument, _, cx| s.choose_file(false, cx)))
            .on_action(cx.listener(|s, _: &crate::SaveDocument, _, cx| s.save(false, cx)))
            .on_action(cx.listener(|s, _: &crate::SaveAs, _, cx| s.save(true, cx)))
            .on_action(cx.listener(|s, _: &crate::ExportMenu, _, cx| {
                s.toggle_popup(Popup::Export);
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &crate::ExportCompose, _, cx| {
                s.export_document(crate::output::OutputFormat::Compose, cx)
            }))
            .on_action(cx.listener(|s, _: &crate::ExportPng, _, cx| {
                s.export_document(crate::output::OutputFormat::Png, cx)
            }))
            .on_action(cx.listener(|s, _: &crate::PrintDocument, _, cx| s.print_document(cx)))
            .on_action(cx.listener(|s, _: &crate::GroupSelection, _, cx| {
                s.group_selection();
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &crate::UngroupSelection, _, cx| {
                s.ungroup_selection();
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &crate::UndoEdit, _, cx| {
                s.undo(false);
                cx.notify()
            }))
            .on_action(cx.listener(|s, _: &crate::RedoEdit, _, cx| {
                s.undo(true);
                cx.notify()
            }))
            .on_action(
                cx.listener(|s, _: &crate::QuitCompose, _, cx| s.request(Pending::Close, cx)),
            )
            .on_action(cx.listener(|s, _: &crate::CopyText, w, cx| s.menu_key("cmd-c", w, cx)))
            .on_action(cx.listener(|s, _: &crate::CutText, w, cx| s.menu_key("cmd-x", w, cx)))
            .on_action(cx.listener(|s, _: &crate::PasteText, w, cx| s.menu_key("cmd-v", w, cx)))
            .on_action(cx.listener(|s, _: &crate::SelectText, w, cx| s.menu_key("cmd-a", w, cx)))
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::mouse_up))
            .child(self.toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(!self.preview, |d| d.child(self.layers(cx)))
                    .child(center)
                    .when(!self.preview, |d| d.child(self.inspector(cx))),
            )
            .child(
                div()
                    .h(px(26.))
                    .flex_shrink_0()
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .text_size(px(10.))
                    .text_color(if self.error {
                        rgb(0xb44535)
                    } else {
                        rgb(MUTED)
                    })
                    .child(self.status.clone())
                    .child(format!(
                        "Page {} of {}   ·   {}",
                        self.editor.page + 1,
                        self.editor.document.pages.len(),
                        if self.dirty() {
                            "Unsaved changes"
                        } else {
                            "All changes saved"
                        }
                    )),
            );
        if self.popup.is_some() {
            root = root.child(self.popup_view(cx));
        }
        if self.pending.is_some() {
            root =
                root.child(
                    div()
                        .occlude()
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x171e1740))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(370.))
                                .p(px(24.))
                                .bg(rgb(0xffffff))
                                .rounded(px(9.))
                                .shadow_lg()
                                .child(
                                    div()
                                        .text_size(px(18.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Save your changes?"),
                                )
                                .child(
                                    div().mt(px(12.)).mb(px(22.)).text_color(rgb(MUTED)).child(
                                        "This document has changes that haven’t been saved.",
                                    ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(6.))
                                        .child(self.button(
                                            "cancel-pending",
                                            "Cancel",
                                            None,
                                            false,
                                            self.busy,
                                            cx,
                                            |s, _, _| s.pending = None,
                                        ))
                                        .child(self.button(
                                            "discard-pending",
                                            "Discard",
                                            None,
                                            false,
                                            self.busy,
                                            cx,
                                            |s, _, cx| {
                                                if let Some(p) = s.pending.take() {
                                                    s.execute(p, cx)
                                                }
                                            },
                                        ))
                                        .child(self.button(
                                            "save-pending",
                                            "Save",
                                            None,
                                            true,
                                            self.busy,
                                            cx,
                                            |s, _, cx| s.save(false, cx),
                                        )),
                                ),
                        ),
                );
        }
        root
    }
}

pub(crate) struct Hint(pub(crate) SharedString);
impl Render for Hint {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(9.))
            .py(px(6.))
            .rounded(px(5.))
            .bg(rgb(0x252a25))
            .text_color(white())
            .text_size(px(11.))
            .child(self.0.clone())
    }
}
