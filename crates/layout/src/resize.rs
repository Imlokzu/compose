use document::{ImageSizing, Point, Rect};

/// Handles start at top-left and continue clockwise through the edge midpoints.
/// A ratio constrains both dimensions, including when dragging an edge handle.
pub fn resize_rect(
    frame: Rect,
    handle: usize,
    delta: Point,
    ratio: Option<f32>,
    from_center: bool,
) -> Rect {
    let left = matches!(handle, 0 | 6 | 7);
    let right = matches!(handle, 2..=4);
    let top = matches!(handle, 0..=2);
    let bottom = matches!(handle, 4..=6);
    let multiplier = if from_center { 2. } else { 1. };
    let dx = delta.x * multiplier;
    let dy = delta.y * multiplier;
    let mut w = frame.width
        + if left {
            -dx
        } else if right {
            dx
        } else {
            0.
        };
    let mut h = frame.height
        + if top {
            -dy
        } else if bottom {
            dy
        } else {
            0.
        };
    if let Some(ratio) = ratio.filter(|r| r.is_finite() && *r > 0.) {
        let horizontal = left || right;
        let vertical = top || bottom;
        if horizontal && (!vertical || dx.abs() / frame.width >= dy.abs() / frame.height) {
            w = w.max(1.0_f32.max(ratio));
            h = w / ratio;
        } else {
            h = h.max(1.0_f32.max(1. / ratio));
            w = h * ratio;
        }
        let scale = (100000. / w.max(h)).min(1.);
        w *= scale;
        h *= scale;
    } else {
        w = w.clamp(1., 100000.);
        h = h.clamp(1., 100000.);
    }
    let x = if from_center || (!left && !right) {
        frame.x + (frame.width - w) / 2.
    } else if left {
        frame.x + frame.width - w
    } else {
        frame.x
    };
    let y = if from_center || (!top && !bottom) {
        frame.y + (frame.height - h) / 2.
    } else if top {
        frame.y + frame.height - h
    } else {
        frame.y
    };
    Rect::new(x, y, w, h)
}

/// Images in Scale mode keep their source aspect ratio, even in a legacy file
/// whose frame was previously stretched. The original asset is never resampled.
pub fn image_bounds(frame: Rect, source_width: f32, source_height: f32, mode: ImageSizing) -> Rect {
    if mode == ImageSizing::Stretch || source_width <= 0. || source_height <= 0. {
        return frame;
    }
    let scale = (frame.width / source_width).min(frame.height / source_height);
    let w = source_width * scale;
    let h = source_height * scale;
    Rect::new(
        frame.x + (frame.width - w) / 2.,
        frame.y + (frame.height - h) / 2.,
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }
    #[test]
    fn every_handle_preserves_ratio_and_opposite_anchor() {
        let f = Rect::new(30., 50., 240., 120.);
        for handle in 0..8 {
            let r = resize_rect(f, handle, Point { x: 37., y: 29. }, Some(2.), false);
            near(r.width / r.height, 2.);
            if matches!(handle, 0 | 6 | 7) {
                near(r.x + r.width, f.x + f.width)
            }
            if matches!(handle, 2..=4) {
                near(r.x, f.x)
            }
            if matches!(handle, 0..=2) {
                near(r.y + r.height, f.y + f.height)
            }
            if matches!(handle, 4..=6) {
                near(r.y, f.y)
            }
        }
    }
    #[test]
    fn centered_resize_tracks_twice_the_delta() {
        let f = Rect::new(10., 20., 200., 100.);
        let r = resize_rect(f, 3, Point { x: 20., y: 0. }, Some(2.), true);
        near(r.width, 240.);
        near(r.height, 120.);
        near(r.x + r.width / 2., 110.);
        near(r.y + r.height / 2., 70.);
    }
    #[test]
    fn stretch_changes_only_dragged_axis() {
        let f = Rect::new(10., 20., 200., 100.);
        let r = resize_rect(f, 3, Point { x: 50., y: 40. }, None, false);
        near(r.width, 250.);
        near(r.height, 100.);
        near(r.y, 20.);
    }
    #[test]
    fn very_small_resize_never_flips_or_loses_ratio() {
        let f = Rect::new(0., 0., 200., 100.);
        for handle in 0..8 {
            let r = resize_rect(
                f,
                handle,
                Point {
                    x: 10000.,
                    y: 10000.,
                },
                Some(2.),
                false,
            );
            assert!(r.valid());
            near(r.width / r.height, 2.);
        }
    }
    #[test]
    fn legacy_frames_do_not_distort_image() {
        let frame = Rect::new(10., 20., 200., 200.);
        let r = image_bounds(frame, 400., 200., ImageSizing::Scale);
        assert_eq!(r, Rect::new(10., 70., 200., 100.));
        assert_eq!(image_bounds(frame, 400., 200., ImageSizing::Stretch), frame);
    }
}
