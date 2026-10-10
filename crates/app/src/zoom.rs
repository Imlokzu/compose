use crate::workspace::{Gesture, Workspace};
use document::{Point as DocPoint, Rect};
use gpui::*;

pub struct Pinch {
    pub anchor: Point<Pixels>,
    pub base_zoom: f32,
    pub scale: f32,
}
impl Workspace {
    pub fn zoom_at_pointer(&mut self, anchor: Point<Pixels>, target: f32) {
        let viewport = Rect::new(
            f32::from(self.viewport.left()),
            f32::from(self.viewport.top()),
            f32::from(self.viewport.size.width),
            f32::from(self.viewport.size.height),
        );
        let (zoom, pan) = layout::zoom_at(
            self.zoom,
            self.pan,
            viewport,
            self.editor.document.pages[self.editor.page].width,
            DocPoint {
                x: f32::from(anchor.x),
                y: f32::from(anchor.y),
            },
            target,
        );
        self.zoom = zoom;
        self.pan = pan;
        self.fitted = true;
    }
    pub fn magnify(&mut self, event: &MagnifyEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ended = matches!(event.touch_phase, TouchPhase::Ended);
        if matches!(event.touch_phase, TouchPhase::Started) {
            self.pinch = None;
            self.pinch_blocked = !self.viewport.contains(&event.position)
                || self.popup.is_some()
                || self.pending.is_some()
                || !matches!(self.gesture, Gesture::None);
        }
        if self.popup.is_some() || self.pending.is_some() || !matches!(self.gesture, Gesture::None)
        {
            self.pinch_blocked = true;
            self.pinch = None;
        }
        if self.pinch_blocked {
            if ended {
                self.pinch_blocked = false;
            }
            return;
        }
        if !event.magnification.is_finite() {
            if ended {
                self.pinch = None;
            }
            return;
        }
        if self.pinch.is_none() {
            if ended || !self.viewport.contains(&event.position) {
                return;
            }
            self.pinch = Some(Pinch {
                anchor: event.position,
                base_zoom: self.zoom,
                scale: 1.,
            });
        }
        let pinch = self.pinch.as_mut().unwrap();
        pinch.scale += event.magnification;
        let target = (pinch.base_zoom * pinch.scale).clamp(layout::MIN_ZOOM, layout::MAX_ZOOM);
        pinch.scale = target / pinch.base_zoom;
        let anchor = pinch.anchor;
        self.zoom_at_pointer(anchor, target);
        if ended {
            self.pinch = None;
        }
        cx.stop_propagation();
        cx.notify();
    }
}
