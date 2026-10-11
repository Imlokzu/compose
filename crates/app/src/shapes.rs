use crate::{
    panels::label,
    workspace::{Field, Popup, Tool, Workspace},
};
use document::{NodeKind, PathCommand, Rect, ShapeKind};
use gpui::{prelude::*, *};

pub fn paint_path(window: &mut Window, commands: &[PathCommand], color: Hsla, width: Option<f32>) {
    let mut path = width
        .map(|w| PathBuilder::stroke(px(w)))
        .unwrap_or_else(PathBuilder::fill);
    let pt = |p: document::Point| point(px(p.x), px(p.y));
    for command in commands {
        match *command {
            PathCommand::Move(p) => path.move_to(pt(p)),
            PathCommand::Line(p) => path.line_to(pt(p)),
            PathCommand::Quad(c, p) => path.curve_to(pt(p), pt(c)),
            PathCommand::Cubic(a, b, p) => path.cubic_bezier_to(pt(p), pt(a), pt(b)),
            PathCommand::Close => path.close(),
        }
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}
pub fn control(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Rectangle => "shape-rectangle",
        ShapeKind::Circle => "shape-circle",
        ShapeKind::Ellipse => "shape-oval",
        ShapeKind::Triangle => "shape-triangle",
        ShapeKind::Diamond => "shape-diamond",
        ShapeKind::Pentagon => "shape-pentagon",
        ShapeKind::Hexagon => "shape-hexagon",
        ShapeKind::Star => "shape-star",
        ShapeKind::Arrow => "shape-arrow",
        ShapeKind::Line => "shape-line",
    }
}
impl Workspace {
    pub fn open_shapes(&mut self, change: bool) {
        if self.word_mode() || self.preview {
            return;
        }
        self.end_edit();
        self.toggle_popup(Popup::Shapes);
        if self.popup == Some(Popup::Shapes) {
            self.shape_change_target = if change { self.editor.selected() } else { None };
        }
    }
    pub fn choose_shape(&mut self, kind: ShapeKind) {
        let target = self.shape_change_target;
        self.close_popup();
        if self.word_mode() || self.preview {
            return;
        }
        if let Some(id) = target {
            if self.editor.is_locked(id)
                || self.editor.selected() != Some(id)
                || self
                    .editor
                    .document
                    .node(id)
                    .is_none_or(|n| !matches!(n.kind, NodeKind::Shape) || n.shape == kind)
            {
                return;
            }
            self.editor.checkpoint();
            let node = self.editor.document.node_mut(id).unwrap();
            let auto_name = node.name == node.shape.label();
            node.shape = kind;
            if auto_name {
                node.name = kind.label().into();
            }
            if kind == ShapeKind::Circle {
                let side = node.frame.width.min(node.frame.height);
                node.frame.x += (node.frame.width - side) / 2.;
                node.frame.y += (node.frame.height - side) / 2.;
                node.frame.width = side;
                node.frame.height = side;
            }
            if kind == ShapeKind::Line && node.appearance.stroke_width < 0.5 {
                node.appearance.stroke_width = 2.;
            }
            self.set_status(format!("Changed to {}", kind.label()), false);
        } else {
            self.shape_kind = kind;
            self.tool = Tool::Shape;
            self.set_status(
                format!("{} · click or drag on the page", kind.label()),
                false,
            );
        }
    }
    pub fn shape_menu(&self, cx: &Context<Self>) -> Div {
        let changing = self.shape_change_target.is_some();
        let active = if changing {
            self.editor
                .selected_node()
                .map(|n| n.shape)
                .unwrap_or_default()
        } else {
            self.shape_kind
        };
        let mut grid = div().flex().flex_wrap().gap(px(6.)).mt(px(10.));
        for kind in ShapeKind::ALL {
            let id = control(kind);
            let button = self
                .button(
                    id,
                    kind.label(),
                    Some(kind.icon()),
                    active == kind,
                    false,
                    cx,
                    move |s, _, _| s.choose_shape(kind),
                )
                .w(px(134.))
                .h(px(38.))
                .justify_start();
            let button = if changing {
                button
            } else {
                self.draggable_insert(button, crate::drag_drop::InsertKind::Shape(kind), cx)
            };
            grid = grid.child(self.capture_control(id, button, cx));
        }
        div().child(grid).child(
            label(if changing {
                "Change the figure; keep its color and style."
            } else {
                "Choose and draw on the page, or drag a figure onto it."
            })
            .mt(px(12.))
            .line_height(px(16.)),
        )
    }
    pub fn shape_properties(&self, cx: &Context<Self>) -> Div {
        let Some(node) = self.editor.selected_node() else {
            return div();
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                self.capture_control(
                    "shape-type",
                    self.button(
                        "shape-type",
                        node.shape.label(),
                        Some(node.shape.icon()),
                        self.popup == Some(Popup::Shapes),
                        self.selection_locked(),
                        cx,
                        |s, _, _| s.open_shapes(true),
                    )
                    .w_full()
                    .justify_between(),
                    cx,
                ),
            )
            .when(node.shape == ShapeKind::Circle, |d| {
                d.child(label("Circle · width and height stay equal."))
            })
            .when(node.shape == ShapeKind::Line, |d| {
                d.child(self.field_view(
                    Field::StrokeWidth,
                    "Thickness",
                    format!("{:.1}", node.appearance.stroke_width),
                    cx,
                ))
            })
    }
    pub fn paint_custom_shape(
        &self,
        node: &document::Node,
        r: Rect,
        opacity: f32,
        window: &mut Window,
    ) {
        let screen = self.screen_rect(r);
        let r = Rect::new(
            f32::from(screen.origin.x),
            f32::from(screen.origin.y),
            f32::from(screen.size.width),
            f32::from(screen.size.height),
        );
        let commands = node
            .shape
            .path(r, node.radius * self.zoom, node.line_flipped);
        let mut fill = crate::panels::color(node.fill);
        fill.a *= opacity;
        let line = node.shape == ShapeKind::Line;
        paint_path(
            window,
            &commands,
            fill,
            line.then_some(node.appearance.stroke_width.max(0.5) * self.zoom),
        );
        if !line && node.appearance.stroke_width > 0. {
            let mut stroke = crate::panels::color(node.appearance.stroke);
            stroke.a *= opacity;
            paint_path(
                window,
                &commands,
                stroke,
                Some(node.appearance.stroke_width * self.zoom),
            );
        }
    }
    pub fn paint_shape_shadow(
        &self,
        node: &document::Node,
        r: Rect,
        opacity: f32,
        window: &mut Window,
    ) {
        let blur = node.appearance.shadow_blur / 2.;
        let passes = if blur > 0. { 8 } else { 1 };
        for i in (0..passes).rev() {
            let expansion = blur * i as f32 / passes as f32;
            let shadow = Rect::new(
                r.x - expansion,
                r.y + node.appearance.shadow_y - expansion,
                r.width + 2. * expansion,
                r.height + 2. * expansion,
            );
            let bounds = self.screen_rect(shadow);
            let shape = node.shape.path(
                Rect::new(
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ),
                (node.radius + expansion) * self.zoom,
                node.line_flipped,
            );
            paint_path(
                window,
                &shape,
                hsla(0., 0., 0., 0.16 * opacity / passes as f32),
                (node.shape == ShapeKind::Line).then_some(
                    (node.appearance.stroke_width.max(0.5) + 2. * expansion) * self.zoom,
                ),
            );
        }
    }
}
