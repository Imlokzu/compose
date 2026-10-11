use crate::workspace::{ColorDrag, ColorPicker, ColorTarget, Field, FieldEdit, Popup, Workspace};
use document::{Color, NodeKind};
use gpui::*;
use text::palette::{Hsv, from_hex};

impl Workspace {
    pub fn selection_locked(&self) -> bool {
        self.editor.selection_locked()
    }
    pub fn close_popup(&mut self) {
        self.commit_field();
        if self.popup == Some(Popup::Images) {
            self.image_picker.invalidate();
            self.image_picker.replace_target = None;
        }
        self.popup = None;
        self.text_popup_anchor = None;
        self.picker = None;
        self.shape_change_target = None;
    }
    pub fn toggle_popup(&mut self, popup: Popup) {
        self.commit_field();
        self.text_popup_anchor = None;
        if self.popup == Some(popup) {
            self.close_popup();
        } else {
            self.popup = Some(popup);
            self.picker = None;
            if popup == Popup::Fonts {
                self.begin_field(Field::FontSearch, self.font_search.clone());
            }
        }
    }
    pub fn set_font(&mut self, family: &str) {
        if renderer::FONT_CHOICES.iter().any(|f| f.family == family) {
            self.format(|s| s.family = family.into());
            self.popup = None;
            self.text_popup_anchor = None;
            self.font_search.clear();
            self.set_status(format!("Typeface · {family}"), false);
        }
    }
    pub fn type_preset(&mut self, index: usize) {
        let Some(preset) = crate::text_menu::TEXT_PRESETS.get(index) else {
            return;
        };
        self.format(|s| preset.apply(s));
        self.set_status(format!("{} style applied", preset.name), false);
    }
    pub fn open_color(&mut self, target: ColorTarget) {
        if self.selection_locked() {
            return;
        }
        self.commit_field();
        self.text_popup_anchor = None;
        let c = self.current_color(target);
        self.picker = Some(ColorPicker {
            target,
            hsv: Hsv::from_color(c),
            drag: None,
            changed: false,
        });
        self.popup = Some(Popup::Colors);
    }
    pub fn current_color(&self, target: ColorTarget) -> Color {
        match target {
            ColorTarget::Stroke => self
                .editor
                .selected_node()
                .map(|n| n.appearance.stroke)
                .unwrap_or_default(),
            ColorTarget::Fill => {
                if self.text().is_some() {
                    self.active_style().color
                } else {
                    self.editor
                        .selected_node()
                        .map(|n| n.fill)
                        .unwrap_or_default()
                }
            }
        }
    }
    pub fn apply_color(&mut self, c: Color, checkpoint: bool) {
        if self.word_mode() && c.a != 255 {
            self.set_status("Word mode uses opaque text colors", true);
            return;
        }
        if self.selection_locked() {
            return;
        }
        let target = self
            .picker
            .as_ref()
            .map(|p| p.target)
            .unwrap_or(ColorTarget::Fill);
        if c == self.current_color(target) {
            return;
        }
        if target == ColorTarget::Fill && self.text().is_some() {
            self.format_with_history(|s| s.color = c, checkpoint)
        } else {
            if checkpoint {
                self.editor.checkpoint();
            }
            if let Some(n) = self.editor.selected_node_mut() {
                match target {
                    ColorTarget::Fill => n.fill = c,
                    ColorTarget::Stroke => {
                        n.appearance.stroke = c;
                    }
                }
            }
        }
    }
    pub fn choose_color(&mut self, c: Color) {
        self.commit_field();
        self.apply_color(c, true);
        if let Some(p) = &mut self.picker {
            p.hsv = Hsv::from_color(c);
        }
        self.set_status("Color updated", false);
    }
    pub fn document_colors(&self) -> Vec<Color> {
        fn collect(nodes: &[document::Node], colors: &mut Vec<Color>) {
            for n in nodes {
                let mut add = |c| {
                    if !colors.contains(&c) {
                        colors.push(c)
                    }
                };
                match &n.kind {
                    NodeKind::Text(t) => {
                        add(t.style.color);
                        for span in &t.spans {
                            add(span.style.color)
                        }
                    }
                    NodeKind::Image { .. } => {}
                    _ => add(n.fill),
                }
                if n.appearance.stroke_width > 0. {
                    add(n.appearance.stroke);
                }
                collect(&n.children, colors)
            }
        }
        let mut result = vec![];
        for p in &self.editor.document.pages {
            collect(&p.nodes, &mut result)
        }
        result.truncate(16);
        result
    }
    pub fn start_color_drag(&mut self, drag: ColorDrag, position: Point<Pixels>) {
        self.commit_field();
        if let Some(p) = &mut self.picker {
            p.drag = Some(drag);
            p.changed = false;
        }
        self.update_color_drag(position);
    }
    pub fn update_color_drag(&mut self, position: Point<Pixels>) {
        let Some(p) = &mut self.picker else { return };
        let Some(drag) = p.drag else { return };
        match drag {
            ColorDrag::Saturation => {
                p.hsv.saturation = (f32::from(position.x - self.color_bounds.left())
                    / f32::from(self.color_bounds.size.width))
                .clamp(0., 1.);
                p.hsv.value = 1.
                    - (f32::from(position.y - self.color_bounds.top())
                        / f32::from(self.color_bounds.size.height))
                    .clamp(0., 1.);
            }
            ColorDrag::Hue => {
                p.hsv.hue = (f32::from(position.x - self.hue_bounds.left())
                    / f32::from(self.hue_bounds.size.width))
                .clamp(0., 0.9999)
                    * 360.;
            }
        }
        let c = p.hsv.color();
        let target = p.target;
        let first = !p.changed;
        if c != self.current_color(target) {
            self.picker.as_mut().unwrap().changed = true;
            self.apply_color(c, first);
        }
    }
    pub fn insert_block(&mut self, component: editor::Component) {
        if self.word_mode() {
            self.set_status("Canvas components are available in Compose mode", true);
            return;
        }
        self.close_popup();
        self.end_edit();
        let page = &self.editor.document.pages[self.editor.page];
        let x = 40.;
        let y = (64. + page.nodes.len() as f32 * 20.).min(page.height - 220.);
        let id = self.editor.insert_component(component, x, y);
        self.editor.select_only(Some(id));
        self.assets_tab = false;
        self.measure();
        self.set_status(
            "Component inserted · expand its layers to edit every part",
            false,
        );
    }
    pub fn toggle_shadow(&mut self) {
        if self.word_mode() {
            self.set_status("Shadows are available in Compose mode", true);
            return;
        }
        self.commit_field();
        if self.selection_locked() {
            return;
        }
        self.editor.checkpoint();
        if let Some(n) = self.editor.selected_node_mut() {
            n.appearance.shadow = !n.appearance.shadow;
        }
    }
    pub fn toggle_stroke(&mut self) {
        if self.word_mode() {
            self.set_status("Object strokes are available in Compose mode", true);
            return;
        }
        self.commit_field();
        if self.selection_locked() {
            return;
        }
        self.editor.checkpoint();
        if let Some(n) = self.editor.selected_node_mut() {
            n.appearance.stroke_width = if n.appearance.stroke_width == 0. {
                1.
            } else {
                0.
            };
        }
    }
    pub fn set_page_size(&mut self, width: f32, height: f32) {
        if self.word_mode()
            && (width <= 2. * document::WORD_MARGIN + 40.
                || height <= 2. * document::WORD_MARGIN + 40.
                || width > 1584.
                || height > 1584.)
        {
            self.set_status("Choose a Word page size that fits 1-inch margins", true);
            return;
        }
        self.commit_field();
        self.end_edit();
        self.editor.checkpoint();
        let p = &mut self.editor.document.pages[self.editor.page];
        p.width = width;
        p.height = height;
        self.fitted = false;
        self.popup = None;
    }
    pub fn toggle_preview(&mut self) {
        self.close_popup();
        self.end_edit();
        self.preview = !self.preview;
        self.fitted = false;
        self.set_status(
            if self.preview {
                "Preview · press Escape to return"
            } else {
                "Ready"
            },
            false,
        );
    }
    pub fn commit_special_field(&mut self, edit: &FieldEdit) -> bool {
        match edit.field {
            Field::LinkUrl => {
                self.link_url = edit.value.clone();
                true
            }
            Field::DocumentName => {
                if !edit.value.trim().is_empty() {
                    self.editor.checkpoint();
                    self.editor.document.name = edit.value.trim().chars().take(160).collect();
                }
                true
            }
            Field::ImageSearch => {
                self.image_picker.query = edit.value.clone();
                true
            }
            Field::FontSearch => {
                self.font_search = edit.value.clone();
                true
            }
            Field::ColorHex => {
                if let Some(c) = from_hex(&edit.value) {
                    self.choose_color(c)
                } else {
                    self.set_status("Use six-digit RGB or eight-digit RGBA hex", true);
                }
                true
            }
            Field::ColorAlpha => {
                if let Ok(v) = edit.value.trim().parse::<f32>()
                    && v.is_finite()
                    && (0.0..=100.).contains(&v)
                {
                    if let Some(p) = &mut self.picker {
                        p.hsv.alpha = (v * 2.55).round() as u8;
                        let c = p.hsv.color();
                        self.apply_color(c, true);
                    }
                } else {
                    self.set_status("Alpha must be between 0 and 100", true);
                }
                true
            }
            Field::StrokeWidth | Field::ShadowBlur | Field::ShadowY => {
                if self.word_mode() {
                    self.set_status("Canvas effects are available in Compose mode", true);
                    return true;
                }
                if self.selection_locked() {
                    return true;
                }
                let Ok(v) = edit.value.trim().parse::<f32>() else {
                    self.set_status("Enter a valid number", true);
                    return true;
                };
                if !v.is_finite() || v.abs() > 100. || (v < 0. && edit.field != Field::ShadowY) {
                    self.set_status("Value is outside the allowed range", true);
                    return true;
                }
                self.editor.checkpoint();
                if let Some(n) = self.editor.selected_node_mut() {
                    match edit.field {
                        Field::StrokeWidth => {
                            n.appearance.stroke_width = if matches!(n.kind, NodeKind::Shape)
                                && n.shape == document::ShapeKind::Line
                            {
                                v.max(0.5)
                            } else {
                                v
                            }
                        }
                        Field::ShadowBlur => n.appearance.shadow_blur = v,
                        _ => n.appearance.shadow_y = v,
                    }
                }
                true
            }
            _ => false,
        }
    }
}
