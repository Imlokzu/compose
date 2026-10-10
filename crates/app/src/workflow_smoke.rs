//! Native regressions for clipboard and in-place image replacement.
//! Clipboard items are passed directly, leaving the user's OS clipboard untouched.
use crate::workspace::{Field, Popup, Tool, Workspace};
use anyhow::{Result, ensure};
use document::{Document, ImageSizing, NodeKind, Point as DocPoint, Rect};
use gpui::{
    AnyWindowHandle, AsyncApp, ClipboardItem, Keystroke, MouseDownEvent, MouseUpEvent,
    PlatformInput, WindowHandle, point, px,
};
use std::time::{Duration, Instant};

fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}

pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        let root = s
            .editor
            .insert_component(editor::Component::Callout, 60., 90.);
        let clip = s.copy_item(false).unwrap();
        let before = s.editor.document.clone();
        s.paste_item(&clip);
        let pasted = s.editor.selected().unwrap();
        ensure!(
            pasted != root && s.editor.selected_node().unwrap().children.len() == 3,
            "Subtree did not paste"
        );
        ensure!(
            s.editor.selected_node().unwrap().frame.x == 76.,
            "First paste did not offset"
        );
        s.paste_item(&clip);
        ensure!(
            s.editor.selected_node().unwrap().frame.x == 92.,
            "Repeated paste overlapped"
        );
        s.undo(false);
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Object paste was not one undo step"
        );
        s.undo(true);
        ensure!(
            s.editor.document.node(pasted).is_some(),
            "Object paste redo failed"
        );
        s.editor.select_only(Some(root));
        s.editor.toggle_lock(root);
        ensure!(s.copy_item(true).is_none(), "Cut removed a locked layer");
        let locked_clip = s.copy_item(false).unwrap();
        s.paste_item(&locked_clip);
        ensure!(
            !s.editor.selected_node().unwrap().locked,
            "Locked copy was not editable"
        );
        let cut_id = s.editor.selected().unwrap();
        let cut_clip = s.copy_item(true).unwrap();
        ensure!(
            s.editor.document.node(cut_id).is_none(),
            "Cut did not remove the layer"
        );
        s.undo(false);
        ensure!(s.editor.document.node(cut_id).is_some(), "Cut undo failed");
        s.replace_document(Document::default(), None);
        s.paste_item(&cut_clip);
        ensure!(
            s.editor.document.pages[0].nodes.len() == 1,
            "Clipboard did not survive document switch"
        );
        ensure!(
            s.editor.document.validate().is_ok(),
            "Pasted document invalid"
        );

        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 64. });
        s.replace_input(0..0, "Small BIG 👩‍💻 end");
        s.anchor = 6;
        s.caret = 9;
        s.format(|v| {
            v.size = 32.;
            v.weight = 700.;
        });
        s.anchor = 6;
        s.caret = 21;
        let rich_clip = s.copy_item(false).unwrap();
        s.end_edit();
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 170. });
        s.replace_input(0..0, "left right");
        s.anchor = 4;
        s.caret = 5;
        let before = s.editor.document.clone();
        s.paste_item(&rich_clip);
        ensure!(
            s.text().unwrap().content == "leftBIG 👩‍💻right",
            "Rich content paste failed"
        );
        ensure!(
            s.text().unwrap().style_at(4).size == 32.
                && s.text().unwrap().style_at(0).size == 16.
                && s.text().unwrap().style_at(19).size == 16.,
            "Paste changed neighbor styles"
        );
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Rich paste undo split the transaction"
        );
        s.end_edit();
        s.paste_item(&ClipboardItem::new_string("A new text box".into()));
        ensure!(
            s.text().unwrap().content == "A new text box" && s.editing,
            "Plain text did not create a text box"
        );
        s.begin_field(Field::Name, String::new());
        s.paste_item(&rich_clip);
        ensure!(
            s.field.as_ref().unwrap().value == "BIG 👩‍💻",
            "Inspector paste used object formatting"
        );
        s.field = None;
        s.end_edit();
        let before = s.editor.document.clone();
        s.toggle_preview();
        s.paste_item(&cut_clip);
        ensure!(
            s.editor.document == before && s.copy_item(true).is_none(),
            "Preview allowed clipboard edits"
        );
        s.toggle_preview();

        let dir = tempfile::tempdir()?;
        let old_path = dir.path().join("old.png");
        let new_path = dir.path().join("new.png");
        image::RgbaImage::from_pixel(400, 200, image::Rgba([90, 160, 120, 255])).save(&old_path)?;
        image::RgbaImage::from_pixel(200, 300, image::Rgba([70, 120, 190, 255])).save(&new_path)?;
        s.import_image(&old_path)?;
        let id = s.editor.selected().unwrap();
        {
            let n = s.editor.selected_node_mut().unwrap();
            n.frame = Rect::new(60., 270., 240., 120.);
            n.name = "A carefully placed photo".into();
            n.radius = 18.;
            n.opacity = 0.8;
            n.appearance.stroke_width = 2.;
            n.appearance.shadow = true;
        }
        let before = s.editor.document.clone();
        s.open_image_replacement(cx);
        ensure!(
            s.popup == Some(Popup::Images) && s.image_picker.replace_target == Some(id),
            "Replacement picker did not target the layer"
        );
        let bytes = std::fs::read(&new_path)?;
        let (image, w, h) = Workspace::decode_image(&bytes)?;
        s.apply_picker_image(bytes, image, w, h, "new name".into(), None)?;
        s.close_popup();
        let n = s.editor.selected_node().unwrap();
        ensure!(
            n.id == id
                && n.name == "A carefully placed photo"
                && n.radius == 18.
                && n.opacity == 0.8
                && n.appearance.shadow
                && n.appearance.stroke_width == 2.,
            "Replacement lost layer identity/style"
        );
        ensure!(
            n.frame == Rect::new(60., 270., 240., 360.),
            "Scale replacement did not preserve position/width/proportions"
        );
        s.undo(false);
        ensure!(s.editor.document == before, "Replacement undo failed");
        s.undo(true);
        let replaced = s.editor.document.clone();
        let path = dir.path().join("replaced.compose");
        file_format::save(&path, &replaced)?;
        ensure!(
            file_format::load(&path)? == replaced,
            "Replacement did not reopen exactly"
        );
        s.set_image_sizing(ImageSizing::Stretch);
        s.editor.selected_node_mut().unwrap().frame = Rect::new(60., 270., 240., 120.);
        let stretched = s.editor.selected_node().unwrap().frame;
        s.open_image_replacement(cx);
        let bytes = std::fs::read(&old_path)?;
        let (image, w, h) = Workspace::decode_image(&bytes)?;
        s.apply_picker_image(bytes, image, w, h, "old name".into(), None)?;
        ensure!(
            s.editor.selected_node().unwrap().frame == stretched,
            "Stretch replacement changed dimensions"
        );
        s.editor.toggle_lock(id);
        let locked = s.editor.document.clone();
        let bytes = std::fs::read(&new_path)?;
        let (image, w, h) = Workspace::decode_image(&bytes)?;
        ensure!(
            s.apply_picker_image(bytes, image, w, h, "locked".into(), None)
                .is_err()
                && s.editor.document == locked,
            "Pending replacement changed a locked layer"
        );
        s.close_popup();
        s.editor.toggle_lock(id);

        // A pasted image remains self-contained after leaving the original document.
        let image_clip = s.copy_item(false).unwrap();
        let old_bytes = match &s.editor.selected_node().unwrap().kind {
            NodeKind::Image { asset } => s.editor.document.assets[asset].clone(),
            _ => unreachable!(),
        };
        s.replace_document(Document::default(), None);
        s.paste_item(&image_clip);
        let NodeKind::Image { asset } = &s.editor.selected_node().unwrap().kind else {
            unreachable!()
        };
        ensure!(
            *s.editor.document.assets[asset] == *old_bytes,
            "Image clipboard lost original bytes"
        );
        let before = s.editor.document.clone();
        s.open_image_replacement(cx);
        s.close_popup();
        ensure!(
            s.image_picker.replace_target.is_none() && s.editor.document == before,
            "Cancelling replacement changed the layer"
        );
        s.editor.document.name = "Everyday editing".into();
        s.set_status("Copy layers · paste styled text · replace images", false);
        s.open_image_replacement(cx);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        // Native Escape still dismisses the replacement modal.
        w.dispatch_keystroke(Keystroke::parse("escape").unwrap(), cx);
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.popup.is_none() && s.image_picker.replace_target.is_none(),
            "Native Escape did not cancel replacement"
        );
        ensure!(
            s.editor.selected_node().is_some(),
            "Modal Escape deselected the image"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    println!(
        "WORKFLOW_PASS: nested object copy/cut/paste, repeat offsets, locks, cross-document images, formatted Unicode, inspector/plain-text paste, preview isolation, image replacement scale/stretch, style identity, cancellation, undo/redo and exact reopen"
    );
    Ok(())
}

