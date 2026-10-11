use crate::{
    panels::{icon, label},
    workspace::{Field, Gesture, Workspace},
};
use document::Point;
use editor::{Align, Distribute};
use gpui::{prelude::*, *};

impl Workspace {
    fn prepare_selection_action(&mut self) -> bool {
        if self.preview || self.pending.is_some() {
            return false;
        }
        let edit = self.field.clone();
        if edit.is_some() {
            self.error = false;
        }
        self.commit_field();
        if edit.is_some() && self.error {
            self.field = edit;
            self.caret_visible = true;
            return false;
        }
        self.end_edit();
        self.close_popup();
        self.gesture = Gesture::None;
        self.guides.clear();
        true
    }
    pub fn group_selection(&mut self) {
        if !self.prepare_selection_action() {
            return;
        }
        let count = self.editor.selection_count();
        if self.editor.group_selected().is_some() {
            self.set_status(
                format!("Grouped {count} layers · double-click to edit inside"),
                false,
            );
        } else {
            self.set_status(
                "Select two or more unlocked layers with the same parent to group",
                true,
            );
        }
    }
    pub fn ungroup_selection(&mut self) {
        if !self.prepare_selection_action() {
            return;
        }
        if self.editor.ungroup_selected() {
            self.set_status("Ungrouped · layer positions preserved", false);
        } else {
            self.set_status("Select an unlocked group to ungroup", true);
        }
    }
    pub fn align_selection(&mut self, align: Align) {
        if self.word_mode() {
            return;
        }
        if !self.prepare_selection_action() {
            return;
        }
        if self.editor.selection_count() > 1 {
            self.editor.align_selected(align);
        } else if let Some(bounds) = self.editor.selection_bounds() {
            let page = &self.editor.document.pages[self.editor.page];
            let delta = match align {
                Align::Left => Point {
                    x: 40. - bounds.x,
                    y: 0.,
                },
                Align::CenterX => Point {
                    x: (page.width - bounds.width) / 2. - bounds.x,
                    y: 0.,
                },
                Align::Right => Point {
                    x: page.width - 40. - bounds.width - bounds.x,
                    y: 0.,
                },
                Align::Top => Point {
                    x: 0.,
                    y: 40. - bounds.y,
                },
                Align::CenterY => Point {
                    x: 0.,
                    y: (page.height - bounds.height) / 2. - bounds.y,
                },
                Align::Bottom => Point {
                    x: 0.,
                    y: page.height - 40. - bounds.height - bounds.y,
                },
            };
            self.editor.translate_selection(delta);
        }
        self.set_status("Alignment applied", false);
    }
    pub fn distribute_selection(&mut self, direction: Distribute) {
        if self.word_mode() {
            return;
        }
        if !self.prepare_selection_action() {
            return;
        }
        if self.editor.distribute_selected(direction) {
            self.set_status("Spacing distributed evenly", false);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn selection_button(
        &self,
        id: &'static str,
        title: &'static str,
        image: &'static str,
        disabled: bool,
        cx: &Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Div {
        let entity = cx.entity();
        div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bound) = bounds.first() {
                    entity.update(cx, |s, _| {
                        s.selection_control_bounds.insert(id, *bound);
                    });
                }
            })
            .child(self.button(
                id,
                title,
                Some(image),
                false,
                disabled,
                cx,
                move |s, w, cx| {
                    if s.prepare_selection_action() {
                        action(s, w, cx);
                    }
                },
            ))
    }
    pub fn selection_inspector(&self, cx: &Context<Self>) -> Div {
        let locked = self.selection_locked();
        let count = self.editor.selection_count();
        let bounds = self.editor.selection_bounds().unwrap_or_default();
        let mut body = div();
        if let Some(group) = self.editor.selected_node().filter(|n| n.is_group) {
            body = body.child(
                self.section(
                    "Group",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(9.))
                        .when(!locked, |d| {
                            d.child(self.field_view(Field::Name, "", group.name.clone(), cx))
                        })
                        .when(locked, |d| d.child(group.name.clone()))
                        .child(label(format!(
                            "{} layers · double-click to edit inside",
                            group.children.len()
                        )))
                        .child(label("Opacity"))
                        .when(locked, |d| {
                            d.child(label(format!("{:.0}% · locked", group.opacity * 100.)))
                        })
                        .when(!locked, |d| {
                            d.child(self.field_view(
                                Field::Opacity,
                                "%",
                                format!("{:.0}", group.opacity * 100.),
                                cx,
                            ))
                        }),
                ),
            );
        } else {
            body = body.child(
                self.section(
                    &format!("{count} layers selected"),
                    div()
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .child(icon("group"))
                        .child(label("Shift-click to add or remove")),
                ),
            );
        }
        body = body.child(
            self.section(
                "Selection bounds",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(label("Position"))
                            .child(format!("{:.0}, {:.0} pt", bounds.x, bounds.y)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(label("Size"))
                            .child(format!("{:.0} × {:.0} pt", bounds.width, bounds.height)),
                    )
                    .child(label("Drag to move together · arrows to nudge")),
            ),
        );
        let mut align = div().flex().gap(px(4.));
        for (id, ico, alignment) in [
            ("selection-left", "align-left", Align::Left),
            ("selection-center-x", "align-center-x", Align::CenterX),
            ("selection-right", "align-right", Align::Right),
            ("selection-top", "align-top", Align::Top),
            ("selection-center-y", "align-center-y", Align::CenterY),
            ("selection-bottom", "align-bottom", Align::Bottom),
        ] {
            align = align.child(
                self.selection_button(id, "", ico, locked, cx, move |s, _, _| {
                    s.align_selection(alignment)
                }),
            );
        }
        body = body.child(self.section(
            if count > 1 {
                "Align selection"
            } else {
                "Align to page"
            },
            align,
        ));
        if count > 1 {
            body = body.child(
                self.section(
                    "Space evenly",
                    div()
                        .flex()
                        .gap(px(7.))
                        .child(self.selection_button(
                            "selection-distribute-x",
                            "Horizontal",
                            "distribute-x",
                            locked || count < 3,
                            cx,
                            |s, _, _| s.distribute_selection(Distribute::Horizontal),
                        ))
                        .child(self.selection_button(
                            "selection-distribute-y",
                            "Vertical",
                            "distribute-y",
                            locked || count < 3,
                            cx,
                            |s, _, _| s.distribute_selection(Distribute::Vertical),
                        )),
                ),
            );
        }
        let organize = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .gap(px(7.))
                    .child(self.selection_button(
                        "selection-group",
                        "Group",
                        "group",
                        !self.editor.can_group(),
                        cx,
                        |s, _, _| s.group_selection(),
                    ))
                    .child(self.selection_button(
                        "selection-ungroup",
                        "Ungroup",
                        "ungroup",
                        !self.editor.can_ungroup(),
                        cx,
                        |s, _, _| s.ungroup_selection(),
                    )),
            )
            .child(label("⌘G   Group       ⇧⌘G   Ungroup"))
            .child(self.selection_button(
                "selection-lock",
                if locked {
                    "Unlock selection"
                } else {
                    "Lock selection"
                },
                if locked { "unlock" } else { "lock" },
                false,
                cx,
                move |s, _, _| {
                    s.end_edit();
                    s.editor.set_selection_locked(!locked);
                },
            ));
        body = body.child(self.section("Organize", organize));
        body.child(
            self.section(
                "Arrange",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(7.))
                    .child(
                        div()
                            .flex()
                            .gap(px(7.))
                            .child(self.selection_button(
                                "selection-back",
                                "Backward",
                                "down",
                                locked,
                                cx,
                                |s, _, _| s.editor.reorder_selected(false),
                            ))
                            .child(self.selection_button(
                                "selection-forward",
                                "Forward",
                                "up",
                                locked,
                                cx,
                                |s, _, _| s.editor.reorder_selected(true),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(7.))
                            .child(self.selection_button(
                                "selection-duplicate",
                                "Duplicate",
                                "duplicate",
                                locked,
                                cx,
                                |s, _, _| s.editor.duplicate_selected(),
                            ))
                            .child(self.selection_button(
                                "selection-delete",
                                "Delete",
                                "trash",
                                locked,
                                cx,
                                |s, _, _| s.editor.delete_selected(),
                            )),
                    ),
            ),
        )
    }
}
