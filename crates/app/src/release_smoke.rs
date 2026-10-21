//! Native regressions for the Windows preview's editing safeguards.
use crate::workspace::{Field, Pending, Workspace};
use anyhow::{Result, ensure};
use document::{Document, NodeKind, Rect};
use gpui::{AnyWindowHandle, AsyncApp, Keystroke, WindowHandle};

fn key(window: WindowHandle<Workspace>, cx: &mut AsyncApp, key: &str) -> Result<()> {
    let primary = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    let key = key.replacen("cmd", primary, 1);
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse(&key).unwrap(), cx);
        w.draw(cx).clear();
    })
}

pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.editor
            .insert(NodeKind::Shape, Rect::new(60., 60., 100., 100.));
        s.pending = Some(Pending::Close);
        cx.notify();
    })?;
    crate::creation_smoke::draw(window, cx)?;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    for shortcut in ["cmd-z", "cmd-shift-z", "cmd-shift-e", "cmd-o", "cmd-n"] {
        key(window, cx, shortcut)?;
        ensure!(
            window.update(cx, |s, _, _| {
                s.editor.document == before
                    && matches!(s.pending, Some(Pending::Close))
                    && s.creation.is_none()
                    && s.popup.is_none()
                    && !s.busy
            })?,
            "Shortcut {shortcut} escaped the unsaved-changes prompt"
        );
    }
    key(window, cx, "escape")?;
    ensure!(
        window.update(cx, |s, _, _| s.pending.is_none())?,
        "Escape did not cancel the prompt"
    );
    key(window, cx, "cmd-z")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document != before)?,
        "Undo stopped working outside the prompt"
    );
    key(window, cx, "cmd-shift-z")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document == before)?,
        "Redo failed"
    );
    window.update(cx, |s, _, cx| {
        s.begin_field(Field::W, "100".into());
        s.field.as_mut().unwrap().value = "not a width".into();
        cx.notify();
    })?;
    crate::creation_smoke::draw(window, cx)?;
    key(window, cx, "tab")?;
    ensure!(
        window.update(cx, |s, _, _| {
            s.field.as_ref().is_some_and(|f| f.value == "not a width")
                && s.error
                && s.editor.document == before
        })?,
        "Tab discarded invalid input or changed the document"
    );
    window.update(cx, |s, _, cx| {
        s.field.as_mut().unwrap().value = "150".into();
        cx.notify();
    })?;
    key(window, cx, "tab")?;
    ensure!(
        window.update(cx, |s, _, _| {
            s.field.is_none()
                && !s.error
                && s.editor
                    .selected_node()
                    .is_some_and(|n| n.frame.width == 150.)
        })?,
        "Corrected field could not be committed with Tab"
    );
    println!(
        "RELEASE_PASS: modal keyboard isolation, normal undo/redo, invalid Tab edit retention and correction"
    );
    Ok(())
}