pub async fn background_replacement(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
) -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Replacement photo.png");
    image::RgbaImage::from_pixel(150, 90, image::Rgba([160, 115, 70, 255])).save(&path)?;
    let (button, before, id) = window.update(cx, |s, w, _| {
        (
            point(
                w.bounds().size.width - px(136.),
                s.viewport.origin.y + px(182.),
            ),
            s.editor.document.clone(),
            s.editor.selected().unwrap(),
        )
    })?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: button,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: button,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.popup == Some(Popup::Images) && s.image_picker.replace_target == Some(id),
            "Native Replace image button did not open the targeted modal"
        );
        s.load_local_image(path.clone(), cx);
        Ok::<_, anyhow::Error>(())
    })??;
    let start = Instant::now();
    while window.update(cx, |s, _, _| s.image_picker.inserting)? {
        ensure!(
            start.elapsed() < Duration::from_secs(8),
            "Background replacement timed out"
        );
        cx.background_executor()
            .timer(Duration::from_millis(50))
            .await;
    }
    window.update(cx, |s, _, cx| {
        ensure!(
            s.popup.is_none() && s.editor.selected() == Some(id),
            "Background replacement left modal open or changed layer"
        );
        let n = s.editor.selected_node().unwrap();
        ensure!(
            n.frame == before.node(id).unwrap().frame && n.name == before.node(id).unwrap().name,
            "Background replacement changed Stretch geometry or name"
        );
        ensure!(
            n.kind != before.node(id).unwrap().kind,
            "Background replacement did not replace pixels"
        );
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Background replacement undo failed"
        );
        // Invalidate while the decode worker is running. Its completion must be ignored.
        s.open_image_replacement(cx);
        s.load_local_image(path.clone(), cx);
        s.close_popup();
        Ok::<_, anyhow::Error>(())
    })??;
    cx.background_executor()
        .timer(Duration::from_millis(250))
        .await;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.document == before && s.popup.is_none(),
            "Cancelled background replacement changed the document"
        );
        s.open_image_replacement(cx);
        s.load_local_image(path.clone(), cx);
        // Simulate a target that disappears before the result reaches the UI.
        s.editor.delete_selected();
        Ok::<_, anyhow::Error>(())
    })??;
    let start = Instant::now();
    while window.update(cx, |s, _, _| s.image_picker.inserting)? {
        ensure!(
            start.elapsed() < Duration::from_secs(8),
            "Deleted-target replacement timed out"
        );
        cx.background_executor()
            .timer(Duration::from_millis(50))
            .await;
    }
    window.update(cx, |s, _, cx| {
        ensure!(
            s.image_picker.error.is_some() && s.editor.document.node(id).is_none(),
            "Deleted image target was silently recreated"
        );
        s.close_popup();
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Deleted-target test changed surviving document"
        );
        s.editor.select_only(Some(id));
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    println!(
        "BACKGROUND_REPLACEMENT_PASS: actual inspector button, background local decode, preserved target geometry, undo, cancelled download and deleted target rejection"
    );
    Ok(())
}
