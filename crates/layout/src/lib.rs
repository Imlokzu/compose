mod camera;
mod resize;
pub use camera::{MAX_ZOOM, MIN_ZOOM, PAGE_TOP, zoom_at};
use document::Rect;
pub use resize::{image_bounds, resize_rect};
#[derive(Clone, Copy, Debug)]
pub struct Guide {
    pub vertical: bool,
    pub position: f32,
}
pub fn snap_rect(
    mut frame: Rect,
    others: &[Rect],
    page_width: f32,
    page_height: f32,
    threshold: f32,
) -> (Rect, Vec<Guide>) {
    let mut xs = vec![0., 40., page_width / 2., page_width - 40., page_width];
    let mut ys = vec![0., 40., page_height / 2., page_height - 40., page_height];
    for r in others {
        xs.extend([r.x, r.x + r.width / 2., r.x + r.width]);
        ys.extend([r.y, r.y + r.height / 2., r.y + r.height]);
    }
    fn best(start: f32, len: f32, targets: &[f32], threshold: f32) -> Option<(f32, f32)> {
        let mut best = None;
        let mut dist = threshold;
        for edge in [start, start + len / 2., start + len] {
            for &t in targets {
                let d = t - edge;
                if d.abs() < dist {
                    dist = d.abs();
                    best = Some((d, t));
                }
            }
        }
        best
    }
    let mut guides = vec![];
    if let Some((d, pos)) = best(frame.x, frame.width, &xs, threshold) {
        frame.x += d;
        guides.push(Guide {
            vertical: true,
            position: pos,
        });
    }
    if let Some((d, pos)) = best(frame.y, frame.height, &ys, threshold) {
        frame.y += d;
        guides.push(Guide {
            vertical: false,
            position: pos,
        });
    }
    (frame, guides)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_and_center() {
        let (r, g) = snap_rect(Rect::new(38., 201., 100., 100.), &[], 600., 500., 6.);
        assert_eq!(r.x, 40.);
        assert_eq!(r.y, 200.);
        assert_eq!(g.len(), 2);
    }
    #[test]
    fn threshold_preserves_distant_position() {
        let r = Rect::new(123., 167., 47., 39.);
        assert_eq!(snap_rect(r, &[], 600., 800., 1.).0, r);
    }
}
