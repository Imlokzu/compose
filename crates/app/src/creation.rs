use crate::{
    panels::{INK, LINE, MUTED, icon, label},
    workspace::{Field, Pending, Workspace},
};
use document::{DocumentMode, presets::Preset};
use gpui::{prelude::*, *};

pub struct Creation {
    pub preset: Preset,
    pub mode: DocumentMode,
    pub name: String,
    pub can_cancel: bool,
}
impl Default for Creation {
    fn default() -> Self {
        Self {
            preset: Preset::Blank,
            mode: DocumentMode::Compose,
            name: "Untitled".into(),
            can_cancel: false,
        }
    }
}
impl Workspace {
    pub fn capture_control(
        &self,
        id: &'static str,
        child: impl IntoElement,
        cx: &Context<Self>,
    ) -> Div {
        let entity = cx.entity();
        div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.first() {
                    entity.update(cx, |s, _| {
                        s.selection_control_bounds.insert(id, *bounds);
                    });
                }
            })
            .child(child)
    }
    pub fn open_creation(&mut self, mode: DocumentMode) {
        if self.busy || self.pending.is_some() {
            return;
        }
        self.close_popup();
        self.end_edit();
        self.gesture = crate::workspace::Gesture::None;
        self.guides.clear();
        self.preset_scroll.set_offset(point(px(0.), px(0.)));
        self.creation = Some(Creation {
            mode,
            can_cancel: true,
            ..Default::default()
        });
    }
    pub fn select_preset(&mut self, preset: Preset) {
        self.commit_field();
        let Some(draft) = &mut self.creation else {
            return;
        };
        if !preset.supports(draft.mode) {
            return;
        }
        let previous = draft.preset.document_name();
        if draft.name == previous || draft.name.is_empty() {
            draft.name = preset.document_name().into();
        }
        draft.preset = preset;
    }
    pub fn create_from_preset(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.pending.is_some() {
            return;
        }
        self.commit_field();
        let Some(draft) = &self.creation else {
            return;
        };
        match draft.preset.create(draft.mode) {
            Ok(mut doc) => {
                if !draft.name.trim().is_empty() {
                    doc.name = draft.name.trim().into();
                }
                self.request(Pending::Create(Box::new(doc)), cx);
            }
            Err(error) => self.set_status(error, true),
        }
    }
    pub fn creation_key(
        &mut self,
        key: &str,
        command: bool,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if command {
            match key {
                "o" => self.choose_file(false, cx),
                "n" => self.open_creation(DocumentMode::Compose),
                "q" => self.request(Pending::Close, cx),
                "a" | "c" | "x" | "v" if self.field.is_some() => return false,
                _ => {}
            }
        } else {
            match key {
                "escape" => {
                    if self.field.is_some() {
                        self.field = None;
                    } else if self.creation.as_ref().is_some_and(|d| d.can_cancel) {
                        self.creation = None;
                        self.focus.focus(window);
                    }
                }
                "enter" => self.create_from_preset(cx),
                "tab" => {
                    if self.field.is_some() {
                        self.commit_field();
                    } else if !shift {
                        let name = self.creation.as_ref().unwrap().name.clone();
                        self.begin_field(Field::NewDocumentName, name);
                    }
                }
                _ if self.field.is_some() => return false,
                _ => {}
            }
        }
        cx.notify();
        true
    }
    pub fn load_preset_previews(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let previews = cx
                .background_executor()
                .spawn(async move {
                    let mut renderer = export::PageRenderer::default();
                    Preset::ALL
                        .into_iter()
                        .filter_map(|preset| {
                            let doc = preset.create(DocumentMode::Compose).ok()?;
                            let png = renderer.render(&doc, 0, 72).ok()?.encode_png().ok()?;
                            let (image, _, _) = Workspace::decode_image(&png).ok()?;
                            Some((preset, image))
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            this.update(cx, |s, cx| {
                s.preset_previews.extend(previews);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub fn creation_view(&self, cx: &Context<Self>) -> Stateful<Div> {
        let draft = self.creation.as_ref().unwrap();
        let entity = cx.entity();
        let mut choices = div().flex().flex_wrap().gap(px(10.)).w_full();
        for preset in Preset::ALL {
            let id = match preset {
                Preset::Blank => "preset-blank",
                Preset::Resume => "preset-resume",
                Preset::PhotoResume => "preset-photo-resume",
                Preset::PhotoCover => "preset-photo-cover",
                Preset::Report => "preset-report",
                Preset::Letter => "preset-letter",
                Preset::Portfolio => "preset-portfolio",
                Preset::Poster => "preset-poster",
            };
            let selected = draft.preset == preset;
            let disabled = !preset.supports(draft.mode);
            let image = self.preset_previews.get(&preset).cloned();
            choices = choices.child(
                self.capture_control(
                    id,
                    div()
                        .id(id)
                        .w(px(119.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(if selected { 0x2878ed } else { LINE }))
                        .bg(rgb(if selected { 0xf2f6ff } else { 0xffffff }))
                        .when(!disabled, |d| {
                            d.cursor_pointer().hover(|d| d.border_color(rgb(0x9db6d6)))
                        })
                        .child(
                            div()
                                .h(px(94.))
                                .p(px(10.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .overflow_hidden()
                                .bg(rgb(0xf2f3ef))
                                .rounded_t(px(5.))
                                .when_some(image, |d, image| {
                                    d.child(
                                        img(image)
                                            .w(px(97.))
                                            .h(px(74.))
                                            .object_fit(ObjectFit::Contain),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .p(px(10.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(preset.name())
                                .child(
                                    label(if disabled {
                                        "Compose only"
                                    } else if selected {
                                        "Selected"
                                    } else {
                                        "A4 · editable"
                                    })
                                    .mt(px(4.)),
                                ),
                        )
                        .when(disabled, |d| d.opacity(0.45))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.select_preset(preset);
                            cx.notify();
                        })),
                    cx,
                ),
            );
        }
        let mut modes = div().flex().gap(px(8.));
        for (mode, id, title, description, glyph) in [
            (
                DocumentMode::Compose,
                "create-mode-compose",
                "Compose",
                "Design freely",
                "components",
            ),
            (
                DocumentMode::Word,
                "create-mode-word",
                "Word-compatible",
                "Export editable .docx",
                "word",
            ),
        ] {
            let disabled = !draft.preset.supports(mode);
            let active = draft.mode == mode;
            modes = modes.child(
                self.capture_control(
                    id,
                    div()
                        .id(id)
                        .w(px(248.))
                        .p(px(12.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(if active { 0x2878ed } else { LINE }))
                        .bg(rgb(if active { 0xf2f6ff } else { 0xffffff }))
                        .when(!disabled, |d| {
                            d.cursor_pointer().hover(|d| d.bg(rgb(0xf4f6f2)))
                        })
                        .when(disabled, |d| d.opacity(0.45))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icon(glyph))
                                .child(div().flex_1().font_weight(FontWeight::MEDIUM).child(title))
                                .when(active, |d| d.child(icon("check").text_color(rgb(0x2878ed)))),
                        )
                        .child(
                            label(if disabled {
                                format!("{} requires Compose", draft.preset.name())
                            } else {
                                description.into()
                            })
                            .mt(px(6.)),
                        )
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.commit_field();
                            if let Some(draft) = &mut s.creation
                                && draft.preset.supports(mode)
                            {
                                draft.mode = mode;
                            }
                            cx.notify();
                        })),
                    cx,
                ),
            );
        }
        let preview = div()
            .w(px(260.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(14.))
            .child(
                div()
                    .w(px(224.))
                    .h(px(317.))
                    .bg(rgb(0xfffefb))
                    .border_1()
                    .border_color(rgb(0xdfe3da))
                    .shadow_md()
                    .when_some(
                        self.preset_previews.get(&draft.preset).cloned(),
                        |d, image| d.child(img(image).size_full().object_fit(ObjectFit::Contain)),
                    ),
            )
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(draft.preset.name()),
            )
            .child(label(draft.preset.description()))
            .when(draft.preset.has_photo(), |d| {
                d.child(
                    label("Use your own photo: select the image, then Replace image.")
                        .w(px(224.))
                        .line_height(px(16.))
                        .text_align(TextAlign::Center),
                )
            });
        let name_field = self.capture_control(
            "create-name",
            self.field_view(Field::NewDocumentName, "", draft.name.clone(), cx)
                .h(px(36.)),
            cx,
        );
        let form = div().w(px(508.)).flex().flex_col().gap(px(12.))
            .child(div().child(div().text_size(px(23.)).line_height(px(28.)).font_weight(FontWeight::SEMIBOLD).child("What will you make?"))
                .child(label("Start fresh or choose an editable layout.").mt(px(6.))))
            .child(div().flex().flex_col().gap(px(6.)).child(label("8 editable layouts · scroll for more"))
                .child(self.capture_control("preset-gallery",div().id("preset-gallery").h(px(194.)).w(px(508.)).overflow_y_scroll().track_scroll(&self.preset_scroll).child(choices),cx)))
            .child(div().flex().flex_col().gap(px(8.)).child(label("DOCUMENT MODE")).child(modes)
                .child(div().text_size(px(11.)).line_height(px(17.)).text_color(rgb(MUTED)).child(if draft.mode == DocumentMode::Word {
                    "Word uses a single text column. Shapes, rounded images, groups and free placement are unavailable. Fonts and page breaks may change in Word."
                } else {
                    "Text boxes, images, shapes and effects. Save .compose, export PNG or print. This mode cannot export to Word."
                })))
            .child(div().flex().flex_col().gap(px(8.)).child(label("DOCUMENT NAME")).child(name_field))
            .child(div().flex().items_center().justify_between()
                .child(label("Saved locally as .compose in either mode"))
                .child(self.capture_control("create-document", self.button("create-document", "Create document", Some("plus"), true, self.busy, cx,
                    |s, _, cx| s.create_from_preset(cx)).h(px(36.)), cx)));
        let mut root = div()
            .id("create-compose")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(0xf7f8f4))
            .text_color(rgb(INK))
            .text_size(px(12.))
            .font_family(".SystemUIFont")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .on_action(cx.listener(|s, _: &crate::OpenDocument, _, cx| s.choose_file(false, cx)))
            .on_action(cx.listener(|s, _: &crate::NewDocument, _, cx| {
                s.open_creation(DocumentMode::Compose);
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &crate::NewWordDocument, _, cx| {
                s.open_creation(DocumentMode::Word);
                cx.notify();
            }))
            .on_action(
                cx.listener(|s, _: &crate::QuitCompose, _, cx| s.request(Pending::Close, cx)),
            )
            .on_action(cx.listener(|s, _: &crate::CopyText, w, cx| s.menu_key("cmd-c", w, cx)))
            .on_action(cx.listener(|s, _: &crate::CutText, w, cx| s.menu_key("cmd-x", w, cx)))
            .on_action(cx.listener(|s, _: &crate::PasteText, w, cx| s.menu_key("cmd-v", w, cx)))
            .on_action(cx.listener(|s, _: &crate::SelectText, w, cx| s.menu_key("cmd-a", w, cx)))
            .child(
                canvas(
                    move |_, _, _| (),
                    move |bounds, (), w, cx| {
                        entity.update(cx, |s, cx| {
                            w.handle_input(
                                &s.focus,
                                ElementInputHandler::new(
                                    if s.field.is_some() {
                                        s.field_bounds
                                    } else {
                                        bounds
                                    },
                                    cx.entity(),
                                ),
                                cx,
                            )
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .h(px(64.))
                    .px(px(32.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(icon("logo").size(px(27.)))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Compose"),
                            ),
                    )
                    .child(self.capture_control(
                        "create-open",
                        self.button(
                            "create-open",
                            "Open document…",
                            Some("open"),
                            false,
                            self.busy,
                            cx,
                            |s, _, cx| s.choose_file(false, cx),
                        ),
                        cx,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .gap(px(40.))
                            .items_center()
                            .child(preview)
                            .child(form),
                    ),
            )
            .child(
                div()
                    .h(px(44.))
                    .px(px(32.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_color(rgb(MUTED))
                    .child(if self.error {
                        self.status.clone()
                    } else {
                        "Write like a document. Design like a canvas.".into()
                    })
                    .when(draft.can_cancel, |d| {
                        d.child(self.capture_control(
                            "create-cancel",
                            self.button(
                                "create-cancel",
                                "Back to document",
                                Some("left"),
                                false,
                                self.busy,
                                cx,
                                |s, _, _| {
                                    s.field = None;
                                    s.creation = None;
                                },
                            ),
                            cx,
                        ))
                    }),
            );
        if self.pending.is_some() {
            root = root.child(self.unsaved_prompt(cx));
        }
        root
    }
    pub fn unsaved_prompt(&self, cx: &Context<Self>) -> Div {
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
                    .bg(white())
                    .rounded(px(9.))
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Save your changes?"),
                    )
                    .child(
                        div()
                            .mt(px(12.))
                            .mb(px(22.))
                            .text_color(rgb(MUTED))
                            .child("This document has changes that haven’t been saved."),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(6.))
                            .child(self.capture_control(
                                "cancel-pending",
                                self.button(
                                    "cancel-pending",
                                    "Cancel",
                                    None,
                                    false,
                                    self.busy,
                                    cx,
                                    |s, _, _| s.pending = None,
                                ),
                                cx,
                            ))
                            .child(self.capture_control(
                                "discard-pending",
                                self.button(
                                    "discard-pending",
                                    "Discard",
                                    None,
                                    false,
                                    self.busy,
                                    cx,
                                    |s, _, cx| {
                                        if let Some(p) = s.pending.take() {
                                            s.execute(p, cx);
                                        }
                                    },
                                ),
                                cx,
                            ))
                            .child(self.capture_control(
                                "save-pending",
                                self.button(
                                    "save-pending",
                                    "Save",
                                    None,
                                    true,
                                    self.busy,
                                    cx,
                                    |s, _, cx| s.save(false, cx),
                                ),
                                cx,
                            )),
                    ),
            )
    }
}
