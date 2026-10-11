//! Explicit native checks against a controlled local relay or the configured live service.
use crate::{
    creation_smoke::{click, draw},
    workspace::Workspace,
};
use anyhow::{Context, Result, ensure};
use document::{Document, NodeKind};
use gpui::*;
use std::{
    io::Cursor,
    time::{Duration, Instant},
};
pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let live = std::env::args().any(|a| a == "--shared-service-live");
    let endpoint = image_tools::configured_service_url()?
        .context("Shared-service check requires a configured public URL or loopback fixture")?;
    ensure!(
        if live {
            endpoint.starts_with("https://")
        } else {
            endpoint.starts_with("http://127.0.0.1:")
        },
        "Use --shared-service with a loopback fixture, or explicitly opt into HTTPS with --shared-service-live"
    );
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.background_service = Some(image_tools::validate_service_url(&endpoint)?);
        let bytes = if live {
            include_bytes!("../../../assets/presets/portrait.png").to_vec()
        } else {
            let source = image::RgbaImage::from_pixel(100, 100, image::Rgba([180, 190, 185, 255]));
            let mut bytes = Cursor::new(Vec::new());
            source.write_to(&mut bytes, image::ImageFormat::Png)?;
            bytes.into_inner()
        };
        let (image, w, h) = Workspace::decode_image(&bytes)?;
        s.insert_decoded_image(bytes, image, w, h, "Test subject".into(), None);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let original = window.update(cx, |s, _, _| s.editor.document.clone())?;
    click(window, cx, "remove-background")?;
    let started = Instant::now();
    while window.update(cx, |s, _, _| s.cutout_target.is_some())? {
        ensure!(
            started.elapsed() < Duration::from_secs(if live { 210 } else { 8 }),
            "Shared service did not complete"
        );
        cx.background_executor()
            .timer(Duration::from_millis(30))
            .await;
    }
    let doc = window.update(cx, |s, _, cx| {
        ensure!(
            s.popup.is_none() && s.field.is_none(),
            "Shared cutout asked for credentials"
        );
        let node = s.editor.selected_node().unwrap();
        let id = node.id;
        let NodeKind::Image { asset } = &node.kind else {
            unreachable!()
        };
        let pixels = image::load_from_memory(&s.editor.document.assets[asset])?.into_rgba8();
        ensure!(
            pixels.pixels().any(|p| p.0[3] == 0) && pixels.pixels().any(|p| p.0[3] == 255),
            "Shared service did not apply the transparent PNG"
        );
        ensure!(
            node.frame == original.node(id).unwrap().frame
                && node.radius == original.node(id).unwrap().radius,
            "Cutout changed geometry/style"
        );
        let result = s.editor.document.clone();
        s.undo(false);
        ensure!(
            s.editor.document == original,
            "Shared cutout was not one undo step"
        );
        s.undo(true);
        ensure!(s.editor.document == result, "Shared cutout redo failed");
        cx.notify();
        Ok::<_, anyhow::Error>(result)
    })??;
    let mut dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/shared-service");
    if live {
        dir = dir.join("live");
    }
    std::fs::create_dir_all(&dir)?;
    if live {
        let NodeKind::Image { asset } = &doc.pages[0].nodes[0].kind else {
            unreachable!()
        };
        std::fs::write(dir.join("cutout.png"), doc.assets[asset].as_slice())?;
    }
    file_format::save(&dir.join("transparent.compose"), &doc)?;
    ensure!(
        file_format::load(&dir.join("transparent.compose"))? == doc,
        "Shared cutout did not survive reopening"
    );
    export::write_png(&dir.join("page.png"), &doc, 0, 144)?;
    if live {
        println!(
            "LIVE_SHARED_SERVICE_PASS: packaged native Remove button → {endpoint} → real provider PNG, no key/account fields, transparent asset, unchanged bounds/style, one-step undo/redo and exact native save/reopen"
        );
    } else {
        println!(
            "SHARED_SERVICE_PASS: actual native Remove button → real local Rust relay → controlled provider PNG, no key/account fields, transparent asset, unchanged bounds/style, one-step undo/redo and exact native save/reopen"
        );
    }
    Ok(())
}
