use crate::workspace::{Popup, Workspace};
use anyhow::{Result, ensure};
use gpui::*;

fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}

/// An AppKit event probe calls the real view selector and GPUI's native converter.
/// It never posts input to the desktop or asks the user to perform a test gesture.
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn emit(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    position: Point<Pixels>,
    delta: f32,
    phase: TouchPhase,
) -> Result<()> {
    use cocoa::{
        base::id,
        foundation::{NSPoint, NSRect},
    };
    use objc::{
        declare::ClassDecl,
        msg_send,
        runtime::{Class, Object, Sel},
        sel, sel_impl,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::sync::OnceLock;
    static CLASS: OnceLock<&'static Class> = OnceLock::new();
    extern "C" fn event_type(_: &Object, _: Sel) -> u64 {
        30
    }
    extern "C" fn flags(_: &Object, _: Sel) -> u64 {
        0
    }
    extern "C" fn probe_phase(this: &Object, _: Sel) -> u64 {
        unsafe { *this.get_ivar::<u64>("probe_phase") }
    }
    extern "C" fn magnification(this: &Object, _: Sel) -> f64 {
        unsafe { *this.get_ivar::<f64>("probe_delta") }
    }
    extern "C" fn location(this: &Object, _: Sel) -> NSPoint {
        unsafe {
            NSPoint::new(
                *this.get_ivar::<f64>("probe_x"),
                *this.get_ivar::<f64>("probe_y"),
            )
        }
    }
    let class = CLASS.get_or_init(|| {
        let mut class =
            ClassDecl::new("ComposeMagnifyEventProbe", Class::get("NSObject").unwrap()).unwrap();
        class.add_ivar::<u64>("probe_phase");
        class.add_ivar::<f64>("probe_delta");
        class.add_ivar::<f64>("probe_x");
        class.add_ivar::<f64>("probe_y");
        unsafe {
            class.add_method(sel!(type), event_type as extern "C" fn(&Object, Sel) -> u64);
            class.add_method(
                sel!(modifierFlags),
                flags as extern "C" fn(&Object, Sel) -> u64,
            );
            class.add_method(
                sel!(phase),
                probe_phase as extern "C" fn(&Object, Sel) -> u64,
            );
            class.add_method(
                sel!(magnification),
                magnification as extern "C" fn(&Object, Sel) -> f64,
            );
            class.add_method(
                sel!(locationInWindow),
                location as extern "C" fn(&Object, Sel) -> NSPoint,
            );
        }
        class.register()
    });
    let (view, height) = AnyWindowHandle::from(window).update(cx, |_, w, _| -> Result<_> {
        let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(w)
            .map_err(|e| anyhow::anyhow!("Native window unavailable: {e:?}"))?
            .as_raw()
        else {
            anyhow::bail!("Expected an AppKit window");
        };
        let view = handle.ns_view.as_ptr() as id;
        let frame: NSRect = unsafe { msg_send![view, frame] };
        Ok((view, frame.size.height as f32))
    })??;
    unsafe {
        let event: id = msg_send![*class, new];
        let object = &mut *event;
        object.set_ivar(
            "probe_phase",
            match phase {
                TouchPhase::Started => 1_u64,
                TouchPhase::Moved => 4,
                TouchPhase::Ended => 8,
            },
        );
        object.set_ivar("probe_delta", delta as f64);
        object.set_ivar("probe_x", f32::from(position.x) as f64);
        object.set_ivar("probe_y", (height - f32::from(position.y)) as f64);
        let _: () = msg_send![view,magnifyWithEvent:event];
        let _: () = msg_send![event, release];
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn emit(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    position: Point<Pixels>,
    delta: f32,
    phase: TouchPhase,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::Magnify(MagnifyEvent {
                position,
                magnification: delta,
                touch_phase: phase,
                ..Default::default()
            }),
            cx,
        );
    })
}
fn scroll(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    p: Point<Pixels>,
    dx: f32,
    dy: f32,
    zoom: bool,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position: p,
                delta: ScrollDelta::Pixels(point(px(dx), px(dy))),
                modifiers: Modifiers {
                    control: zoom,
                    ..Default::default()
                },
                ..Default::default()
            }),
            cx,
        );
    })
}
pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    draw(window, cx)?;
    let (before, ids, dirty, zoom, pan, point) = window.update(cx, |s, _, _| {
        (
            s.editor.document.clone(),
            s.editor.selection_ids().to_vec(),
            s.dirty(),
            s.zoom,
            s.pan,
            s.viewport.center(),
        )
    })?;
    let original = window.update(cx, |s, _, _| s.to_doc(point))?;
    emit(window, cx, point, 0., TouchPhase::Started)?;
    emit(window, cx, point, 0.12, TouchPhase::Moved)?;
    emit(window, cx, point, 0.18, TouchPhase::Moved)?;
    draw(window, cx)?;
    window.update(cx, |s, _, _| {
        ensure!(
            (s.zoom - zoom * 1.3).abs() < 0.001,
            "Native magnification events did not change zoom"
        );
        let after = s.to_doc(point);
        ensure!(
            (after.x - original.x).abs() < 0.001 && (after.y - original.y).abs() < 0.001,
            "Pinch anchor drift: before {original:?}, after {after:?}, pointer {point:?}, native anchor {:?}",s.pinch.as_ref().map(|p|p.anchor)
        );
        Ok::<_, anyhow::Error>(())
    })??;
    emit(window, cx, point, -0.30, TouchPhase::Moved)?;
    emit(window, cx, point, 0., TouchPhase::Ended)?;
    draw(window, cx)?;
    ensure!(
        window.update(cx, |s, _, _| (s.zoom - zoom).abs() < 0.001
            && s.pinch.is_none())?,
        "Pinch-out/in did not return to the original view"
    );
    scroll(window, cx, point, 14., -22., false)?;
    draw(window, cx)?;
    ensure!(
        window.update(cx, |s, _, _| (s.zoom - zoom).abs() < 0.001
            && (s.pan.x - pan.x - 14.).abs() < 0.001
            && (s.pan.y - pan.y + 22.).abs() < 0.001)?,
        "Two-finger scrolling zoomed instead of panning"
    );
    let anchor = window.update(cx, |s, _, _| s.to_doc(point))?;
    scroll(window, cx, point, 0., 12., true)?;
    scroll(window, cx, point, 0., 18., true)?;
    draw(window, cx)?;
    ensure!(
        window.update(cx, |s, _, _| {
            let after = s.to_doc(point);
            (after.x - anchor.x).abs() < 0.001 && (after.y - anchor.y).abs() < 0.001
        })?,
        "Batched modifier-scroll zoom drifted before redraw"
    );
    let prior = window.update(cx, |s, _, _| s.zoom)?;
    let sidebar = gpui::point(px(80.), px(160.));
    emit(window, cx, sidebar, 0., TouchPhase::Started)?;
    emit(window, cx, point, 0.4, TouchPhase::Moved)?;
    emit(window, cx, point, 0., TouchPhase::Ended)?;
    ensure!(
        window.update(cx, |s, _, _| s.zoom == prior)?,
        "A pinch started in the sidebar zoomed the page"
    );
    window.update(cx, |s, _, cx| {
        s.toggle_popup(Popup::Export);
        cx.notify();
    })?;
    draw(window, cx)?;
    emit(window, cx, point, 0., TouchPhase::Started)?;
    emit(window, cx, point, 0.4, TouchPhase::Moved)?;
    emit(window, cx, point, 0., TouchPhase::Ended)?;
    ensure!(
        window.update(cx, |s, _, _| s.zoom == prior)?,
        "Pinch changed the canvas behind a popup"
    );
    window.update(cx, |s, _, cx| {
        s.close_popup();
        cx.notify();
    })?;
    draw(window, cx)?;
    emit(window, cx, point, 0., TouchPhase::Started)?;
    emit(window, cx, point, f32::NAN, TouchPhase::Moved)?;
    emit(window, cx, point, 0., TouchPhase::Ended)?;
    emit(window, cx, point, 0., TouchPhase::Started)?;
    emit(window, cx, point, 100., TouchPhase::Moved)?;
    draw(window, cx)?;
    ensure!(
        window.update(cx, |s, _, _| s.zoom == layout::MAX_ZOOM
            && s.pan.x.is_finite()
            && s.pan.y.is_finite())?,
        "Pinch exceeded the maximum zoom"
    );
    emit(window, cx, point, -0.1, TouchPhase::Moved)?;
    draw(window, cx)?;
    ensure!(
        window.update(cx, |s, _, _| s.zoom < layout::MAX_ZOOM)?,
        "Reversing after the zoom limit did not respond"
    );
    emit(window, cx, point, -100., TouchPhase::Moved)?;
    draw(window, cx)?;
    ensure!(
        window.update(cx, |s, _, _| s.zoom == layout::MIN_ZOOM)?,
        "Pinch exceeded the minimum zoom"
    );
    emit(window, cx, point, 0., TouchPhase::Ended)?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.document == before && s.editor.selection_ids() == ids && s.dirty() == dirty,
            "Touchpad navigation changed document content, selection or save state"
        );
        s.zoom = zoom;
        s.pan = pan;
        s.pinch = None;
        s.pinch_blocked = false;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    println!(
        "TOUCHPAD_PASS: AppKit magnifyWithEvent/native conversion, pinch in/out, stable anchors before redraw, two-finger pan, modifier-scroll zoom, sidebar/popup isolation, limits/reversal/invalid deltas, unchanged document/selection/save state"
    );
    Ok(())
}
