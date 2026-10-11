use crate::{
    creation_smoke::{click, draw},
    workspace::{Popup, Tool, Workspace},
};
use anyhow::{Result, ensure};
use document::{Document, DocumentMode, NodeKind, Point as DocPoint, Rect};
use gpui::*;
use std::time::{Duration, Instant};
fn key(window: WindowHandle<Workspace>, cx: &mut AsyncApp, key: &str) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
        w.draw(cx).clear();
    })
}
fn drag(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    start: Point<Pixels>,
    end: Point<Pixels>,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: start + point(px(8.), px(8.)),
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: end,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }),
            cx,
        );
        w.draw(cx).clear();
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
async fn settle(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let start = Instant::now();
    while window.update(cx, |s, _, _| s.busy)? {
        ensure!(
            start.elapsed() < Duration::from_secs(8),
            "Drop import timed out"
        );
        cx.background_executor()
            .timer(Duration::from_millis(30))
            .await;
    }
    draw(window, cx)
}
pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/actions");
    std::fs::create_dir_all(&dir)?;
    let ids = window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        let mut ids = vec![];
        for (y, name) in [(80., "First"), (160., "Second"), (240., "Third")] {
            s.insert_at(Tool::Text, DocPoint { x: 72., y });
            s.replace_input(0..0, name);
            s.end_edit();
            ids.push(s.editor.selected().unwrap());
        }
        cx.notify();
        ids
    })?;
    draw(window, cx)?;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    let (start, end) = window.update(cx, |s, _, _| {
        (
            s.layer_bounds[&ids[0]].center(),
            s.layer_bounds[&ids[2]].origin + point(px(105.), px(4.)),
        )
    })?;
    drag(window, cx, start, end)?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.document.pages[0].nodes.last().unwrap().id == ids[0],
            "Native top-down layer drag did not reorder"
        );
        for id in &ids {
            ensure!(
                s.editor.document.node(*id).unwrap().frame == before.node(*id).unwrap().frame,
                "Layer reorder moved an object"
            );
        }
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Layer reorder was not one undo step"
        );
        s.replace_document(Document::default(), None);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let (start, end) = window.update(cx, |s, _, _| {
        (
            s.selection_control_bounds["insert-text"].center(),
            s.screen_rect(Rect::new(92., 110., 1., 1.)).origin,
        )
    })?;
    drag(window, cx, start, end)?;
    window.update(cx, |s, _, cx| {
        let n = s
            .editor
            .selected_node()
            .ok_or_else(|| anyhow::anyhow!("Text drop created no object"))?;
        ensure!(
            matches!(n.kind, NodeKind::Text(_))
                && (n.frame.x - 92.).abs() < 0.1
                && (n.frame.y - 110.).abs() < 0.1,
            "Insert text drop used wrong placement: {:?}",
            n.frame
        );
        s.undo(false);
        ensure!(
            s.editor.document.pages[0].nodes.is_empty(),
            "Insert drop undo failed"
        );
        s.assets_tab = true;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let (start, end) = window.update(cx, |s, _, _| {
        (
            s.selection_control_bounds["asset-title"].center(),
            s.screen_rect(Rect::new(80., 130., 1., 1.)).origin,
        )
    })?;
    drag(window, cx, start, end)?;
    window.update(cx, |s, _, cx| {
        let n = s
            .editor
            .selected_node()
            .ok_or_else(|| anyhow::anyhow!("Asset drop created no component"))?;
        ensure!(
            !n.children.is_empty() && (n.frame.x - 80.).abs() < 0.1,
            "Asset drag created no editable frame"
        );
        s.undo(false);
        ensure!(
            s.editor.document.pages[0].nodes.is_empty(),
            "Component drop was not one undo step"
        );
        s.assets_tab = false;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let image_a = dir.join("drop-a.png");
    let image_b = dir.join("drop-b.png");
    image::RgbaImage::from_pixel(100, 60, image::Rgba([50, 110, 80, 255])).save(&image_a)?;
    image::RgbaImage::from_pixel(80, 80, image::Rgba([190, 130, 70, 255])).save(&image_b)?;
    let position = window.update(cx, |s, _, _| {
        s.screen_rect(Rect::new(96., 210., 1., 1.)).origin
    })?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::FileDrop(FileDropEvent::Entered {
                position,
                paths: ExternalPaths::new([image_a, image_b]),
            }),
            cx,
        );
        w.draw(cx).clear();
        w.dispatch_event(
            PlatformInput::FileDrop(FileDropEvent::Submit { position }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    settle(window, cx).await?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.document.pages[0].nodes.len() == 2 && s.editor.document.assets.len() == 2,
            "External image drop failed"
        );
        s.undo(false);
        ensure!(
            s.editor.document.pages[0].nodes.is_empty(),
            "Multiple-image drop was not one undo step"
        );
        s.insert_at(Tool::Text, DocPoint { x: 72., y: 110. });
        s.replace_input(0..0, "Read our project page for details.");
        s.anchor = 9;
        s.caret = 21;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    key(window, cx, "ctrl-k")?;
    ensure!(
        window.update(cx, |s, _, _| s.popup == Some(Popup::Link)
            && s.range() == (9..21))?,
        "Ctrl-K did not preserve the linked passage"
    );
    window.update(cx, |s, _, cx| {
        let value = s.active_range();
        s.replace_input(value, "example.com/project");
        cx.notify();
    })?;
    click(window, cx, "link-apply")?;
    let doc = window.update(cx, |s, _, cx| {
        let t = s.text().unwrap();
        ensure!(
            t.style_at(10).link.as_deref() == Some("https://example.com/project"),
            "Link URL was not normalized/applied"
        );
        ensure!(
            t.style_at(0).link.is_none() && t.style_at(25).link.is_none(),
            "Link leaked beyond selected passage"
        );
        let id = s.editor.selected().unwrap();
        let r = s.absolute_frame(id).unwrap();
        let highlight = s.layout().unwrap().selection(9..21)[0];
        let p = s
            .screen_rect(Rect::new(
                r.x + highlight.x + highlight.width / 2.,
                r.y + highlight.y + highlight.height / 2.,
                1.,
                1.,
            ))
            .origin;
        ensure!(
            s.link_at_screen(p).as_deref() == Some("https://example.com/project"),
            "Clickable link hit-test failed"
        );
        let copy = s.copy_item(false).unwrap();
        s.end_edit();
        s.editor.select_only(None);
        s.paste_item(&copy);
        ensure!(
            s.text().unwrap().style_at(0).link.is_some(),
            "Formatted clipboard lost hyperlink"
        );
        s.undo(false);
        s.editor.select_only(Some(id));
        s.editing = true;
        s.anchor = 9;
        s.caret = 21;
        cx.notify();
        Ok::<_, anyhow::Error>(s.editor.document.clone())
    })??;
    file_format::save(&dir.join("hyperlinks.compose"), &doc)?;
    ensure!(
        file_format::load(&dir.join("hyperlinks.compose"))? == doc,
        "Hyperlink native roundtrip failed"
    );
    key(window, cx, "ctrl-k")?;
    window.update(cx, |s, _, cx| {
        let r = s.active_range();
        s.replace_input(r, "javascript:bad()");
        cx.notify();
    })?;
    key(window, cx, "enter")?;
    ensure!(
        window.update(cx, |s, _, _| s.error
            && s.popup == Some(Popup::Link)
            && s.editor.document == doc)?,
        "Unsupported link changed the document"
    );
    key(window, cx, "escape")?;
    window.update(cx, |s, _, cx| {
        s.replace_document(
            Document {
                mode: DocumentMode::Word,
                ..Default::default()
            },
            None,
        );
        s.insert_at(Tool::Text, DocPoint { x: 72., y: 72. });
        s.replace_input(0..0, "First");
        s.end_edit();
        s.insert_at(Tool::Text, DocPoint { x: 72., y: 160. });
        s.replace_input(0..0, "Second");
        s.end_edit();
        cx.notify();
    })?;
    draw(window, cx)?;
    let (a, b, start, end) = window.update(cx, |s, _, _| {
        let a = s.editor.document.pages[0].nodes[0].id;
        let b = s.editor.document.pages[0].nodes[1].id;
        (
            a,
            b,
            s.layer_bounds[&b].center(),
            s.layer_bounds[&a].origin + point(px(105.), px(4.)),
        )
    })?;
    drag(window, cx, start, end)?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.document.pages[0].nodes[0].id == b
                && s.editor.document.pages[0].nodes[1].id == a,
            "Word layers do not follow dragged reading order"
        );
        ensure!(
            s.editor.document.pages[0].nodes[0].frame.y
                < s.editor.document.pages[0].nodes[1].frame.y,
            "Word block reflow failed"
        );
        s.replace_document(Document::default(), None);
        let bytes = include_bytes!("../../../assets/presets/portrait.png").to_vec();
        let (image, w, h) = Workspace::decode_image(&bytes).unwrap();
        s.insert_decoded_image(bytes, image, w, h, "Portrait".into(), None);
        s.background_service = None;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    click(window, cx, "remove-background")?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.popup == Some(Popup::Background)
                && s.editor.document == before
                && s.cutout_target.is_none(),
            "Unconfigured shared service changed an image"
        );
        ensure!(
            s.field.is_none(),
            "Shared service should not ask users for a provider key"
        );
        s.close_popup();
        s.set_status(
            "Drag layers or Insert items · select text and press Ctrl/⌘K",
            false,
        );
        if std::env::args().any(|a| a == "--show-background-settings") {
            s.open_background_settings();
        }
        if std::env::args().any(|a| a == "--show-links") {
            s.replace_document(doc.clone(), None);
            let id = s.editor.document.pages[0].nodes[0].id;
            s.editor.select_only(Some(id));
            s.editing = true;
            s.anchor = 9;
            s.caret = 21;
            s.open_link_editor();
        }
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    println!(
        "ACTIONS_PASS: native Layers drag/undo, Insert text/Asset drops, external multi-image drop/one-step undo, range hyperlink/Ctrl-K/hit-test/clipboard/roundtrip/unsafe URL guard, Word reading-order drag/reflow, shared-service setup guard and no provider-key prompt"
    );
    Ok(())
}
