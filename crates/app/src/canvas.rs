use crate::workspace::{Gesture, Tool, Workspace};
use document::{Node, NodeKind, Point as DocPoint, Rect};
use gpui::*;
impl Workspace {
    pub fn to_doc(&self, p: Point<Pixels>) -> DocPoint {
        DocPoint {
            x: f32::from(p.x - self.page_origin.x) / self.zoom,
            y: f32::from(p.y - self.page_origin.y) / self.zoom,
        }
    }
    pub fn screen_rect(&self, r: Rect) -> Bounds<Pixels> {
        Bounds::new(
            point(
                self.page_origin.x + px(r.x * self.zoom),
                self.page_origin.y + px(r.y * self.zoom),
            ),
            size(px(r.width * self.zoom), px(r.height * self.zoom)),
        )
    }
    pub fn absolute_frame(&self, id: u64) -> Option<Rect> {
        self.editor.absolute_frame(id)
    }
    pub fn handle_points(r: Rect) -> [(f32, f32); 8] {
        let (x, y, w, h) = (r.x, r.y, r.width, r.height);
        [
            (x, y),
            (x + w / 2., y),
            (x + w, y),
            (x + w, y + h / 2.),
            (x + w, y + h),
            (x + w / 2., y + h),
            (x, y + h),
            (x, y + h / 2.),
        ]
    }
    pub fn paint_canvas(
        &mut self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.viewport != bounds {
            cx.notify();
        }
        self.viewport = bounds;
        let entity = cx.entity().downgrade();
        window.on_mouse_event(move |event: &MagnifyEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble {
                entity.update(cx, |s, cx| s.magnify(event, window, cx)).ok();
            }
        });
        if !self.fitted {
            self.fit();
            cx.notify();
        }
        self.measure();
        let p = &self.editor.document.pages[self.editor.page];
        let (page_w, page_h) = (p.width, p.height);
        self.page_origin = point(
            bounds.left() + (bounds.size.width - px(page_w * self.zoom)) / 2. + px(self.pan.x),
            bounds.top() + px(layout::PAGE_TOP + self.pan.y),
        );
        let page = self.screen_rect(Rect::new(0., 0., page_w, page_h));
        window.paint_shadows(
            page,
            px(2.).into(),
            &[BoxShadow {
                color: rgba(0x20282015).into(),
                offset: point(px(0.), px(3.)),
                blur_radius: px(14.),
                spread_radius: px(0.),
            }],
        );
        window.paint_quad(fill(page, rgb(0xfffefb)).corner_radii(px(2.)));
        let label = format!(
            "{}   ·   {:.0} × {:.0} pt",
            self.editor.document.pages[self.editor.page].name, page_w, page_h
        );
        let label_len = label.len();
        let _ = window
            .text_system()
            .shape_line(
                label.into(),
                px(11.),
                &[TextRun {
                    len: label_len,
                    font: font(".SystemUIFont"),
                    color: rgb(0x777c76).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            )
            .paint(
                point(page.left(), page.top() - px(24.)),
                px(16.),
                window,
                cx,
            );
        let nodes = self.editor.document.pages[self.editor.page].nodes.clone();
        window.with_content_mask(Some(ContentMask { bounds: page }), |window| {
            self.paint_nodes(&nodes, 0., 0., 1., window, cx);
        });
        if !self.preview
            && let Some(r) = self.editor.selection_bounds()
        {
            if self.editor.selection_count() > 1 {
                for (_, frame) in self.editor.selection_frames() {
                    window.paint_quad(outline(
                        self.screen_rect(frame),
                        rgba(0x2878ed75),
                        BorderStyle::Solid,
                    ));
                }
            }
            let b = self.screen_rect(r);
            window.paint_quad(outline(b, rgb(0x2878ed), BorderStyle::Solid));
            if !self.editing && !self.selection_locked() && self.resizable_selection() {
                for (x, y) in Self::handle_points(r) {
                    let p = self.screen_rect(Rect::new(x, y, 0., 0.)).origin;
                    window.paint_quad(
                        fill(
                            Bounds::new(p - point(px(3.), px(3.)), size(px(6.), px(6.))),
                            white(),
                        )
                        .border_widths(px(1.))
                        .border_color(rgb(0x2878ed)),
                    );
                }
            }
        }
        if let Gesture::Marquee { start, current, .. } = self.gesture {
            let r = Rect::new(
                start.x.min(current.x),
                start.y.min(current.y),
                (current.x - start.x).abs().max(0.01),
                (current.y - start.y).abs().max(0.01),
            );
            window.paint_quad(
                fill(self.screen_rect(r), rgba(0x2878ed15))
                    .border_widths(px(1.))
                    .border_color(rgb(0x2878ed)),
            );
        }
        if !self.preview
            && !self.editing
            && !self.selection_locked()
            && self.resizable_selection()
            && let Some(id) = self.editor.selected()
            && let Some(r) = self.absolute_frame(id)
            && let Some(h) = self.radius_handle(r)
        {
            let center = self.screen_rect(Rect::new(h.x, h.y, 0., 0.)).origin;
            window.paint_quad(
                fill(
                    Bounds::new(center - point(px(4.), px(4.)), size(px(8.), px(8.))),
                    white(),
                )
                .corner_radii(px(4.))
                .border_widths(px(1.5))
                .border_color(rgb(0x2878ed)),
            );
        }
        for guide in &self.guides {
            let r = if guide.vertical {
                Rect::new(guide.position, 0., 1. / self.zoom, page_h)
            } else {
                Rect::new(0., guide.position, page_w, 1. / self.zoom)
            };
            window.paint_quad(fill(self.screen_rect(r), rgba(0xea749bc0)));
        }
        window.handle_input(
            &self.focus,
            ElementInputHandler::new(
                if self.field.is_some() {
                    self.field_bounds
                } else {
                    bounds
                },
                cx.entity(),
            ),
            cx,
        );
    }
    fn paint_nodes(
        &mut self,
        nodes: &[Node],
        x: f32,
        y: f32,
        opacity: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for n in nodes {
            if !n.visible {
                continue;
            }
            let r = Rect::new(x + n.frame.x, y + n.frame.y, n.frame.width, n.frame.height);
            let b = self.screen_rect(r);
            let opacity = opacity * n.opacity;
            let corners = px(n.radius.min(r.width.min(r.height) / 2.) * self.zoom);
            if n.appearance.shadow && !matches!(n.kind, NodeKind::Text(_)) {
                window.paint_shadows(
                    b,
                    corners.into(),
                    &[BoxShadow {
                        color: hsla(0., 0., 0., 0.16 * opacity),
                        offset: point(px(0.), px(n.appearance.shadow_y * self.zoom)),
                        blur_radius: px(n.appearance.shadow_blur * self.zoom),
                        spread_radius: px(0.),
                    }],
                );
            }
            match &n.kind {
                NodeKind::Shape | NodeKind::Frame => {
                    let mut color = crate::panels::color(n.fill);
                    color.a *= opacity;
                    window
                        .paint_quad(fill(b, color).corner_radii(px(
                            n.radius.min(r.width.min(r.height) / 2.) * self.zoom,
                        )));
                }
                NodeKind::Image { asset } => {
                    if !self.images.contains_key(asset)
                        && let Some(bytes) = self.editor.document.assets.get(asset)
                    {
                        match Self::decode_image(bytes) {
                            Ok((image, _, _)) => {
                                self.images.insert(asset.clone(), image);
                            }
                            Err(e) => self.set_status(format!("Image unavailable: {e}"), true),
                        }
                    }
                    if let Some(image) = self.images.get(asset) {
                        let source = image.size(0);
                        let picture = layout::image_bounds(
                            r,
                            source.width.0 as f32,
                            source.height.0 as f32,
                            n.image_sizing,
                        );
                        let b = self.screen_rect(picture);
                        window.with_element_opacity(Some(opacity), |window| {
                            if let Err(e) = window.paint_image(
                                b,
                                px(n.radius.min(picture.width.min(picture.height) / 2.)
                                    * self.zoom)
                                .into(),
                                image.clone(),
                                0,
                                false,
                            ) {
                                eprintln!("Image paint: {e}");
                            }
                        });
                    }
                }
                NodeKind::Text(_) => {
                    if let Some((_, _, l)) = self.layouts.get(&n.id) {
                        window.with_content_mask(Some(ContentMask { bounds: b }), |window| {
                            if self.editing && self.editor.selected() == Some(n.id) {
                                for sr in l.selection(self.range()) {
                                    let sr = Rect::new(r.x + sr.x, r.y + sr.y, sr.width, sr.height);
                                    window.paint_quad(fill(self.screen_rect(sr), rgba(0x2878ed35)));
                                }
                            }
                            l.paint(b.origin, self.zoom, opacity, window, cx);
                            if self.editing && self.editor.selected() == Some(n.id) {
                                if let Some(marked) = &self.marked {
                                    for sr in l.selection(marked.clone()) {
                                        window.paint_quad(fill(
                                            self.screen_rect(Rect::new(
                                                r.x + sr.x,
                                                r.y + sr.y + sr.height - 1.,
                                                sr.width,
                                                1.,
                                            )),
                                            rgb(0x2878ed),
                                        ));
                                    }
                                }
                                if self.range().is_empty()
                                    && self.caret_visible
                                    && self.field.is_none()
                                {
                                    let c = l.caret(self.caret);
                                    window.paint_quad(fill(
                                        self.screen_rect(Rect::new(
                                            r.x + c.x,
                                            r.y + c.y,
                                            1.3 / self.zoom,
                                            c.height,
                                        )),
                                        rgb(0x2878ed),
                                    ));
                                }
                            }
                        });
                    }
                }
            }
            if n.appearance.stroke_width > 0. && !matches!(n.kind, NodeKind::Text(_)) {
                let mut stroke = crate::panels::color(n.appearance.stroke);
                stroke.a *= opacity;
                window.paint_quad(
                    outline(b, stroke, BorderStyle::Solid)
                        .corner_radii(corners)
                        .border_widths(px(n.appearance.stroke_width * self.zoom)),
                );
            }
            self.paint_nodes(&n.children, r.x, r.y, opacity, window, cx);
        }
    }
    pub fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending.is_some() || self.preview {
            return;
        }
        self.close_popup();
        self.commit_field();
        self.focus.focus(window);
        self.typing = None;
        self.caret_visible = true;
        if self.space_held || event.button == MouseButton::Middle {
            self.gesture = Gesture::Pan {
                start: event.position,
                pan: self.pan,
            };
            return;
        }
        let p = self.to_doc(event.position);
        if !self.preview
            && !self.editing
            && !self.selection_locked()
            && self.resizable_selection()
            && let Some(id) = self.editor.selected()
            && let Some(r) = self.absolute_frame(id)
        {
            if let Some(h) = self.radius_handle(r)
                && (p.x - h.x).abs() < 7. / self.zoom
                && (p.y - h.y).abs() < 7. / self.zoom
            {
                self.gesture = Gesture::Radius {
                    start: p,
                    radius: self.editor.selected_node().unwrap().radius,
                    started: false,
                };
                cx.notify();
                return;
            }
            for (i, (x, y)) in Self::handle_points(r).iter().enumerate() {
                if (p.x - x).abs() < 6. / self.zoom && (p.y - y).abs() < 6. / self.zoom {
                    self.gesture = Gesture::Resize {
                        start: p,
                        frame: r,
                        handle: i,
                        started: false,
                    };
                    cx.notify();
                    return;
                }
            }
        }
        let page = &self.editor.document.pages[self.editor.page];
        if !Rect::new(0., 0., page.width, page.height).contains(p) {
            self.end_edit();
            let base = if event.modifiers.shift {
                self.editor.selection_ids().to_vec()
            } else {
                vec![]
            };
            self.editor.set_selection(base.clone());
            if self.tool == Tool::Select {
                self.gesture = Gesture::Marquee {
                    start: p,
                    current: p,
                    base,
                };
            }
            cx.notify();
            return;
        }
        let editing_group = self
            .editor
            .selected()
            .and_then(|id| self.editor.parent_of(id))
            .filter(|id| self.editor.document.node(*id).is_some_and(|n| n.is_group))
            .and_then(|id| self.absolute_frame(id))
            .is_some_and(|r| r.contains(p));
        let hit =
            if event.click_count >= 2 || self.editing || self.tool == Tool::Text || editing_group {
                self.editor.hit_test_deep(p)
            } else {
                self.editor.hit_test(p)
            };
        let text_target = hit.is_some_and(|id| {
            self.editor
                .document
                .node(id)
                .is_some_and(|n| matches!(n.kind, NodeKind::Text(_)))
        });
        if self.tool != Tool::Select && !(self.tool == Tool::Text && text_target) {
            self.insert_at(self.tool, p);
            self.measure();
            cx.notify();
            return;
        }
        if let Some(id) = hit {
            let text = matches!(
                self.editor.document.node(id).unwrap().kind,
                NodeKind::Text(_)
            );
            let already = self.editor.selected() == Some(id) && self.editing;
            if self.tool == Tool::Select
                && event.modifiers.shift
                && !already
                && event.click_count == 1
            {
                self.end_edit();
                self.editor.toggle_selected(id);
                self.gesture = Gesture::None;
                cx.notify();
                return;
            }
            if text && (event.click_count >= 2 || already || self.tool == Tool::Text) {
                self.editor.select_only(Some(id));
                self.editing = true;
                self.insertion_style = None;
                self.marked = None;
                self.measure();
                let r = self.absolute_frame(id).unwrap();
                let l = self.layout().unwrap();
                let index = l.hit_test(p.x - r.x, p.y - r.y);
                if event.click_count >= 3 {
                    self.anchor = 0;
                    self.caret = self.text().unwrap().content.len();
                } else if event.click_count == 2 {
                    let range = l.word_at(p.x - r.x, p.y - r.y);
                    self.anchor = range.start;
                    self.caret = range.end;
                } else {
                    if !event.modifiers.shift || !already {
                        self.anchor = index;
                    }
                    self.caret = index;
                }
                self.gesture = Gesture::Text;
            } else {
                self.end_edit();
                let collapse = (self.editor.selection_count() > 1 && self.editor.is_selected(id))
                    .then_some(id);
                if !self.editor.is_selected(id) || event.click_count >= 2 {
                    self.editor.select_only(Some(id));
                }
                if self.selection_locked() {
                    cx.notify();
                    return;
                }
                let frame = self.editor.selection_bounds().unwrap();
                self.gesture = Gesture::Move {
                    start: p,
                    frame,
                    frames: self.editor.selection_frames(),
                    collapse,
                    started: false,
                };
            }
        } else {
            self.end_edit();
            if event.click_count >= 2 {
                self.editor.select_only(None);
                self.insert_at(Tool::Text, p);
                self.measure();
            } else {
                let base = if event.modifiers.shift {
                    self.editor.selection_ids().to_vec()
                } else {
                    vec![]
                };
                self.editor.set_selection(base.clone());
                self.gesture = Gesture::Marquee {
                    start: p,
                    current: p,
                    base,
                };
            }
        }
        cx.notify();
    }
    pub fn mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cursor = self.canvas_cursor();
        self.last_pointer = Some(event.position);
        if self.picker.as_ref().is_some_and(|p| p.drag.is_some()) {
            self.update_color_drag(event.position);
            cx.notify();
            return;
        }
        if cursor != self.canvas_cursor() {
            cx.notify();
        }
        let p = self.to_doc(event.position);
        let gesture = std::mem::replace(&mut self.gesture, Gesture::None);
        self.gesture = match gesture {
            Gesture::Pan { start, pan } => {
                self.pan = DocPoint {
                    x: pan.x + f32::from(event.position.x - start.x),
                    y: pan.y + f32::from(event.position.y - start.y),
                };
                Gesture::Pan { start, pan }
            }
            Gesture::Text => {
                if let Some(id) = self.editor.selected()
                    && let Some(r) = self.absolute_frame(id)
                    && let Some(l) = self.layout()
                {
                    self.caret = l.hit_test(p.x - r.x, p.y - r.y);
                    self.caret_visible = true;
                }
                Gesture::Text
            }
            Gesture::Move {
                start,
                frame,
                frames,
                collapse,
                mut started,
            } => {
                let (mut dx, mut dy) = (p.x - start.x, p.y - start.y);
                if started || dx.abs() + dy.abs() > 2. / self.zoom {
                    if !started {
                        self.editor.checkpoint();
                        started = true
                    }
                    if event.modifiers.shift {
                        if dx.abs() > dy.abs() {
                            dy = 0.
                        } else {
                            dx = 0.
                        }
                    }
                    let mut r = Rect::new(frame.x + dx, frame.y + dy, frame.width, frame.height);
                    let page = &self.editor.document.pages[self.editor.page];
                    let others = self.editor.snap_bounds();
                    if !event.modifiers.alt {
                        let (snapped, guides) =
                            layout::snap_rect(r, &others, page.width, page.height, 5. / self.zoom);
                        r = snapped;
                        self.guides = guides;
                    } else {
                        self.guides.clear()
                    }
                    let delta = DocPoint {
                        x: r.x - frame.x,
                        y: r.y - frame.y,
                    };
                    let moved: Vec<_> = frames
                        .iter()
                        .map(|(id, original)| {
                            (
                                *id,
                                Rect::new(
                                    original.x + delta.x,
                                    original.y + delta.y,
                                    original.width,
                                    original.height,
                                ),
                            )
                        })
                        .collect();
                    self.editor.apply_frames(&moved);
                }
                Gesture::Move {
                    start,
                    frame,
                    frames,
                    collapse,
                    started,
                }
            }
            Gesture::Resize {
                start,
                frame,
                handle,
                mut started,
            } => {
                let (dx, dy) = (p.x - start.x, p.y - start.y);
                if started || dx.abs() + dy.abs() > 1. / self.zoom {
                    if !started {
                        self.editor.checkpoint();
                        started = true
                    }
                    let ratio = if self.image_scales() {
                        self.image_ratio()
                    } else if event.modifiers.shift {
                        Some(frame.width / frame.height)
                    } else {
                        None
                    };
                    let r = layout::resize_rect(
                        frame,
                        handle,
                        DocPoint { x: dx, y: dy },
                        ratio,
                        event.modifiers.alt,
                    );
                    if let Some(id) = self.editor.selected() {
                        self.editor.apply_frames(&[(id, r)]);
                    }
                }
                Gesture::Resize {
                    start,
                    frame,
                    handle,
                    started,
                }
            }
            Gesture::Radius {
                start,
                radius,
                mut started,
            } => {
                let delta = (p.x - start.x) + (p.y - start.y);
                if started || delta.abs() > 1. / self.zoom {
                    if !started {
                        self.editor.checkpoint();
                        started = true;
                    }
                    if let Some(n) = self.editor.selected_node_mut() {
                        n.radius =
                            (radius + delta).clamp(0., n.frame.width.min(n.frame.height) / 2.);
                    }
                }
                Gesture::Radius {
                    start,
                    radius,
                    started,
                }
            }
            Gesture::Marquee { start, base, .. } => {
                let r = Rect::new(
                    start.x.min(p.x),
                    start.y.min(p.y),
                    (p.x - start.x).abs().max(0.01),
                    (p.y - start.y).abs().max(0.01),
                );
                if r.width + r.height > 2. / self.zoom {
                    let mut ids = base.clone();
                    ids.extend(self.editor.marquee_ids(r));
                    self.editor.set_selection(ids);
                }
                Gesture::Marquee {
                    start,
                    current: p,
                    base,
                }
            }
            Gesture::None => return,
        };
        cx.notify();
    }
    pub fn mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(p) = &mut self.picker {
            p.drag = None;
        }
        if let Gesture::Move {
            started: false,
            collapse: Some(id),
            ..
        } = self.gesture
        {
            self.editor.select_only(Some(id));
        }
        self.gesture = Gesture::None;
        self.guides.clear();
        cx.notify();
    }
    pub fn scroll(&mut self, e: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            return;
        }
        if self.pinch.is_some() {
            return;
        }
        let d = e.delta.pixel_delta(px(24.));
        if e.modifiers.control || e.modifiers.platform {
            let factor = (f32::from(d.y) * 0.006).exp();
            if factor.is_finite() {
                self.zoom_at_pointer(e.position, self.zoom * factor);
            }
        } else {
            self.pan.x += f32::from(d.x);
            self.pan.y += f32::from(d.y);
        }
        cx.notify();
    }
}

