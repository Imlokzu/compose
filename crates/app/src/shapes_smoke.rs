//! Native shape gestures and the real picker/inspector controls.
use crate::{
    creation_smoke::{click, draw},
    workspace::{Field, Tool, Workspace},
};
use anyhow::{Result, ensure};
use document::{Color, Document, DocumentMode, NodeKind, Point as DocPoint, Rect, ShapeKind};
use gpui::*;

fn gesture(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    start: DocPoint,
    end: DocPoint,
) -> Result<()> {
    let (start, end) = window.update(cx, |s, _, _| {
        (
            s.screen_rect(Rect::new(start.x, start.y, 0., 0.)).origin,
            s.screen_rect(Rect::new(end.x, end.y, 0., 0.)).origin,
        )
    })?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        let modifiers = Modifiers {
            alt: true,
            ..Default::default()
        };
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                button: MouseButton::Left,
                click_count: 1,
                modifiers,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: end,
                pressed_button: Some(MouseButton::Left),
                modifiers,
            }),
            cx,
        );
        w.draw(cx).clear();
        w.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: end,
                pressed_button: Some(MouseButton::Left),
                modifiers,
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                button: MouseButton::Left,
                click_count: 1,
                modifiers,
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
fn number(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    field: Field,
    value: &str,
) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.begin_field(field, String::new());
        cx.notify();
    })?;
    draw(window, cx)?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        for ch in value.chars() {
            w.dispatch_keystroke(
                Keystroke {
                    key: ch.to_string(),
                    key_char: Some(ch.to_string()),
                    modifiers: Modifiers::default(),
                },
                cx,
            );
        }
        w.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        w.draw(cx).clear();
    })
}
pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.editor.document.name = "Figures".into();
        s.zoom = 0.72;
        s.fitted = true;
        cx.notify();
    })?;
    draw(window, cx)?;
    let mut ids = Vec::new();
    for (i, kind) in ShapeKind::ALL.into_iter().enumerate() {
        click(window, cx, "tool-shape")?;
        click(window, cx, crate::shapes::control(kind))?;
        let x = 48. + (i % 3) as f32 * 170.;
        let y = 54. + (i / 3) as f32 * 164.;
        let (start, end) = if kind == ShapeKind::Line {
            (DocPoint { x, y: y + 100. }, DocPoint { x: x + 110., y })
        } else {
            (
                DocPoint { x, y },
                DocPoint {
                    x: x + 110.,
                    y: y + 80.,
                },
            )
        };
        gesture(window, cx, start, end)?;
        let id = window.update(cx, |s, _, _| {
            let n = s.editor.selected_node().unwrap();
            ensure!(
                matches!(n.kind, NodeKind::Shape) && n.shape == kind,
                "Picker created the wrong figure: {kind:?}"
            );
            ensure!(
                s.tool == Tool::Select && s.popup.is_none(),
                "Drawing did not return to selection"
            );
            ensure!(
                kind != ShapeKind::Circle || n.frame.width == n.frame.height,
                "Circle creation stretched"
            );
            ensure!(
                kind != ShapeKind::Line || n.line_flipped,
                "Line direction was lost"
            );
            Ok::<_, anyhow::Error>(n.id)
        })??;
        ids.push(id);
    }
    let circle = ids[1];
    window.update(cx, |s, _, cx| {
        s.editor.select_only(Some(circle));
        cx.notify();
    })?;
    for handle in 0..8 {
        draw(window, cx)?;
        let frame = window.update(cx, |s, _, _| s.absolute_frame(circle).unwrap())?;
        let (x, y) = Workspace::handle_points(frame)[handle];
        gesture(
            window,
            cx,
            DocPoint { x, y },
            DocPoint {
                x: x + 21.,
                y: y + 13.,
            },
        )?;
        window.update(cx, |s, _, cx| {
            let n = s.editor.selected_node().unwrap();
            ensure!(
                (n.frame.width - n.frame.height).abs() < 0.001,
                "Circle handle {handle} stretched"
            );
            ensure!(n.frame != frame, "Circle handle {handle} did not resize");
            s.undo(false);
            ensure!(
                s.editor.document.node(circle).unwrap().frame == frame,
                "Circle resize undo failed"
            );
            cx.notify();
            Ok::<_, anyhow::Error>(())
        })??;
    }
    number(window, cx, Field::W, "125")?;
    window.update(cx, |s, _, _| {
        let n = s.editor.document.node(circle).unwrap();
        ensure!(
            n.frame.width == 125. && n.frame.height == 125.,
            "Circle numeric width lost proportions"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    number(window, cx, Field::H, "110")?;
    window.update(cx, |s, _, _| {
        let n = s.editor.document.node(circle).unwrap();
        ensure!(
            n.frame.width == 110. && n.frame.height == 110.,
            "Circle numeric height lost proportions"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    let rectangle = ids[0];
    let before = window.update(cx, |s, _, cx| {
        s.editor.select_only(Some(rectangle));
        let n = s.editor.selected_node_mut().unwrap();
        n.name = "Badge".into();
        n.fill = Color::rgb(190, 85, 65);
        n.radius = 6.;
        n.appearance.stroke_width = 2.;
        cx.notify();
        n.clone()
    })?;
    click(window, cx, "shape-type")?;
    click(window, cx, "shape-star")?;
    window.update(cx, |s, _, cx| {
        let after = s.editor.document.node(rectangle).unwrap();
        ensure!(
            after.shape == ShapeKind::Star
                && after.name == "Badge"
                && after.frame == before.frame
                && after.fill == before.fill
                && after.radius == before.radius,
            "Changing shape lost properties"
        );
        s.undo(false);
        ensure!(
            s.editor.document.node(rectangle).unwrap() == &before,
            "Shape change undo failed"
        );
        s.undo(true);
        ensure!(
            s.editor.document.node(rectangle).unwrap().shape == ShapeKind::Star,
            "Shape change redo failed"
        );
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    let moved_frame = window.update(cx, |s, _, _| s.absolute_frame(rectangle).unwrap())?;
    gesture(
        window,
        cx,
        DocPoint {
            x: moved_frame.x + moved_frame.width / 2.,
            y: moved_frame.y + moved_frame.height / 2.,
        },
        DocPoint {
            x: moved_frame.x + moved_frame.width / 2. + 17.,
            y: moved_frame.y + moved_frame.height / 2. + 23.,
        },
    )?;
    window.update(cx, |s, _, cx| {
        let frame = s.absolute_frame(rectangle).unwrap();
        ensure!(
            (frame.x - moved_frame.x - 17.).abs() < 0.01
                && (frame.y - moved_frame.y - 23.).abs() < 0.01,
            "Star did not drag normally"
        );
        s.undo(false);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    click(window, cx, "tool-shape")?;
    let drag_start = window.update(cx, |s, _, _| {
        s.to_doc(s.selection_control_bounds["shape-pentagon"].center())
    })?;
    let count = window.update(cx, |s, _, _| s.editor.document.pages[0].nodes.len())?;
    gesture(window, cx, drag_start, DocPoint { x: 438., y: 727. })?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.popup.is_none()
                && s.editor.document.pages[0].nodes.len() == count + 1
                && s.editor.selected_node().unwrap().shape == ShapeKind::Pentagon,
            "Dragging a picker figure did not insert it"
        );
        s.undo(false);
        s.editor.select_only(Some(rectangle));
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    let line = ids[9];
    window.update(cx, |s, _, cx| {
        s.editor.select_only(Some(line));
        cx.notify();
    })?;
    number(window, cx, Field::StrokeWidth, "5")?;
    let doc = window.update(cx, |s, _, _| {
        ensure!(
            s.editor
                .document
                .node(line)
                .unwrap()
                .appearance
                .stroke_width
                == 5.,
            "Line thickness inspector failed"
        );
        s.editor.select_only(None);
        Ok::<_, anyhow::Error>(s.editor.document.clone())
    })??;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/shapes");
    std::fs::create_dir_all(&dir)?;
    file_format::save(&dir.join("figures.compose"), &doc)?;
    ensure!(
        file_format::load(&dir.join("figures.compose"))? == doc,
        "Shape save/reopen mismatch"
    );
    export::write_png(&dir.join("figures.png"), &doc, 0, 144)?;
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.editor.document.mode = DocumentMode::Word;
        s.open_shapes(false);
        ensure!(s.popup.is_none(), "Word enabled figures");
        s.insert_at(Tool::Shape, DocPoint { x: 40., y: 40. });
        ensure!(
            s.editor.document.pages[0].nodes.is_empty(),
            "Word inserted a figure"
        );
        s.replace_document(doc, None);
        s.shape_kind = ShapeKind::Rectangle;
        if std::env::args().any(|a| a == "--show-shapes") {
            s.open_shapes(false);
        }
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    println!(
        "SHAPES_PASS: real native picker/draw/drop for all ten figures, shape movement, eight circle handles, numeric proportions, line direction/thickness, inspector conversion/style/undo/redo, Word restriction, exact native save/reopen and PNG"
    );
    Ok(())
}
