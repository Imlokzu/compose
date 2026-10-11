use crate::{
    panels::{INK, LINE, MUTED, icon},
    workspace::{Field, Gesture, Popup, Tool, Workspace},
};
use document::NodeId;
use gpui::{prelude::*, *};

#[derive(Clone, Copy)]
pub enum ContextAction {
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
    Edit,
    Move,
    Rename,
    Up,
    Down,
    Lock,
    SelectAll,
    Group,
    Ungroup,
    Fit,
    Text,
    Link,
}

impl Workspace {
    pub fn canvas_context_menu(
        &mut self,
        e: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.preview || self.pending.is_some() || self.creation.is_some() || self.busy {
            return;
        }
        let hit = self
            .editor
            .hit_test_context(self.to_doc(e.position), self.editing);
        self.open_object_context(hit, e.position, window, cx);
        cx.stop_propagation();
    }
    pub fn open_object_context(
        &mut self,
        id: Option<NodeId>,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.preview || self.pending.is_some() || self.busy {
            return;
        }
        self.close_popup();
        if id.is_none_or(|id| !self.editor.is_selected(id)) {
            self.end_edit();
            self.editor.select_only(id);
        }
        self.gesture = Gesture::None;
        self.guides.clear();
        self.context_position = position;
        self.popup = Some(Popup::Context);
        self.focus.focus(window);
        cx.notify();
    }
    pub fn context_action(&mut self, action: ContextAction, cx: &mut Context<Self>) {
        self.close_popup();
        match action {
            ContextAction::Copy | ContextAction::Cut => {
                if let Some(item) = self.copy_item(matches!(action, ContextAction::Cut)) {
                    cx.write_to_clipboard(item);
                }
            }
            ContextAction::Paste => {
                if let Some(item) = cx.read_from_clipboard() {
                    self.paste_item(&item);
                }
            }
            ContextAction::Duplicate => {
                self.end_edit();
                self.editor.duplicate_selected();
            }
            ContextAction::Delete => {
                self.end_edit();
                self.editor.delete_selected();
            }
            ContextAction::Edit if !self.selection_locked() && self.text().is_some() => {
                self.tool = Tool::Select;
                self.editing = true;
                self.caret = self.text().unwrap().content.len();
                self.anchor = self.caret;
                self.set_status(
                    "Select a passage to format · drag the border to move the text box",
                    false,
                );
            }
            ContextAction::Move => {
                self.end_edit();
                self.tool = Tool::Select;
                self.set_status(
                    if self.word_mode() {
                        "Word blocks follow reading order. Use Move up / Move down."
                    } else {
                        "Drag the selected box to move it · double-click to edit text"
                    },
                    false,
                );
            }
            ContextAction::Rename
                if !self.selection_locked() && self.editor.selection_count() == 1 =>
            {
                self.end_edit();
                let name = self.editor.selected_node().unwrap().name.clone();
                self.begin_field(Field::Name, name);
            }
            ContextAction::Up | ContextAction::Down => {
                self.end_edit();
                self.editor
                    .reorder_selected(matches!(action, ContextAction::Up));
            }
            ContextAction::Lock => {
                self.end_edit();
                self.editor.set_selection_locked(!self.selection_locked());
            }
            ContextAction::SelectAll => {
                if self.editing {
                    self.anchor = 0;
                    self.caret = self.text().map(|t| t.content.len()).unwrap_or(0);
                } else {
                    self.editor.select_all();
                }
            }
            ContextAction::Group => self.group_selection(),
            ContextAction::Ungroup => self.ungroup_selection(),
            ContextAction::Fit => self.fit(),
            ContextAction::Text => {
                self.end_edit();
                self.tool = Tool::Text;
                self.set_status("Drag on the page to draw a text box", false);
            }
            ContextAction::Link => self.open_link_editor(),
            _ => {}
        }
        self.measure();
        cx.notify();
    }
    #[allow(clippy::too_many_arguments)]
    fn context_row(
        &self,
        id: &'static str,
        title: &'static str,
        glyph: &'static str,
        shortcut: &'static str,
        disabled: bool,
        action: ContextAction,
        cx: &Context<Self>,
    ) -> Div {
        self.capture_control(
            id,
            div()
                .id(id)
                .h(px(30.))
                .px(px(9.))
                .rounded(px(4.))
                .flex()
                .items_center()
                .gap(px(9.))
                .text_color(rgb(if disabled { 0xaeb4ab } else { INK }))
                .when(!disabled, |d| {
                    d.cursor_pointer().hover(|d| d.bg(rgb(0xeef3fc)))
                })
                .child(
                    icon(glyph)
                        .size(px(14.))
                        .when(disabled, |d| d.opacity(0.35)),
                )
                .child(div().flex_1().child(title))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(MUTED))
                        .child(shortcut),
                )
                .on_click(cx.listener(move |s, _, _, cx| {
                    if !disabled {
                        s.context_action(action, cx);
                    }
                })),
            cx,
        )
    }
    pub fn context_menu_view(&self, cx: &Context<Self>) -> AnyElement {
        let selected = self.editor.selected().is_some();
        let locked = self.selection_locked();
        let text = self.text().is_some() && self.editor.selection_count() == 1;
        let passage = self.editing && !self.range().is_empty();
        let divider = || div().h(px(1.)).my(px(4.)).bg(rgb(LINE));
        let mut menu = div()
            .w(px(238.))
            .p(px(5.))
            .rounded(px(7.))
            .bg(white())
            .border_1()
            .border_color(rgb(LINE))
            .shadow_lg()
            .occlude();
        if text {
            menu = menu
                .child(self.context_row(
                    "context-edit",
                    if self.editing {
                        "Select / move text box"
                    } else {
                        "Edit text"
                    },
                    "text",
                    "Enter",
                    locked,
                    if self.editing {
                        ContextAction::Move
                    } else {
                        ContextAction::Edit
                    },
                    cx,
                ))
                .child(self.context_row(
                    "context-link",
                    "Add / edit link",
                    "link",
                    "Ctrl/⌘K",
                    locked,
                    ContextAction::Link,
                    cx,
                ))
                .child(divider());
        }
        menu = menu
            .child(self.context_row(
                "context-cut",
                "Cut",
                "clear-style",
                "⌘X",
                !selected || locked,
                ContextAction::Cut,
                cx,
            ))
            .child(self.context_row(
                "context-copy",
                "Copy",
                "duplicate",
                "⌘C",
                !selected,
                ContextAction::Copy,
                cx,
            ))
            .child(self.context_row(
                "context-paste",
                "Paste",
                "page",
                "⌘V",
                locked,
                ContextAction::Paste,
                cx,
            ));
        if selected && !passage {
            menu = menu
                .child(divider())
                .child(self.context_row(
                    "context-duplicate",
                    "Duplicate",
                    "duplicate",
                    "⌘D",
                    locked,
                    ContextAction::Duplicate,
                    cx,
                ))
                .child(self.context_row(
                    "context-rename",
                    "Rename",
                    "text",
                    "",
                    locked || self.editor.selection_count() != 1,
                    ContextAction::Rename,
                    cx,
                ))
                .child(self.context_row(
                    "context-up",
                    if self.word_mode() {
                        "Move later"
                    } else {
                        "Bring forward"
                    },
                    "up",
                    "⌘]",
                    locked,
                    ContextAction::Up,
                    cx,
                ))
                .child(self.context_row(
                    "context-down",
                    if self.word_mode() {
                        "Move earlier"
                    } else {
                        "Send backward"
                    },
                    "down",
                    "⌘[",
                    locked,
                    ContextAction::Down,
                    cx,
                ));
            if !self.word_mode() {
                menu = menu.child(self.context_row(
                    "context-group",
                    "Group",
                    "group",
                    "⌘G",
                    !self.editor.can_group(),
                    ContextAction::Group,
                    cx,
                ));
                if self.editor.selected_node().is_some_and(|n| n.is_group) {
                    menu = menu.child(self.context_row(
                        "context-ungroup",
                        "Ungroup",
                        "ungroup",
                        "⇧⌘G",
                        locked,
                        ContextAction::Ungroup,
                        cx,
                    ));
                }
            }
            let unlock = self.selection_locked();
            menu = menu
                .child(divider())
                .child(self.context_row(
                    "context-lock",
                    if unlock { "Unlock" } else { "Lock" },
                    if unlock { "unlock" } else { "lock" },
                    "",
                    false,
                    ContextAction::Lock,
                    cx,
                ))
                .child(self.context_row(
                    "context-delete",
                    "Delete",
                    "trash",
                    "⌫",
                    locked,
                    ContextAction::Delete,
                    cx,
                ));
        }
        if !selected {
            menu = menu
                .child(divider())
                .child(self.context_row(
                    "context-text",
                    "Draw text box",
                    "text",
                    "T",
                    false,
                    ContextAction::Text,
                    cx,
                ))
                .child(self.context_row(
                    "context-fit",
                    "Fit page",
                    "preview",
                    "⌘0",
                    false,
                    ContextAction::Fit,
                    cx,
                ));
        }
        menu = menu.child(divider()).child(self.context_row(
            "context-select-all",
            "Select all",
            "select",
            "⌘A",
            false,
            ContextAction::SelectAll,
            cx,
        ));
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .id("context-dismiss")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.close_popup();
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|s, _, _, cx| {
                            s.close_popup();
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            )
            .child(
                anchored()
                    .position(self.context_position)
                    .snap_to_window_with_margin(Edges {
                        top: px(8.),
                        bottom: px(32.),
                        left: px(8.),
                        right: px(8.),
                    })
                    .child(menu),
            )
            .into_any_element()
    }
}