impl Workspace {
    pub fn radius_handle(&self, r: Rect) -> Option<DocPoint> {
        let n = self.editor.selected_node()?;
        if n.is_group
            || matches!(n.kind, NodeKind::Text(_))
            || r.width.min(r.height) * self.zoom < 48.
        {
            return None;
        }
        let inset = (14. / self.zoom + n.radius.min(r.width.min(r.height) / 2.) * 0.5)
            .min(r.width.min(r.height) / 2.);
        Some(DocPoint {
            x: r.x + inset,
            y: r.y + inset,
        })
    }
}

impl Workspace {
    pub fn resizable_selection(&self) -> bool {
        self.editor.selected_node().is_some_and(|n| !n.is_group)
    }
    pub fn canvas_cursor(&self) -> CursorStyle {
        fn resize(handle: usize) -> CursorStyle {
            match handle {
                0 | 4 => CursorStyle::ResizeUpLeftDownRight,
                2 | 6 => CursorStyle::ResizeUpRightDownLeft,
                1 | 5 => CursorStyle::ResizeUpDown,
                _ => CursorStyle::ResizeLeftRight,
            }
        }
        match self.gesture {
            Gesture::Pan { .. } | Gesture::Move { started: true, .. } => {
                return CursorStyle::ClosedHand;
            }
            Gesture::Resize { handle, .. } => return resize(handle),
            Gesture::Radius { .. } => return CursorStyle::Crosshair,
            _ => {}
        }
        if self.space_held {
            return CursorStyle::OpenHand;
        }
        if !self.editing
            && self.resizable_selection()
            && let Some(pointer) = self.last_pointer
            && let Some(id) = self.editor.selected()
            && let Some(r) = self.absolute_frame(id)
        {
            let p = self.to_doc(pointer);
            if let Some(h) = self.radius_handle(r)
                && (p.x - h.x).abs() < 7. / self.zoom
                && (p.y - h.y).abs() < 7. / self.zoom
            {
                return CursorStyle::Crosshair;
            }
            for (i, (x, y)) in Self::handle_points(r).iter().enumerate() {
                if (p.x - x).abs() < 6. / self.zoom && (p.y - y).abs() < 6. / self.zoom {
                    return resize(i);
                }
            }
        }
        if self.editing || self.tool == Tool::Text {
            CursorStyle::IBeam
        } else if self.tool == Tool::Shape || self.tool == Tool::Frame {
            CursorStyle::Crosshair
        } else {
            CursorStyle::Arrow
        }
    }
}
