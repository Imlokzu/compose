use crate::{
    output::{self, OutputFormat},
    workspace::{Pending, Popup, Tool, Workspace},
};
use anyhow::{Result, ensure};
use document::{Color, Document, DocumentMode, Point as DocPoint};
use gpui::*;
fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}
fn mode_click(window: WindowHandle<Workspace>, cx: &mut AsyncApp, id: &'static str) -> Result<()> {
    draw(window, cx)?;
    let position = window
        .update(cx, |s, _, _| {
            s.selection_control_bounds.get(id).map(|b| b.center())
        })?
        .ok_or_else(|| anyhow::anyhow!("Mode button was not laid out: {id}"))?;
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
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}

pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/word-mode");
    std::fs::create_dir_all(&dir)?;
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.toggle_popup(Popup::Modes);
        cx.notify();
    })?;
    mode_click(window, cx, "mode-word")?;
    ensure!(
        window.update(cx, |s, _, _| s.word_mode() && s.popup.is_none())?,
        "Native Word mode button did not switch the document"
    );
    ensure!(
        AnyWindowHandle::from(window)
            .update(cx, |_, w, cx| w.is_action_available(&crate::ExportWord, cx))?,
        "Word export unavailable in the native File menu"
    );
    window.update(cx, |s, _, cx| {
        s.toggle_popup(Popup::Modes);
        cx.notify();
    })?;
    mode_click(window, cx, "mode-compose")?;
    ensure!(
        window.update(cx, |s, _, _| !s.word_mode())?,
        "Native Compose mode button did not restore canvas mode"
    );
    ensure!(
        !AnyWindowHandle::from(window)
            .update(cx, |_, w, cx| w.is_action_available(&crate::ExportWord, cx))?,
        "Compose mode exposes native Word export"
    );
    window.update(cx,|s,_,cx| {
        s.replace_document(Document::default(),None);
        s.switch_document_mode(DocumentMode::Word);
        ensure!(s.word_mode(),"Empty document did not switch to Word mode");
        s.insert_at(Tool::Shape,DocPoint{x:40.,y:50.});ensure!(s.editor.document.pages[0].nodes.is_empty(),"Word mode inserted a canvas shape");
        s.insert_at(Tool::Text,DocPoint{x:140.,y:50.});s.replace_input(0..0,"Word compatible document");s.end_edit();s.format(|style|{style.size=28.;style.weight=700.;});
        s.end_edit();s.insert_at(Tool::Text,DocPoint{x:40.,y:240.});s.replace_input(0..0,"Text stays editable in Microsoft Word.\nThis paragraph has bold, italic and colored text.");
        s.anchor=0;s.caret=4;s.format(|style|{style.weight=700.;style.italic=true;style.color=Color::rgb(46,113,83);});s.end_edit();
        let data=include_bytes!("../../../brand/reference-ui.jpg").to_vec();let(image,w,h)=Workspace::decode_image(&data)?;
        s.insert_decoded_image(data,image,w,h,"An embedded image".into(),None);
        let image=s.editor.selected().unwrap();ensure!(s.editor.document.node(image).unwrap().radius==0.,"Word image insertion added canvas corners");
        s.set_radius(10.);s.toggle_shadow();s.set_image_sizing(document::ImageSizing::Stretch);
        ensure!(s.editor.document.validate_word().is_ok(),"Word-only controls allowed incompatible effects");
        s.editor.document.name="Word compatible document".into();s.measure();
        ensure!(s.editor.document.pages[0].nodes.windows(2).all(|pair|pair[0].frame.y+pair[0].frame.height<=pair[1].frame.y),"Word blocks overlap");
        s.toggle_popup(Popup::Export);cx.notify();Ok::<_,anyhow::Error>(())
    })??;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())?;
    let doc = window.update(cx, |s, _, _| s.editor.document.clone())?;
    cx.background_executor()
        .spawn({
            let dir = dir.clone();
            let doc = doc.clone();
            async move {
                output::write(
                    &dir.join("word-proof.docx"),
                    OutputFormat::Word,
                    &doc,
                    0,
                    144,
                )?;
                file_format::save(&dir.join("word-proof.compose"), &doc)?;
                ensure!(
                    file_format::load(&dir.join("word-proof.compose"))? == doc,
                    "Word mode did not survive native save/reopen"
                );
                export::write_png(&dir.join("word-proof.png"), &doc, 0, 144)?;
                Ok::<_, anyhow::Error>(())
            }
        })
        .await?;
    window.update(cx, |s, _, cx| {
        s.close_popup();
        let before = s.editor.document.clone();
        s.switch_document_mode(DocumentMode::Compose);
        ensure!(
            !s.word_mode(),
            "Word document did not switch back to Compose"
        );
        s.undo(false);
        ensure!(s.editor.document == before, "Mode change was not undoable");
        s.undo(true);
        s.insert_at(Tool::Shape, DocPoint { x: 40., y: 50. });
        let native = s.editor.document.clone();
        s.switch_document_mode(DocumentMode::Word);
        ensure!(
            s.editor.document == native,
            "Rejected conversion destroyed a canvas layer"
        );
        let original = dir.join("must-remain.docx");
        std::fs::write(&original, b"original")?;
        ensure!(
            output::write(&original, OutputFormat::Word, &native, 0, 144).is_err(),
            "Compose document exported to Word"
        );
        ensure!(
            std::fs::read(&original)? == b"original",
            "Rejected native export truncated an existing file"
        );
        s.request(Pending::NewWord, cx);
        ensure!(
            s.creation.is_some() && s.editor.document == native,
            "New-document chooser changed the working document"
        );
        s.create_from_preset(cx);
        ensure!(
            s.pending.is_some(),
            "New Word document bypassed unsaved changes"
        );
        s.pending = None;
        s.replace_document(doc.clone(), None);
        s.saved = doc.clone();
        s.path = Some(dir.join("word-proof.compose"));
        s.tool = Tool::Select;
        s.toggle_popup(Popup::Modes);
        s.set_status("Word mode · export editable .docx from Export", false);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    println!(
        "DOCUMENT_MODES_PASS: Word/Compose switching, reading-order reflow, blocked canvas effects/shapes, rich text and embedded image DOCX, persisted mode, undo/redo, rejected export preserves destination, unsaved new-document guard"
    );
    Ok(())
}
