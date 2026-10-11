//! Framework-independent figure geometry used by painting, export and hit testing.
use crate::{Point, Rect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Circle,
    Ellipse,
    Triangle,
    Diamond,
    Pentagon,
    Hexagon,
    Star,
    Arrow,
    Line,
}
#[derive(Clone, Copy, Debug)]
pub enum PathCommand {
    Move(Point),
    Line(Point),
    Quad(Point, Point),
    Cubic(Point, Point, Point),
    Close,
}
impl ShapeKind {
    pub const ALL: [Self; 10] = [
        Self::Rectangle,
        Self::Circle,
        Self::Ellipse,
        Self::Triangle,
        Self::Diamond,
        Self::Pentagon,
        Self::Hexagon,
        Self::Star,
        Self::Arrow,
        Self::Line,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Circle => "Circle",
            Self::Ellipse => "Oval",
            Self::Triangle => "Triangle",
            Self::Diamond => "Diamond",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Star => "Star",
            Self::Arrow => "Arrow",
            Self::Line => "Line",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Self::Rectangle => "shape",
            Self::Circle => "circle",
            Self::Ellipse => "oval",
            Self::Triangle => "triangle",
            Self::Diamond => "diamond",
            Self::Pentagon => "pentagon",
            Self::Hexagon => "hexagon",
            Self::Star => "star",
            Self::Arrow => "arrow",
            Self::Line => "line",
        }
    }
    pub fn supports_radius(self) -> bool {
        !matches!(self, Self::Circle | Self::Ellipse | Self::Line)
    }
    pub fn default_size(self) -> (f32, f32) {
        match self {
            Self::Circle | Self::Diamond | Self::Pentagon | Self::Hexagon | Self::Star => {
                (140., 140.)
            }
            Self::Line => (180., 1.),
            Self::Arrow => (180., 90.),
            _ => (160., 120.),
        }
    }
    pub fn path(self, r: Rect, radius: f32, flipped: bool) -> Vec<PathCommand> {
        use PathCommand::*;
        let p = |x: f32, y: f32| Point {
            x: r.x + x * r.width,
            y: r.y + y * r.height,
        };
        if self == Self::Line {
            return vec![
                Move(p(0., if flipped { 1. } else { 0. })),
                Line(p(1., if flipped { 0. } else { 1. })),
            ];
        }
        if matches!(self, Self::Circle | Self::Ellipse) {
            let k = 0.552_284_8 / 2.;
            return vec![
                Move(p(1., 0.5)),
                Cubic(p(1., 0.5 + k), p(0.5 + k, 1.), p(0.5, 1.)),
                Cubic(p(0.5 - k, 1.), p(0., 0.5 + k), p(0., 0.5)),
                Cubic(p(0., 0.5 - k), p(0.5 - k, 0.), p(0.5, 0.)),
                Cubic(p(0.5 + k, 0.), p(1., 0.5 - k), p(1., 0.5)),
                Close,
            ];
        }
        if self == Self::Rectangle && radius > 0. {
            let radius = radius.min(r.width.min(r.height) / 2.);
            let x = radius / r.width;
            let y = radius / r.height;
            let k = 0.552_284_8;
            return vec![
                Move(p(x, 0.)),
                Line(p(1. - x, 0.)),
                Cubic(p(1. - x + x * k, 0.), p(1., y - y * k), p(1., y)),
                Line(p(1., 1. - y)),
                Cubic(p(1., 1. - y + y * k), p(1. - x + x * k, 1.), p(1. - x, 1.)),
                Line(p(x, 1.)),
                Cubic(p(x - x * k, 1.), p(0., 1. - y + y * k), p(0., 1. - y)),
                Line(p(0., y)),
                Cubic(p(0., y - y * k), p(x - x * k, 0.), p(x, 0.)),
                Close,
            ];
        }
        let regular = |count: usize, inner: Option<f32>| -> Vec<Point> {
            let n = if inner.is_some() { count * 2 } else { count };
            let mut pts = (0..n)
                .map(|i| {
                    let a =
                        -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n as f32;
                    let size = if i % 2 == 1 { inner.unwrap_or(1.) } else { 1. };
                    Point {
                        x: a.cos() * size,
                        y: a.sin() * size,
                    }
                })
                .collect::<Vec<_>>();
            let minx = pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
            let maxx = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
            let miny = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
            let maxy = pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
            for pt in &mut pts {
                *pt = p((pt.x - minx) / (maxx - minx), (pt.y - miny) / (maxy - miny));
            }
            pts
        };
        let vertices = match self {
            Self::Rectangle => vec![p(0., 0.), p(1., 0.), p(1., 1.), p(0., 1.)],
            Self::Triangle => vec![p(0.5, 0.), p(1., 1.), p(0., 1.)],
            Self::Diamond => vec![p(0.5, 0.), p(1., 0.5), p(0.5, 1.), p(0., 0.5)],
            Self::Pentagon => regular(5, None),
            Self::Hexagon => regular(6, None),
            Self::Star => regular(5, Some(0.42)),
            Self::Arrow => vec![
                p(0., 0.3),
                p(0.58, 0.3),
                p(0.58, 0.),
                p(1., 0.5),
                p(0.58, 1.),
                p(0.58, 0.7),
                p(0., 0.7),
            ],
            _ => unreachable!(),
        };
        let radius = radius.max(0.).min(r.width.min(r.height) / 2.);
        let mut corners = Vec::new();
        for (i, &v) in vertices.iter().enumerate() {
            let before = vertices[(i + vertices.len() - 1) % vertices.len()];
            let after = vertices[(i + 1) % vertices.len()];
            let distance = |a: Point, b: Point| (a.x - b.x).hypot(a.y - b.y);
            let a = distance(v, before);
            let b = distance(v, after);
            let d = radius.min(a / 2.).min(b / 2.);
            let toward = |to: Point, len: f32| Point {
                x: v.x + (to.x - v.x) * d / len,
                y: v.y + (to.y - v.y) * d / len,
            };
            corners.push((toward(before, a), v, toward(after, b)));
        }
        let mut path = vec![Move(corners[0].0)];
        for (i, &(enter, corner, exit)) in corners.iter().enumerate() {
            if i > 0 {
                path.push(Line(enter));
            }
            if radius > 0. {
                path.push(Quad(corner, exit));
            } else {
                path.push(Line(corner));
            }
        }
        path.push(Close);
        path
    }
    pub fn contains(self, r: Rect, radius: f32, flipped: bool, stroke: f32, point: Point) -> bool {
        if self == Self::Line {
            let path = self.path(r, 0., flipped);
            let (PathCommand::Move(a), PathCommand::Line(b)) = (path[0], path[1]) else {
                unreachable!()
            };
            return segment_distance(point, a, b) <= (stroke / 2.).max(4.);
        }
        if !r.contains(point) {
            return false;
        }
        if self == Self::Rectangle && radius > 0. {
            let radius = radius.min(r.width.min(r.height) / 2.);
            let x = point.x.clamp(r.x + radius, r.x + r.width - radius);
            let y = point.y.clamp(r.y + radius, r.y + r.height - radius);
            return (point.x - x).powi(2) + (point.y - y).powi(2) <= radius * radius + 0.001;
        }
        if matches!(self, Self::Circle | Self::Ellipse) {
            return ((point.x - r.x - r.width / 2.) / (r.width / 2.)).powi(2)
                + ((point.y - r.y - r.height / 2.) / (r.height / 2.)).powi(2)
                <= 1.000_01;
        }
        let mut points = Vec::new();
        let mut from = Point::default();
        for cmd in self.path(r, radius, flipped) {
            match cmd {
                PathCommand::Move(p) | PathCommand::Line(p) => {
                    points.push(p);
                    from = p;
                }
                PathCommand::Quad(c, p) => {
                    let start = from;
                    for step in 1..=12 {
                        let t = step as f32 / 12.;
                        let s = 1. - t;
                        points.push(Point {
                            x: s * s * start.x + 2. * s * t * c.x + t * t * p.x,
                            y: s * s * start.y + 2. * s * t * c.y + t * t * p.y,
                        });
                    }
                    from = p;
                }
                PathCommand::Cubic(..) => unreachable!(),
                PathCommand::Close => {}
            }
        }
        let mut inside = false;
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if segment_distance(point, a, b) < 0.01 {
                return true;
            }
            if (a.y > point.y) != (b.y > point.y)
                && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
            {
                inside = !inside;
            }
        }
        inside
    }
}
fn segment_distance(p: Point, a: Point, b: Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = dx * dx + dy * dy;
    let t = if length > 0. {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / length
    } else {
        0.
    }
    .clamp(0., 1.);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}
