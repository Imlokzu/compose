use document::{Point, Rect};

pub const MIN_ZOOM: f32 = 0.15;
pub const MAX_ZOOM: f32 = 4.;
pub const PAGE_TOP: f32 = 48.;

/// Viewport, anchor and pan are logical screen pixels; page width is document points.
/// Derive the origin from current camera state, never a previous painted frame.
pub fn zoom_at(
    zoom: f32,
    pan: Point,
    viewport: Rect,
    page_width: f32,
    anchor: Point,
    target_zoom: f32,
) -> (f32, Point) {
    if ![
        zoom,
        pan.x,
        pan.y,
        viewport.x,
        viewport.y,
        viewport.width,
        viewport.height,
        page_width,
        anchor.x,
        anchor.y,
        target_zoom,
    ]
    .iter()
    .all(|v| v.is_finite())
        || zoom <= 0.
        || page_width <= 0.
    {
        return (zoom, pan);
    }
    let origin = Point {
        x: viewport.x + (viewport.width - page_width * zoom) / 2. + pan.x,
        y: viewport.y + PAGE_TOP + pan.y,
    };
    let document = Point {
        x: (anchor.x - origin.x) / zoom,
        y: (anchor.y - origin.y) / zoom,
    };
    let next = target_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    let pan = Point {
        x: anchor.x - viewport.x - (viewport.width - page_width * next) / 2. - document.x * next,
        y: anchor.y - viewport.y - PAGE_TOP - document.y * next,
    };
    (next, pan)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn document_at(zoom: f32, pan: Point, v: Rect, p: Point) -> Point {
        Point {
            x: (p.x - v.x - (v.width - 595.28 * zoom) / 2. - pan.x) / zoom,
            y: (p.y - v.y - PAGE_TOP - pan.y) / zoom,
        }
    }
    #[test]
    fn repeated_events_before_redraw_keep_the_same_document_anchor() {
        let v = Rect::new(220., 49., 600., 700.);
        let p = Point { x: 530., y: 310. };
        let (mut zoom, mut pan) = (0.7, Point { x: 35., y: -80. });
        let doc = document_at(zoom, pan, v, p);
        for next in [0.8, 1.1, 1.7, 0.6, 0.7] {
            (zoom, pan) = zoom_at(zoom, pan, v, 595.28, p, next);
            let got = document_at(zoom, pan, v, p);
            assert!((got.x - doc.x).abs() < 0.001 && (got.y - doc.y).abs() < 0.001);
        }
    }
    #[test]
    fn limits_and_invalid_deltas_never_break_the_camera() {
        let v = Rect::new(0., 0., 600., 700.);
        let p = Point { x: 340., y: 240. };
        let (zoom, pan) = zoom_at(1., Point::default(), v, 595.28, p, 100.);
        assert_eq!(zoom, MAX_ZOOM);
        let (zoom, pan) = zoom_at(zoom, pan, v, 595.28, p, -2.);
        assert_eq!(zoom, MIN_ZOOM);
        assert!(pan.x.is_finite() && pan.y.is_finite());
        assert_eq!(zoom_at(zoom, pan, v, 595.28, p, f32::NAN), (zoom, pan));
    }
}
