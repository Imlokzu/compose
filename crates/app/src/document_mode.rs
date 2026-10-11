use crate::{
    panels::{LINE, label},
    workspace::{Field, Gesture, Pending, Tool, Workspace},
};
use document::{DocumentMode, NodeKind, WORD_BLOCK_GAP, WORD_MARGIN};
use gpui::{prelude::*, *};

impl Workspace {
    pub fn word_mode(&self) -> bool {
        self.editor.document.mode == DocumentMode::Word
    }
    pub fn switch_document_mode(&mut self, mode: DocumentMode) {
        if self.preview || self.busy || self.pending.is_some() {
            return;
        }
        let field = self.field.clone();
        if field.is_some() {
            self.error = false;
        }
        self.commit_field();
        if field.is_some() && self.error {
            self.field = field;
            self.caret_visible = true;
            return;
        }
        if self.editor.document.mode == mode {
            self.close_popup();
            return;
        }
        if mode == DocumentMode::Word
            && let Some(reason) = self.editor.document.word_issues().first()
        {
            self.set_status(
                format!("Keep this in Compose mode: {reason}. Start a new Word document instead."),
                true,
            );
            return;
        }
        self.image_picker.invalidate();
        self.end_edit();
        self.gesture = Gesture::None;
        self.guides.clear();
        self.close_popup();
        self.editor.checkpoint();
        self.editor.document.mode = mode;
        self.assets_tab = false;
        self.tool = Tool::Select;
        if mode == DocumentMode::Word {
            for page in &mut self.editor.document.pages {
                page.nodes.sort_by(|a, b| {
                    a.frame
                        .y
                        .total_cmp(&b.frame.y)
                        .then(a.frame.x.total_cmp(&b.frame.x))
                });
            }
        }
        self.layouts.clear();
        self.measure();
        self.fitted = false;
        self.set_status(
            match mode {
                DocumentMode::Word => "Word mode · text and images flow in reading order",
                DocumentMode::Compose => "Compose mode · freeform canvas · Word export disabled",
            },
            false,
        );
    }
    pub fn reflow_word(&mut self) {
        if !self.word_mode() {
            return;
        }
        for page in &mut self.editor.document.pages {
            let width = page.width - 2. * WORD_MARGIN;
            let mut y = WORD_MARGIN;
            for node in &mut page.nodes {
                if !node.visible {
                    continue;
                }
                let height = match &node.kind {
                    NodeKind::Text(text) => {
                        let refresh = self
                            .layouts
                            .get(&node.id)
                            .is_none_or(|(previous, old, _)| previous != text || *old != width);
                        if refresh {
                            let layout = self.engine.layout(text, width);
                            self.layouts.insert(node.id, (text.clone(), width, layout));
                        }
                        node.frame.width = width;
                        self.layouts[&node.id].2.height.max(1.)
                    }
                    NodeKind::Image { asset } => {
                        let ratio = self
                            .images
                            .get(asset)
                            .map(|i| {
                                let size = i.size(0);
                                size.width.0 as f32 / size.height.0 as f32
                            })
                            .or_else(|| {
                                let data = self.editor.document.assets.get(asset)?;
                                let (w, h) =
                                    image::ImageReader::new(std::io::Cursor::new(data.as_slice()))
                                        .with_guessed_format()
                                        .ok()?
                                        .into_dimensions()
                                        .ok()?;
                                (h > 0).then_some(w as f32 / h as f32)
                            });
                        if let Some(ratio) = ratio {
                            node.frame.width = node
                                .frame
                                .width
                                .min(width)
                                .min((page.height - 2. * WORD_MARGIN - WORD_BLOCK_GAP) * ratio);
                            node.frame.height = node.frame.width / ratio;
                        }
                        node.frame.height
                    }
                    _ => node.frame.height,
                };
                node.frame.x = WORD_MARGIN;
                node.frame.y = y;
                node.frame.height = height;
                y += height + WORD_BLOCK_GAP;
            }
        }
    }
    pub fn mode_menu(&self, cx: &Context<Self>) -> Div {
        let issues = self.editor.document.word_issues();
        let can_word = issues.is_empty();
        div().flex().flex_col().gap(px(12.))
            .child(label("Choose how this document works"))
            .child(self.mode_button("mode-compose","Compose · canvas",Some("components"),!self.word_mode(),false,cx,|s,_,_|s.switch_document_mode(DocumentMode::Compose)))
            .child(label("Freeform layouts, shapes, groups and effects. Export Compose, PNG or print."))
            .child(div().h(px(1.)).bg(rgb(LINE)))
            .child(self.mode_button("mode-word","Word-compatible",Some("word"),self.word_mode(),!can_word,cx,|s,_,_|s.switch_document_mode(DocumentMode::Word)))
            .child(label("Editable text and images in reading order. Export Word (.docx), Compose or PNG."))
            .when(can_word&&!self.word_mode()&&!self.editor.document.pages.iter().all(|p|p.nodes.is_empty()),|d|d.child(label("Switching reflows text and images into a single column. Undo restores this canvas.")))
            .when(!can_word,|d|d.child(div().text_color(rgb(0xa44335)).line_height(px(17.)).child(format!("This canvas has features Word mode cannot preserve: {}",issues.iter().take(2).cloned().collect::<Vec<_>>().join("; ")))))
            .child(div().h(px(1.)).bg(rgb(LINE)))
            .child(self.mode_button("new-word-document","New Word document",Some("plus"),false,self.busy,cx,|s,_,cx|{s.close_popup();s.request(Pending::NewWord,cx)}))
            .child(self.mode_button("new-compose-document","New Compose canvas",Some("plus"),false,self.busy,cx,|s,_,cx|{s.close_popup();s.request(Pending::New,cx)}))
    }
    #[allow(clippy::too_many_arguments)]
    fn mode_button(
        &self,
        id: &'static str,
        title: &'static str,
        icon: Option<&'static str>,
        active: bool,
        disabled: bool,
        cx: &Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
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
            .child(self.button(id, title, icon, active, disabled, cx, action))
    }
    pub fn word_node_properties(&self, cx: &Context<Self>) -> Div {
        let Some(node) = self.editor.selected_node() else {
            return div();
        };
        let mut body = div();
        if matches!(node.kind, NodeKind::Text(_)) {
            body = body.child(self.section(
                "Text color",
                self.button(
                    "word-text-color",
                    "Choose color…",
                    Some("palette"),
                    false,
                    self.selection_locked(),
                    cx,
                    |s, _, _| s.open_color(crate::workspace::ColorTarget::Fill),
                ),
            ));
        }
        if matches!(node.kind, NodeKind::Image { .. }) {
            body = body.child(
                self.section(
                    "Image size",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(9.))
                        .child(self.field_view(
                            Field::W,
                            "Width pt",
                            format!("{:.1}", node.frame.width),
                            cx,
                        ))
                        .child(label(
                            "Proportions stay locked; images fit the text column.",
                        )),
                ),
            );
        }
        let page = &self.editor.document.pages[self.editor.page];
        body=body.child(self.section("Word layout",div().flex().flex_col().gap(px(9.))
            .child(label("Text and images follow page order with 1-inch margins."))
            .when(node.frame.y+node.frame.height>page.height-WORD_MARGIN,|d|d.child(div().text_color(rgb(0xa44335)).child("This block extends past the page. Word will paginate its text; split it onto another page here before printing.")))));
        body.child(self.word_order_controls(cx))
    }
    pub fn word_order_controls(&self, cx: &Context<Self>) -> Div {
        self.section(
            "Reading order",
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(self.button(
                            "word-move-up",
                            "Move up",
                            Some("up"),
                            false,
                            self.selection_locked(),
                            cx,
                            |s, _, _| {
                                s.commit_field();
                                s.end_edit();
                                s.editor.reorder_selected(false);
                                s.measure();
                            },
                        ))
                        .child(self.button(
                            "word-move-down",
                            "Move down",
                            Some("down"),
                            false,
                            self.selection_locked(),
                            cx,
                            |s, _, _| {
                                s.commit_field();
                                s.end_edit();
                                s.editor.reorder_selected(true);
                                s.measure();
                            },
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(self.button(
                            "word-duplicate",
                            "Duplicate",
                            Some("duplicate"),
                            false,
                            self.selection_locked(),
                            cx,
                            |s, _, _| {
                                s.commit_field();
                                s.end_edit();
                                s.editor.duplicate_selected();
                                s.measure();
                            },
                        ))
                        .child(self.button(
                            "word-delete",
                            "Delete",
                            Some("trash"),
                            false,
                            self.selection_locked(),
                            cx,
                            |s, _, _| {
                                s.end_edit();
                                s.editor.delete_selected();
                                s.measure();
                            },
                        )),
                ),
        )
    }
}
