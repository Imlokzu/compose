use crate::{
    output::{self, OutputFormat},
    workspace::{Popup, Workspace},
};
use anyhow::{Result, ensure};
use document::{Color, Document, NodeKind, Rect};
use gpui::*;
use std::{path::PathBuf, sync::Arc};

pub fn example_document() -> Result<Document> {
    let mut doc = Document {
        name: "Output proof".into(),
        ..Default::default()
    };
    let heading = doc.insert(
        0,
        NodeKind::Text(text::RichText::new("Made to leave\nthe canvas.")),
        Rect::new(48., 64., 499., 160.),
    );
    if let NodeKind::Text(t) = &mut doc.node_mut(heading).unwrap().kind {
        t.style.size = 52.;
        t.style.weight = 900.;
    }
    let mut copy = text::RichText::new(
        "A real document. A ready-to-share image.\nYour fonts, colors and layout — kept together.",
    );
    copy.style.size = 18.;
    copy.style.line_height = 1.5;
    let mut strong = copy.style.clone();
    strong.weight = 700.;
    strong.color = Color::rgb(46, 113, 83);
    copy.apply_style(0..16, strong);
    doc.insert(0, NodeKind::Text(copy), Rect::new(48., 250., 499., 84.));
    let bytes = include_bytes!("../../../brand/reference-ui.jpg").to_vec();
    doc.assets.insert("proof-image".into(), Arc::new(bytes));
    let image = doc.insert(
        0,
        NodeKind::Image {
            asset: "proof-image".into(),
        },
        Rect::new(48., 366., 499., 280.),
    );
    doc.node_mut(image).unwrap().radius = 18.;
    doc.node_mut(image).unwrap().appearance.shadow = true;
    let stripe = doc.insert(0, NodeKind::Shape, Rect::new(48., 699., 499., 4.));
    doc.node_mut(stripe).unwrap().fill = Color::rgb(46, 113, 83);
    let mut footer =
        text::RichText::new("COMPOSE     /     WRITE LIKE A DOCUMENT. DESIGN LIKE A CANVAS.");
    footer.style.size = 10.;
    footer.style.letter_spacing = 1.;
    doc.insert(0, NodeKind::Text(footer), Rect::new(48., 724., 499., 30.));
    let mut next = doc.pages[0].clone();
    next.id = doc.next_id;
    doc.next_id += 1;
    next.name = "Page 2".into();
    next.nodes.clear();
    doc.pages.push(next);
    let mut text = text::RichText::new(
        "Second page\n\nLight   Regular   Bold   Black\n\nCafé · Українська · مرحبا",
    );
    text.style.size = 30.;
    text.style.weight = 300.;
    for (range, weight) in [(22..29, 400.), (32..36, 700.), (39..44, 900.)] {
        if range.end <= text.content.len() {
            let mut s = text.style.clone();
            s.weight = weight;
            text.apply_style(range, s);
        }
    }
    doc.insert(1, NodeKind::Text(text), Rect::new(48., 64., 499., 350.));
    doc.validate().map_err(anyhow::Error::msg)?;
    Ok(doc)
}
fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/output")
}

pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let dir = directory();
    std::fs::create_dir_all(&dir)?;
    let doc = example_document()?;
    window.update(cx, |s, _, cx| {
        s.replace_document(doc.clone(), None);
        s.zoom = 0.7;
        s.fitted = true;
        s.toggle_popup(Popup::Export);
        cx.notify();
    })?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())?;
    let point = window
        .update(cx, |s, _, _| {
            s.output_resolution_bounds.get(&300).map(|b| b.center())
        })?
        .ok_or_else(|| anyhow::anyhow!("Resolution choices did not render"))?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: point,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: point,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    // Selecting a resolution must never format the selected text or mutate history.
    window.update(cx, |s, _, _| {
        ensure!(s.popup == Some(Popup::Export), "Export menu disappeared");
        ensure!(
            s.export_dpi == 300,
            "Native resolution button did not respond"
        );
        ensure!(
            s.editor.document == doc,
            "Export controls changed the document"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    let task = cx.background_executor().spawn({
        let doc = doc.clone();
        let dir = dir.clone();
        async move {
            output::write(
                &dir.join("proof.compose"),
                OutputFormat::Compose,
                &doc,
                0,
                300,
            )?;
            ensure!(
                file_format::load(&dir.join("proof.compose"))? == doc,
                "Exported native copy changed the document"
            );
            output::write(&dir.join("proof.png"), OutputFormat::Png, &doc, 0, 300)?;
            output::write(&dir.join("page-2.png"), OutputFormat::Png, &doc, 1, 144)?;
            #[cfg(target_os = "macos")]
            {
                let job = crate::printing::prepare(&doc)?;
                ensure!(
                    job.save_pdf_for_test(&dir.join("print-proof.pdf"))?,
                    "Print operation failed"
                );
                if std::env::args().any(|a| a == "--check-print-dialog") {
                    ensure!(job.check_dialog()?, "Print panel check failed");
                }
            }
            Ok::<_, anyhow::Error>(())
        }
    });
    task.await?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.document == doc && s.path.is_none() && !s.dirty(),
            "Export changed editor state"
        );
        s.close_popup();
        s.popup = Some(Popup::Export);
        s.set_status("Export checked · Compose, PNG and native printing", false);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    println!(
        "OUTPUT_PASS: native export menu/resolution, unchanged document state, .compose reopen, page-only PNG with DPI, two-page native print output"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{OutputFormat, output};
    #[test]
    fn export_destination_cannot_overwrite_an_unconfirmed_extension() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("image.png");
        std::fs::write(&png, b"original").unwrap();
        assert!(output::destination(dir.path().join("image"), OutputFormat::Png).is_err());
        assert!(output::destination(dir.path().join("image.jpg"), OutputFormat::Png).is_err());
        assert_eq!(
            output::destination(png.clone(), OutputFormat::Png).unwrap(),
            png
        );
        assert_eq!(std::fs::read(&png).unwrap(), b"original");
    }
}
