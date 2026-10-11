//! Real native pointer/keyboard tests for creation, drawn boxes and context menus.
use crate::workspace::{Gesture, Popup, Tool, Workspace};
use anyhow::{Result, ensure};
use document::{Document, DocumentMode, NodeKind, Rect, presets::Preset};
use gpui::*;

pub(crate) fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}
fn pointer(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    start: Point<Pixels>,
    end: Point<Pixels>,
    button: MouseButton,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                button,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        if start != end {
            w.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: end,
                    pressed_button: Some(button),
                    modifiers: Modifiers {
                        alt: true,
                        ..Default::default()
                    },
                }),
                cx,
            );
        }
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                button,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
pub(crate) fn click(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    id: &'static str,
) -> Result<()> {
    draw(window, cx)?;
    if id.starts_with("preset-") && id != "preset-gallery" {
        for _ in 0..4 {
            let (bounds, gallery) = window.update(cx, |s, _, _| {
                (
                    s.selection_control_bounds.get(id).copied(),
                    s.selection_control_bounds.get("preset-gallery").copied(),
                )
            })?;
            let (Some(bounds), Some(gallery)) = (bounds, gallery) else {
                break;
            };
            if bounds.top() >= gallery.top() && bounds.bottom() <= gallery.bottom() {
                break;
            }
            let delta = if bounds.top() < gallery.top() {
                gallery.top() - bounds.top() + px(4.)
            } else {
                gallery.bottom() - bounds.bottom() - px(4.)
            };
            AnyWindowHandle::from(window).update(cx, |_, w, cx| {
                w.dispatch_event(
                    PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: gallery.center(),
                        delta: ScrollDelta::Pixels(point(px(0.), delta)),
                        ..Default::default()
                    }),
                    cx,
                );
                w.draw(cx).clear();
            })?;
        }
    }
    let position = window
        .update(cx, |s, _, _| {
            s.selection_control_bounds.get(id).map(|b| b.center())
        })?
        .ok_or_else(|| anyhow::anyhow!("Missing creation/context control: {id}"))?;
    pointer(window, cx, position, position, MouseButton::Left)
}
fn key(window: WindowHandle<Workspace>, cx: &mut AsyncApp, key: &str) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
        w.draw(cx).clear();
    })
}
fn point_on_page(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    x: f32,
    y: f32,
) -> Result<Point<Pixels>> {
    window.update(cx, |s, _, _| s.screen_rect(Rect::new(x, y, 1., 1.)).origin)
}
pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.open_creation(DocumentMode::Compose);
        cx.notify();
    })?;
    draw(window, cx)?;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    click(window, cx, "preset-report")?;
    click(window, cx, "create-mode-word")?;
    click(window, cx, "preset-poster")?;
    window.update(cx, |s, _, _| {
        let draft = s.creation.as_ref().unwrap();
        ensure!(
            draft.preset == Preset::Report && draft.mode == DocumentMode::Word,
            "Unsupported poster entered Word mode"
        );
        ensure!(
            s.editor.document == before,
            "Preset selection changed the working file"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    click(window, cx, "create-name")?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse("cmd-a").unwrap(), cx);
        for ch in "My first report".chars() {
            w.dispatch_keystroke(
                Keystroke {
                    key: ch.to_string(),
                    key_char: Some(ch.to_string()),
                    modifiers: Modifiers::default(),
                },
                cx,
            );
        }
        w.draw(cx).clear();
    })?;
    click(window, cx, "create-document")?;
    let word = window.update(cx, |s, _, _| {
        ensure!(
            s.creation.is_none() && s.word_mode(),
            "Create did not enter Word editor"
        );
        ensure!(
            s.editor.document.name == "My first report",
            "Name input was not saved: {}",
            s.editor.document.name
        );
        ensure!(
            s.editor.document.pages[0].nodes.len() == 8,
            "Report is not editable layers"
        );
        ensure!(
            !s.editor.document.word_overflows(0),
            "Report preset overflows"
        );
        Ok::<_, anyhow::Error>(s.editor.document.clone())
    })??;
    ensure!(
        AnyWindowHandle::from(window)
            .update(cx, |_, w, cx| w.is_action_available(&crate::ExportWord, cx))?,
        "Created Word file cannot export DOCX"
    );
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/creation");
    std::fs::create_dir_all(&dir)?;
    cx.background_executor()
        .spawn({
            let doc = word.clone();
            let dir = dir.clone();
            async move {
                file_format::save(&dir.join("report.compose"), &doc)?;
                ensure!(
                    file_format::load(&dir.join("report.compose"))? == doc,
                    "Preset roundtrip failed"
                );
                export::word::write_docx(&dir.join("report.docx"), &doc)?;
                export::write_png(&dir.join("report.png"), &doc, 0, 144)?;
                Ok::<_, anyhow::Error>(())
            }
        })
        .await?;
    window.update(cx, |s, _, cx| {
        let id = s.editor.document.pages[0].nodes[0].id;
        s.editor.select_only(Some(id));
        s.editing = true;
        s.replace_input(0..0, "Edited ");
        s.end_edit();
        cx.notify();
    })?;
    let edited = window.update(cx, |s, _, _| s.editor.document.clone())?;
    key(window, cx, "cmd-n")?;
    click(window, cx, "preset-letter")?;
    click(window, cx, "create-document")?;
    ensure!(
        window.update(cx, |s, _, _| s.pending.is_some()
            && s.editor.document == edited)?,
        "Creation lost unsaved document"
    );
    click(window, cx, "cancel-pending")?;
    click(window, cx, "create-cancel")?;
    ensure!(
        window.update(cx, |s, _, _| s.creation.is_none()
            && s.editor.document == edited)?,
        "Cancel lost the working document"
    );
    key(window, cx, "cmd-n")?;
    click(window, cx, "preset-poster")?;
    click(window, cx, "create-mode-word")?;
    ensure!(
        window.update(cx, |s, _, _| s.creation.as_ref().unwrap().mode
            == DocumentMode::Compose)?,
        "Compose-only preset switched to Word"
    );
    click(window, cx, "create-document")?;
    click(window, cx, "discard-pending")?;
    let poster = window.update(cx, |s, _, _| {
        ensure!(
            !s.word_mode() && s.creation.is_none(),
            "Poster creation failed"
        );
        ensure!(
            s.editor.document.pages[0]
                .nodes
                .iter()
                .any(|n| matches!(n.kind, NodeKind::Shape)),
            "Poster has no real shape"
        );
        Ok::<_, anyhow::Error>(s.editor.document.clone())
    })??;
    ensure!(
        !AnyWindowHandle::from(window)
            .update(cx, |_, w, cx| w.is_action_available(&crate::ExportWord, cx))?,
        "Poster exposes Word export"
    );
    cx.background_executor()
        .spawn({
            let dir = dir.clone();
            async move { export::write_png(&dir.join("poster.png"), &poster, 0, 144) }
        })
        .await?;
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.tool = Tool::Text;
        cx.notify();
    })?;
    draw(window, cx)?;
    let start = point_on_page(window, cx, 80., 100.)?;
    let end = point_on_page(window, cx, 370., 310.)?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document.pages[0].nodes.is_empty()
            && matches!(s.gesture, Gesture::Create { .. }))?,
        "Text spawned before its size was drawn"
    );
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: end,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    let id = window.update(cx, |s, _, cx| {
        let n = s.editor.selected_node().unwrap();
        ensure!(
            (n.frame.width - 290.).abs() < 0.1 && (n.frame.height - 210.).abs() < 0.1,
            "Drawn text bounds wrong: {:?}",
            n.frame
        );
        let id = n.id;
        s.replace_input(0..0, "Move me without losing my size.");
        ensure!(
            (s.editor.document.node(id).unwrap().frame.height - 210.).abs() < 0.1,
            "Typing erased drawn box height"
        );
        cx.notify();
        Ok::<_, anyhow::Error>(id)
    })??;
    draw(window, cx)?;
    let start = point_on_page(window, cx, 225., 100.)?;
    let end = point_on_page(window, cx, 250., 128.)?;
    pointer(window, cx, start, end, MouseButton::Left)?;
    window.update(cx, |s, _, _| {
        let n = s.editor.document.node(id).unwrap();
        ensure!(
            (n.frame.x - 105.).abs() < 0.1 && (n.frame.y - 128.).abs() < 0.1,
            "Text border failed to move: {:?}",
            n.frame
        );
        s.undo(false);
        ensure!(
            (s.editor.document.node(id).unwrap().frame.x - 80.).abs() < 0.1,
            "Border move was not one undo step"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let position = point_on_page(window, cx, 180., 180.)?;
    window.update(cx, |s, _, cx| {
        s.tool = Tool::Text;
        cx.notify();
    })?;
    pointer(window, cx, position, position, MouseButton::Right)?;
    ensure!(
        window.update(cx, |s, _, _| s.popup == Some(Popup::Context)
            && s.editor.document.pages[0].nodes.len() == 1)?,
        "Right click spawned an object"
    );
    click(window, cx, "context-duplicate")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document.pages[0].nodes.len() == 2)?,
        "Context duplicate did not work"
    );
    window.update(cx, |s, _, cx| {
        s.undo(false);
        s.editor.select_only(Some(id));
        cx.notify();
    })?;
    draw(window, cx)?;
    pointer(window, cx, position, position, MouseButton::Right)?;
    click(window, cx, "context-lock")?;
    pointer(window, cx, position, position, MouseButton::Right)?;
    click(window, cx, "context-delete")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document.node(id).is_some())?,
        "Context menu deleted a locked object"
    );
    click(window, cx, "context-lock")?;
    pointer(window, cx, position, position, MouseButton::Right)?;
    click(window, cx, "context-delete")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document.node(id).is_none())?,
        "Context delete failed"
    );
    key(window, cx, "cmd-z")?;
    window.update(cx, |s, _, cx| {
        s.tool = Tool::Text;
        s.end_edit();
        cx.notify();
    })?;
    draw(window, cx)?;
    let blank = point_on_page(window, cx, 420., 430.)?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: blank,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    key(window, cx, "escape")?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: blank,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document.pages[0].nodes.len() == 1)?,
        "Escape did not cancel drawn text"
    );
    window.update(cx, |s, _, cx| {
        s.tool = Tool::Shape;
        cx.notify();
    })?;
    let a = point_on_page(window, cx, 500., 600.)?;
    let b = point_on_page(window, cx, 420., 500.)?;
    pointer(window, cx, a, b, MouseButton::Left)?;
    window.update(cx, |s, _, cx| {
        let r = s.editor.selected_node().unwrap().frame;
        ensure!(
            (r.x - 420.).abs() < 0.1
                && (r.width - 80.).abs() < 0.1
                && (r.height - 100.).abs() < 0.1,
            "Reverse shape drawing failed"
        );
        s.undo(false);
        s.editor.select_only(Some(id));
        s.editing = true;
        s.anchor = 0;
        s.caret = 7;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let p = point_on_page(window, cx, 210., 180.)?;
    pointer(window, cx, p, p, MouseButton::Right)?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.range() == (0..7) && s.editing,
            "Right-click lost the selected passage"
        );
        s.close_popup();
        ensure!(
            s.copy_item(false).is_some(),
            "Selected text cannot be copied"
        );
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let layer = window.update(cx, |s, _, _| s.layer_bounds[&id].center())?;
    pointer(window, cx, layer, layer, MouseButton::Right)?;
    ensure!(
        window.update(cx, |s, _, _| s.popup == Some(Popup::Context)
            && s.editor.selected() == Some(id))?,
        "Layer right click failed"
    );
    click(window, cx, "context-edit")?;
    window.update(cx, |s, _, cx| {
        s.end_edit();
        s.open_creation(DocumentMode::Compose);
        s.select_preset(Preset::Report);
        cx.notify();
    })?;
    println!(
        "CREATION_PASS: real editable presets, name input, Word restrictions/DOCX, save/reopen, unsaved Cancel/Discard, drag-to-size text, preserved height, border move/undo, right-click no insertion, duplicate/lock/delete/undo, Escape cancellation, reverse shape draw, selected passage and Layers menus"
    );
    Ok(())
}
