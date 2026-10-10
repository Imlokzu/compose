//! Explicit native/live integration check, never part of the offline unit suite.
use crate::{
    image_picker::ImageTab,
    workspace::{Field, Workspace},
};
use anyhow::{Result, ensure};
use gpui::*;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
static FAILED: AtomicBool = AtomicBool::new(false);
pub fn failed() -> bool {
    FAILED.load(Ordering::Relaxed)
}
pub fn start(window: WindowHandle<Workspace>, cx: &mut App) {
    cx.spawn(async move|cx|{
  cx.background_executor().timer(Duration::from_millis(700)).await;
  match run(window,cx).await{Ok(())=>println!("IMAGE_PICKER_PASS: native tabs and web search input, real Bing results/thumbnails/original, insertion, source metadata, undo, save/reopen, local file import, rejected invalid image, cancellation"),Err(e)=>{FAILED.store(true,Ordering::Relaxed);eprintln!("IMAGE_PICKER_FAIL: {e:#}");}}
  window.update(cx,|_,w,cx|{cx.notify();w.refresh();}).ok();
  if !std::env::args().any(|a|a=="--image-picker-hold"){cx.background_executor().timer(Duration::from_millis(750)).await;AnyWindowHandle::from(window).update(cx,|_,_,cx|cx.quit()).ok();}
 }).detach();
}
fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}
fn click(window: WindowHandle<Workspace>, cx: &mut AsyncApp, p: Point<Pixels>) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: p,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: p,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
async fn until(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    ready: impl Fn(&Workspace) -> bool,
) -> Result<()> {
    let start = Instant::now();
    loop {
        if window.update(cx, |s, _, _| ready(s))? {
            return Ok(());
        }
        ensure!(
            start.elapsed() < Duration::from_secs(50),
            "Native image operation timed out"
        );
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;
    }
}
async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("Local photo.png");
    image::RgbaImage::from_pixel(300, 180, image::Rgba([70, 135, 99, 255])).save(&local)?;
    window.update(cx, |s, _, cx| {
        s.open_image_picker(cx);
        s.load_local_image(local.clone(), cx);
    })?;
    until(window, cx, |s| !s.image_picker.inserting).await?;
    window.update(cx, |s, _, cx| {
        ensure!(s.popup.is_none(), "Local import left modal open");
        ensure!(
            matches!(
                s.editor.selected_node().map(|n| &n.kind),
                Some(document::NodeKind::Image { .. })
            ),
            "Local image was not inserted"
        );
        s.undo(false);
        s.open_image_picker(cx);
        Ok::<_, anyhow::Error>(())
    })??;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    let invalid = dir.path().join("not-image.png");
    std::fs::write(&invalid, "not an image")?;
    window.update(cx, |s, _, cx| s.load_local_image(invalid, cx))?;
    until(window, cx, |s| !s.image_picker.inserting).await?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.image_picker.error.is_some() && s.editor.document == before,
            "Invalid local import changed the document or hid its error"
        );
        s.image_picker.error = None;
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let (left, top) = window.update(cx, |s, w, _| {
        let h = (f32::from(s.viewport.size.height) + 28.).clamp(500., 620.);
        (
            (f32::from(w.bounds().size.width) - 740.) / 2.,
            (f32::from(s.viewport.size.height) + 75. - h) / 2.,
        )
    })?;
    click(window, cx, point(px(left + 205.), px(top + 78.)))?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.image_picker.tab == ImageTab::Search,
            "Native Search online tab did not switch"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    click(window, cx, point(px(left + 120.), px(top + 130.)))?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.field
                .as_ref()
                .is_some_and(|f| f.field == Field::ImageSearch),
            "Search field did not receive focus"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        for c in "pixel art cat".chars() {
            w.dispatch_keystroke(
                Keystroke {
                    modifiers: Modifiers::default(),
                    key: c.to_string(),
                    key_char: Some(c.to_string()),
                },
                cx,
            );
        }
        w.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
    })?;
    until(window, cx, |s| !s.image_picker.loading).await?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.image_picker.error.is_none(),
            "Live search failed: {:?}",
            s.image_picker.error
        );
        ensure!(
            !s.image_picker.results.is_empty(),
            "Live search returned no supported images"
        );
        ensure!(
            s.image_picker.provider == image_search::Provider::Web
                && s.image_picker
                    .results
                    .iter()
                    .any(|r| !r.item.source_url.contains("wikimedia.org")),
            "Web search returned only the Commons catalog"
        );
        ensure!(
            s.image_picker.searched == "pixel art cat",
            "Search text was not submitted"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    until(window, cx, |s| {
        s.image_picker
            .results
            .first()
            .is_some_and(|r| r.thumbnail.is_some() || r.thumbnail_failed)
    })
    .await?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.image_picker.results[0].thumbnail.is_some(),
            "First live thumbnail failed to load"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    // Select first real result, then click its Insert button in the footer.
    click(window, cx, point(px(left + 110.), px(top + 244.)))?;
    let chosen = window.update(cx, |s, _, _| {
        ensure!(
            s.image_picker.selected.is_some(),
            "Image tile did not select"
        );
        Ok::<_, anyhow::Error>(s.image_picker.selected_result().unwrap().item.clone())
    })??;
    let bottom = window.update(cx, |s, _, _| {
        let h = (f32::from(s.viewport.size.height) + 28.).clamp(500., 620.);
        (f32::from(s.viewport.size.height) + 75. + h) / 2.
    })?;
    click(window, cx, point(px(left + 664.), px(bottom - 44.)))?;
    until(window, cx, |s| !s.image_picker.inserting).await?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.popup.is_none(),
            "Insert failed or footer button missed: {:?}",
            s.image_picker.error
        );
        let node = s
            .editor
            .selected_node()
            .ok_or_else(|| anyhow::anyhow!("No inserted object"))?;
        let source = node
            .image_source
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Attribution was not persisted"))?;
        ensure!(
            source.author == chosen.author
                && source.page_url == chosen.source_url
                && source.license == chosen.license,
            "Source metadata changed"
        );
        let actual_ratio = s
            .image_ratio()
            .ok_or_else(|| anyhow::anyhow!("No decoded image dimensions"))?;
        ensure!(
            (node.frame.width / node.frame.height - actual_ratio).abs() < 0.01,
            "Image was stretched during insertion"
        );
        let document = s.editor.document.clone();
        let saved = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts/image-picker/imported.compose");
        file_format::save(&saved, &document)?;
        ensure!(
            file_format::load(&saved)? == document,
            "Downloaded image did not reopen identically"
        );
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Image insertion was not one undo step"
        );
        s.undo(true);
        ensure!(s.editor.document == document, "Image redo failed");
        s.saved = document;
        s.path = Some(saved);
        s.open_image_picker(cx);
        s.image_picker.query = "space".into();
        s.search_images(false, cx);
        let generation = s.image_picker.generation;
        s.close_popup();
        ensure!(
            !s.image_picker.accepts(generation),
            "Closed picker still accepts old requests"
        );
        s.open_image_picker(cx);
        s.image_picker.query = "pixel art cat".into();
        s.search_images(false, cx);
        Ok::<_, anyhow::Error>(())
    })??;
    until(window, cx, |s| !s.image_picker.loading).await?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.image_picker.error.is_none(),
            "Reopened search failed: {:?}",
            s.image_picker.error
        );
        ensure!(
            s.image_picker.searched == "pixel art cat",
            "Old search replaced the new request"
        );
        s.image_picker.selected = s.image_picker.results.first().map(|r| r.item.id);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    until(window, cx, |s| {
        s.image_picker
            .results
            .iter()
            .take(6)
            .all(|r| r.thumbnail.is_some() || r.thumbnail_failed)
    })
    .await?;
    window.update(cx, |s, _, cx| {
        let before = s.editor.document.clone();
        s.image_picker.selected = s
            .image_picker
            .results
            .iter()
            .find(|r| r.thumbnail_bytes.is_some())
            .map(|r| r.item.id);
        ensure!(
            s.image_picker.selected.is_some(),
            "No cached preview to insert"
        );
        s.insert_search_preview(cx);
        ensure!(
            s.popup.is_none()
                && s.editor
                    .selected_node()
                    .unwrap()
                    .image_source
                    .as_ref()
                    .unwrap()
                    .credit
                    .starts_with("Search preview"),
            "Preview insertion did not preserve its source/quality label"
        );
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Preview insertion was not one undo step"
        );
        s.open_image_picker(cx);
        s.image_picker.query = "forest".into();
        s.set_image_provider(image_search::Provider::Commons, cx);
        Ok::<_, anyhow::Error>(())
    })??;
    until(window, cx, |s| !s.image_picker.loading).await?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.image_picker.error.is_none()
                && !s.image_picker.results.is_empty()
                && s.image_picker
                    .results
                    .iter()
                    .all(|r| r.item.provider == image_search::Provider::Commons),
            "Commons source switch failed"
        );
        s.image_picker.query = "pixel art cat".into();
        s.set_image_provider(image_search::Provider::Web, cx);
        Ok::<_, anyhow::Error>(())
    })??;
    until(window, cx, |s| !s.image_picker.loading).await?;
    until(window, cx, |s| {
        s.image_picker
            .results
            .iter()
            .take(6)
            .all(|r| r.thumbnail.is_some() || r.thumbnail_failed)
    })
    .await?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.image_picker.error.is_none()
                && !s.image_picker.results.is_empty()
                && s.image_picker
                    .results
                    .iter()
                    .all(|r| r.item.provider == image_search::Provider::Web),
            "Switching back to full web search failed"
        );
        s.image_picker.selected = s.image_picker.results.first().map(|r| r.item.id);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    Ok(())
}
