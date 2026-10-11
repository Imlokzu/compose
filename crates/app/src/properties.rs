use crate::workspace::{Field, Workspace};
use document::{ImageSizing, NodeKind, Rect};
use text::TextStyle;

impl Workspace {
    pub fn object_ratio(&self) -> Option<f32> {
        if self.editor.selected_node().is_some_and(|n| {
            matches!(n.kind, NodeKind::Shape) && n.shape == document::ShapeKind::Circle
        }) {
            Some(1.)
        } else if self.image_scales() {
            self.image_ratio()
        } else {
            None
        }
    }
    pub fn supports_radius(&self) -> bool {
        self.editor.selected_node().is_some_and(|n| {
            !matches!(n.kind, NodeKind::Text(_))
                && (!matches!(n.kind, NodeKind::Shape) || n.shape.supports_radius())
        })
    }
    pub fn image_dimensions(&self) -> Option<(f32, f32)> {
        let NodeKind::Image { asset } = &self.editor.selected_node()?.kind else {
            return None;
        };
        if let Some(image) = self.images.get(asset) {
            let size = image.size(0);
            return Some((size.width.0 as f32, size.height.0 as f32));
        }
        let bytes = self.editor.document.assets.get(asset)?;
        let (w, h) = image::ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
            .with_guessed_format()
            .ok()?
            .into_dimensions()
            .ok()?;
        Some((w as f32, h as f32))
    }
    pub fn image_ratio(&self) -> Option<f32> {
        self.image_dimensions().map(|(w, h)| w / h)
    }
    pub fn image_scales(&self) -> bool {
        self.editor.selected_node().is_some_and(|n| {
            matches!(n.kind, NodeKind::Image { .. }) && n.image_sizing == ImageSizing::Scale
        })
    }
    pub fn set_image_sizing(&mut self, mode: ImageSizing) {
        if self.word_mode() && mode == ImageSizing::Stretch {
            self.set_status("Image stretching is available in Compose mode", true);
            return;
        }
        self.commit_field();
        let Some(ratio) = self.image_ratio() else {
            return;
        };
        let n = self.editor.selected_node().unwrap();
        if n.image_sizing == mode
            && (mode == ImageSizing::Stretch
                || (n.frame.width / n.frame.height - ratio).abs() < 0.0001)
        {
            return;
        }
        self.editor.checkpoint();
        let n = self.editor.selected_node_mut().unwrap();
        n.image_sizing = mode;
        if mode == ImageSizing::Scale {
            let h = n.frame.width / ratio;
            n.frame.y += (n.frame.height - h) / 2.;
            n.frame.height = h;
        }
        self.set_status(
            match mode {
                ImageSizing::Scale => "Scale · image proportions are locked",
                ImageSizing::Stretch => "Stretch · width and height are independent",
            },
            false,
        );
    }
    pub fn scale_image(&mut self, factor: f32) {
        self.commit_field();
        if self.image_ratio().is_none() || !factor.is_finite() || factor <= 0. {
            return;
        }
        let frame = self.editor.selected_node().unwrap().frame;
        let w = frame.width * factor;
        let h = if self.image_scales() {
            w / self.image_ratio().unwrap()
        } else {
            frame.height * factor
        };
        if w < 1. || h < 1. || w > 10000. || h > 10000. {
            self.set_status("Image reached the size limit", true);
            return;
        }
        self.editor.checkpoint();
        let n = self.editor.selected_node_mut().unwrap();
        n.frame = Rect::new(
            frame.x + (frame.width - w) / 2.,
            frame.y + (frame.height - h) / 2.,
            w,
            h,
        );
        n.radius *= factor;
        self.set_status("Image scaled", false);
    }
    pub fn set_radius(&mut self, radius: f32) {
        if self.word_mode() {
            self.set_status("Image corners are available in Compose mode", true);
            return;
        }
        self.commit_field();
        let Some(n) = self.editor.selected_node() else {
            return;
        };
        if !self.supports_radius() {
            return;
        }
        let radius = radius.clamp(0., n.frame.width.min(n.frame.height) / 2.);
        if (n.radius - radius).abs() < 0.001 {
            return;
        }
        self.editor.checkpoint();
        self.editor.selected_node_mut().unwrap().radius = radius;
        self.set_status("Corners updated", false);
    }
    pub fn formatting_scope(&self) -> String {
        if self.editing && !self.range().is_empty() {
            let count = self
                .text()
                .map(|t| t.content[self.range()].chars().count())
                .unwrap_or(0);
            format!("Selected text · {count} characters")
        } else if self.editing {
            "New text at cursor".into()
        } else {
            "Entire text box".into()
        }
    }
    pub fn format_values(&self) -> Vec<TextStyle> {
        let Some(t) = self.text() else {
            return vec![TextStyle::default()];
        };
        if self.editing && self.range().is_empty() {
            return vec![self.active_style()];
        }
        let range = if self.editing {
            self.range()
        } else {
            0..t.content.len()
        };
        let mut styles = vec![t.style_at(range.start)];
        for span in &t.spans {
            if span.range.start < range.end && span.range.end > range.start {
                styles.push(span.style.clone());
            }
            if span.range.end > range.start && span.range.end < range.end {
                styles.push(t.style_at(span.range.end));
            }
        }
        styles
    }
    pub fn style_value(&self, field: Field) -> String {
        let styles = self.format_values();
        let first = &styles[0];
        let read = |s: &TextStyle| match field {
            Field::Size => s.size,
            Field::LineHeight => s.line_height,
            Field::Spacing => s.letter_spacing,
            _ => 0.,
        };
        let v = read(first);
        if styles.iter().any(|s| (read(s) - v).abs() > 0.001) {
            return "Mixed".into();
        }
        match field {
            Field::Size => {
                if v.fract().abs() < 0.01 {
                    format!("{v:.0}")
                } else {
                    format!("{v:.1}")
                }
            }
            Field::LineHeight => format!("{v:.2}"),
            _ => format!("{v:.1}"),
        }
    }
    pub fn selection_toolbar_position(&self, width: f32, height: f32) -> Option<(f32, f32)> {
        if !self.fitted {
            return None;
        }
        let n = self.absolute_frame(self.editor.selected()?)?;
        let l = self.layout()?;
        let geometry = if self.range().is_empty() {
            l.caret(self.caret)
        } else {
            let selection = l.selection(self.range());
            let first = *selection.first()?;
            selection.iter().skip(1).fold(first, |bounds, r| {
                let x = bounds.x.min(r.x);
                let y = bounds.y.min(r.y);
                Rect::new(
                    x,
                    y,
                    (bounds.x + bounds.width).max(r.x + r.width) - x,
                    (bounds.y + bounds.height).max(r.y + r.height) - y,
                )
            })
        };
        // Project from the current camera: painting may still hold the previous
        // frame's page origin immediately after a pan/zoom event.
        let vw = f32::from(self.viewport.size.width);
        let vh = f32::from(self.viewport.size.height);
        let page = &self.editor.document.pages[self.editor.page];
        let left_px =
            (vw - page.width * self.zoom) / 2. + self.pan.x + (n.x + geometry.x) * self.zoom;
        let top_px = 48. + self.pan.y + (n.y + geometry.y) * self.zoom;
        let right_px = left_px + geometry.width * self.zoom;
        let bottom_px = top_px + geometry.height * self.zoom;
        let left = left_px.max(0.);
        let right = right_px.min(vw);
        let top = top_px.max(0.);
        let bottom = bottom_px.min(vh);
        if right < left || bottom < top {
            return None;
        }
        let x = ((left + right - width) / 2.).clamp(8., (vw - width - 8.).max(8.));
        let above = top - height - 10.;
        let y = if above >= 8. { above } else { bottom + 10. };
        Some((x, y.clamp(8., (vh - height - 8.).max(8.))))
    }
}
