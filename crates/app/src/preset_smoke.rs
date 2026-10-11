use crate::{
    creation_smoke::{click, draw},
    workspace::{Popup, Workspace},
};
use anyhow::{Result, ensure};
use document::{Document, DocumentMode, NodeKind, presets::Preset};
use gpui::*;
use std::time::{Duration, Instant};

pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/presets");
    std::fs::create_dir_all(&dir)?;
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.open_creation(DocumentMode::Word);
        cx.notify();
    })?;
    click(window, cx, "preset-resume")?;
    click(window, cx, "preset-photo-resume")?;
    ensure!(
        window.update(cx, |s, _, _| s.creation.as_ref().unwrap().preset
            == Preset::Resume)?,
        "Photo résumé entered Word mode"
    );
    click(window, cx, "create-document")?;
    let resume = window.update(cx, |s, _, cx| {
        ensure!(
            s.word_mode() && !s.editor.document.word_overflows(0),
            "Word résumé does not fit its page"
        );
        ensure!(
            s.editor.document.pages[0].nodes.len() == 11,
            "Résumé is not editable text blocks"
        );
        s.editor
            .document
            .validate_word()
            .map_err(anyhow::Error::msg)?;
        s.saved = s.editor.document.clone();
        cx.notify();
        Ok::<_, anyhow::Error>(s.editor.document.clone())
    })??;
    ensure!(
        AnyWindowHandle::from(window)
            .update(cx, |_, w, cx| w.is_action_available(&crate::ExportWord, cx))?,
        "Word résumé has no native DOCX action"
    );
    cx.background_executor()
        .spawn({
            let dir = dir.clone();
            async move {
                export::word::write_docx(&dir.join("resume.docx"), &resume)?;
                export::write_png(&dir.join("resume.png"), &resume, 0, 144)?;
                file_format::save(&dir.join("resume.compose"), &resume)?;
                ensure!(
                    file_format::load(&dir.join("resume.compose"))? == resume,
                    "Résumé did not reopen"
                );
                Ok::<_, anyhow::Error>(())
            }
        })
        .await?;
    let mut photo_resume = None;
    for (preset, id, name) in [
        (Preset::PhotoResume, "preset-photo-resume", "photo-resume"),
        (Preset::PhotoCover, "preset-photo-cover", "photo-cover"),
        (Preset::Portfolio, "preset-portfolio", "portfolio"),
    ] {
        window.update(cx, |s, _, cx| {
            s.saved = s.editor.document.clone();
            s.open_creation(DocumentMode::Compose);
            cx.notify();
        })?;
        click(window, cx, id)?;
        click(window, cx, "create-mode-word")?;
        ensure!(
            window.update(cx, |s, _, _| s.creation.as_ref().unwrap().mode
                == DocumentMode::Compose)?,
            "Photo layout switched into Word mode"
        );
        click(window, cx, "create-document")?;
        let (doc, photo_id, position) = window.update(cx, |s, _, _| {
            ensure!(
                s.creation.is_none() && !s.word_mode(),
                "Photo preset did not enter its editor"
            );
            let image = s.editor.document.pages[0]
                .nodes
                .iter()
                .find(|n| matches!(n.kind, NodeKind::Image { .. }))
                .unwrap();
            ensure!(
                s.editor.document.assets.len() == 1,
                "Photo preset did not embed its image"
            );
            ensure!(
                s.layouts.len() > 5,
                "Photo preset has no editable text layouts"
            );
            let position = s.screen_rect(image.frame).center();
            Ok::<_, anyhow::Error>((s.editor.document.clone(), image.id, position))
        })??;
        AnyWindowHandle::from(window).update(cx, |_, w, cx| {
            w.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    position,
                    click_count: 1,
                    ..Default::default()
                }),
                cx,
            );
            w.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent {
                    position,
                    click_count: 1,
                    ..Default::default()
                }),
                cx,
            );
            w.draw(cx).clear();
        })?;
        ensure!(
            window.update(cx, |s, _, _| s.editor.selected() == Some(photo_id))?,
            "Preset photo cannot be selected on the canvas"
        );
        if preset == Preset::PhotoResume {
            click(window, cx, "replace-image")?;
            window.update(cx, |s, _, cx| {
                ensure!(
                    s.popup == Some(Popup::Images)
                        && s.image_picker.replace_target == Some(photo_id),
                    "Preset image replacement was not targeted"
                );
                s.load_local_image(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../assets/presets/architecture.png"),
                    cx,
                );
                Ok::<_, anyhow::Error>(())
            })??;
            let start = Instant::now();
            while window.update(cx, |s, _, _| s.image_picker.inserting)? {
                ensure!(
                    start.elapsed() < Duration::from_secs(8),
                    "Preset replacement timed out"
                );
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
            window.update(cx, |s, _, cx| {
                let original = doc.node(photo_id).unwrap();
                let current = s.editor.document.node(photo_id).unwrap();
                ensure!(
                    current.kind != original.kind,
                    "Preset image pixels were not replaced"
                );
                ensure!(
                    current.frame.x == original.frame.x
                        && current.frame.y == original.frame.y
                        && current.frame.width == original.frame.width
                        && current.radius == original.radius,
                    "Replacing portrait lost placement or style"
                );
                ensure!(
                    (current.frame.width / current.frame.height - 1.5).abs() < 0.001,
                    "Preset replacement distorted its new image"
                );
                s.undo(false);
                ensure!(
                    s.editor.document == doc,
                    "Preset image replacement was not one undo step"
                );
                cx.notify();
                Ok::<_, anyhow::Error>(())
            })??;
            photo_resume = Some(doc.clone());
        }
        cx.background_executor()
            .spawn({
                let dir = dir.clone();
                async move {
                    export::write_png(&dir.join(format!("{name}.png")), &doc, 0, 144)?;
                    file_format::save(&dir.join(format!("{name}.compose")), &doc)?;
                    ensure!(
                        file_format::load(&dir.join(format!("{name}.compose")))? == doc,
                        "Photo preset lost its layers or original bytes on reopening"
                    );
                    Ok::<_, anyhow::Error>(())
                }
            })
            .await?;
    }
    window.update(cx, |s, _, cx| {
        s.replace_document(photo_resume.unwrap(), None);
        if std::env::args().any(|a| a == "--show-photo-editor") {
            let id = s.editor.document.pages[0]
                .nodes
                .iter()
                .find(|n| matches!(n.kind, NodeKind::Image { .. }))
                .unwrap()
                .id;
            s.editor.select_only(Some(id));
        } else {
            s.open_creation(DocumentMode::Compose);
            s.select_preset(if std::env::args().any(|a| a == "--show-photo-cover") {
                Preset::PhotoCover
            } else {
                Preset::PhotoResume
            });
        }
        cx.notify();
    })?;
    draw(window, cx)?;
    println!(
        "PRESETS_PASS: native Word résumé creation/DOCX, photo-mode guards, scrolled portfolio creation, real text/image layers, canvas photo selection, actual Replace image/background decode/proportions/undo, embedded-photo save/reopen and PNG output"
    );
    Ok(())
}
