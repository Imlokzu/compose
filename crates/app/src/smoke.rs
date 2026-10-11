use anyhow::{Result, ensure};
use document::{Document, NodeKind, Rect};
use text::RichText;
pub fn run() -> Result<()> {
    let mut editor = editor::Editor::new(Document::default());
    let mut rich = RichText::new("Compose — small and BIG\nUnicode: café 👩‍💻 مرحبا");
    let mut big = rich.style.clone();
    big.size = 34.;
    big.weight = 700.;
    rich.apply_style(22..25, big);
    editor.insert(NodeKind::Text(rich), Rect::new(48., 64., 320., 140.));
    let mut engine = renderer::TextEngine::new();
    if let NodeKind::Text(t) = &editor.selected_node().unwrap().kind {
        ensure!(engine.layout(t, 320.).height > 0.);
    }
    editor.checkpoint();
    editor.selected_node_mut().unwrap().frame.x = 72.;
    ensure!(editor.undo());
    ensure!(editor.selected_node().unwrap().frame.x == 48.);
    ensure!(editor.redo());
    let dir = tempfile::tempdir()?;
    let image_path = dir.path().join("image.png");
    image::RgbaImage::from_pixel(24, 16, image::Rgba([100, 150, 80, 255])).save(&image_path)?;
    let bytes = std::fs::read(&image_path)?;
    let (_, w, h) = crate::workspace::Workspace::decode_image(&bytes)?;
    ensure!((w, h) == (24, 16));
    editor.checkpoint();
    editor
        .document
        .assets
        .insert("testimage".into(), std::sync::Arc::new(bytes));
    editor.document.insert(
        0,
        NodeKind::Image {
            asset: "testimage".into(),
        },
        Rect::new(120., 240., 240., 160.),
    );
    editor.insert(NodeKind::Shape, Rect::new(64., 480., 180., 80.));
    let path = dir.path().join("smoke.compose");
    file_format::save(&path, &editor.document)?;
    let loaded = file_format::load(&path)?;
    ensure!(loaded == editor.document, "Roundtrip changed the document");
    println!(
        "PASS: Parley layout, rich text, image decode, geometry, undo/redo, .compose save/reopen"
    );
    Ok(())
}

/// Exercise GPUI's event dispatch and platform input handler without moving the
/// real pointer, changing the clipboard, or sending keystrokes to another app.
static NATIVE_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn failed() -> bool {
    NATIVE_FAILED.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn native(window: gpui::WindowHandle<crate::workspace::Workspace>, cx: &mut gpui::App) {
    use gpui::*;
    let any: AnyWindowHandle = window.into();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(std::time::Duration::from_millis(700)).await;
        let result = async {
            run_native(window, cx)?;
            if std::env::args().any(|a| a == "--font-weights") {
                let faces = window.update(cx, |s, w, _| s.engine.verify_native_faces(w))??;
                println!("FONT_FACES_PASS: {faces} distinct native faces match Parley glyph indices");
                crate::text_menu_smoke::font_weights(window, cx)?;
            }
            if std::env::args().any(|a| a == "--refinements" || a == "--polish") {
                crate::refinement_smoke::run(window, cx)?;
            }
            if std::env::args().any(|a| a == "--polish") {
                crate::polish_smoke::run(window, cx)?;
            }
            if std::env::args().any(|a| a == "--workflows") {
                crate::workflow_smoke::run(window, cx)?;
                crate::workflow_smoke::background_replacement(window, cx).await?;
            }
            if std::env::args().any(|a| a == "--text-menu") {
                crate::text_menu_smoke::run(window, cx)?;
            }
            if std::env::args().any(|a| a == "--output") {
                crate::output_smoke::run(window, cx).await?;
            }
            if std::env::args().any(|a| a == "--selection") {
                crate::selection_smoke::run(window, cx).await?;
            }
            if std::env::args().any(|a| a == "--touchpad") {
                crate::zoom_smoke::run(window, cx)?;
            }
            if std::env::args().any(|a| a == "--document-modes") {
                crate::document_mode_smoke::run(window,cx).await?;
            }
            if std::env::args().any(|a| a == "--creation") {
                crate::creation_smoke::run(window,cx).await?;
            }
            if std::env::args().any(|a| a == "--presets") {
                crate::preset_smoke::run(window,cx).await?;
            }
            if std::env::args().any(|a|a=="--actions"){crate::actions_smoke::run(window,cx).await?;}
            if std::env::args().any(|a|a=="--shared-service"||a=="--shared-service-live"){crate::shared_service_smoke::run(window,cx).await?;}
            Ok::<_, anyhow::Error>(())
        }.await;
        match result {
            Ok(()) => println!("NATIVE_SMOKE_PASS: key dispatch, UTF-16/IME, formatting, inspector typing, text/image drag and resize, undo/redo, zoom, save/reopen"),
            Err(error) => { NATIVE_FAILED.store(true,std::sync::atomic::Ordering::Relaxed);eprintln!("NATIVE_SMOKE_FAIL: {error:#}"); }
        }
        any.update(cx,|_,window,_|window.refresh()).ok();
        cx.background_executor().timer(std::time::Duration::from_millis(350)).await;
        window.update(cx,|app,window,cx|{println!("NATIVE_FINAL_STATE: {} / {} layers / popup {}",app.editor.document.name,app.editor.document.pages[app.editor.page].nodes.len(),app.popup.is_some());cx.notify();window.refresh();}).ok();
        if !std::env::args().any(|a|a=="--ui-smoke-hold") {
            cx.background_executor().timer(std::time::Duration::from_millis(800)).await;
            any.update(cx,|_,_,cx|cx.quit()).ok();
        }
    }).detach();
}

fn run_native(
    window: gpui::WindowHandle<crate::workspace::Workspace>,
    cx: &mut gpui::AsyncApp,
) -> Result<()> {
    use gpui::*;
    let any: AnyWindowHandle = window.into();

    let origin = window.update(cx, |app, _, _| app.page_origin)?;
    let zoom = window.update(cx, |app, _, _| app.zoom)?;
    let pos = origin + point(px(48. * zoom), px(70. * zoom));
    any.update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position: pos,
                click_count: 2,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position: pos,
                click_count: 2,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    ensure!(
        window.update(cx, |app, _, _| app.editing)?,
        "Double-click did not enter text editing"
    );
    any.update(cx, |_, w, cx| {
        for ch in "Write freely.".chars() {
            w.dispatch_keystroke(
                Keystroke {
                    modifiers: Modifiers::default(),
                    key: ch.to_string(),
                    key_char: Some(ch.to_string()),
                },
                cx,
            );
        }
        w.draw(cx).clear();
    })?;
    ensure!(
        window.update(cx, |app, _, _| app.text().map(|t| t.content.clone()))?
            == Some("Write freely.".into()),
        "Native text dispatch lost characters"
    );
    window.update(cx, |app, w, cx| {
        app.replace_text_in_range(None, "👩‍💻", w, cx);
        ensure!(
            app.selected_text_range(false, w, cx).unwrap().range.end == 18,
            "UTF-16 cursor mismatch after emoji"
        );
        app.replace_and_mark_text_in_range(Some(13..18), "か", Some(1..1), w, cx);
        ensure!(
            app.marked_text_range(w, cx) == Some(13..14),
            "IME marked range mismatch"
        );
        app.replace_text_in_range(None, "漢", w, cx);
        ensure!(
            app.text().unwrap().content == "Write freely.漢",
            "IME commit replaced the wrong bytes"
        );
        app.replace_text_in_range(Some(13..14), "", w, cx);
        Ok::<_, anyhow::Error>(())
    })??;
    any.update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse("cmd-a").unwrap(), cx);
        w.dispatch_keystroke(Keystroke::parse("cmd-b").unwrap(), cx);
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, cx| {
        ensure!(
            app.text().unwrap().style_at(0).weight == 700.,
            "Bold did not apply to selection"
        );
        app.format(|s| s.size = 36.);
        app.begin_field(crate::workspace::Field::W, "400".into());
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    any.update(cx, |_, w, cx| {
        w.draw(cx).clear();
        for ch in "360".chars() {
            w.dispatch_keystroke(
                Keystroke {
                    modifiers: Modifiers::default(),
                    key: ch.to_string(),
                    key_char: Some(ch.to_string()),
                },
                cx,
            );
        }
        w.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, cx| {
        ensure!(
            app.editor.selected_node().unwrap().frame.width == 360.,
            "Inspector keyboard editing failed"
        );
        app.end_edit();
        app.measure();
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    any.update(cx, |_, w, cx| {
        w.draw(cx).clear();
    })?;
    let (start, old) = window.update(cx, |app, _, _| {
        let r = app.editor.selected_node().unwrap().frame;
        (
            app.screen_rect(Rect::new(r.x + 20., r.y + 15., 1., 1.))
                .origin,
            r,
        )
    })?;
    let moved = start + point(px(25. * zoom), px(30. * zoom));
    any.update(cx, |_, w, cx| {
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
                position: moved,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers {
                    alt: true,
                    ..Default::default()
                },
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: moved,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, _| {
        let r = app.editor.selected_node().unwrap().frame;
        ensure!(
            (r.x - old.x - 25.).abs() < 0.1 && (r.y - old.y - 30.).abs() < 0.1,
            "Canvas drag failed: {r:?}"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    let (start, old) = window.update(cx, |app, _, _| {
        let r = app.editor.selected_node().unwrap().frame;
        (
            app.screen_rect(Rect::new(r.x + r.width, r.y + r.height / 2., 1., 1.))
                .origin,
            r,
        )
    })?;
    let moved = start + point(px(40. * zoom), px(0.));
    any.update(cx, |_, w, cx| {
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
                position: moved,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::default(),
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: moved,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, _| {
        ensure!(
            (app.editor.selected_node().unwrap().frame.width - old.width - 40.).abs() < 0.1,
            "Resize handle failed"
        );
        app.undo(false);
        ensure!(
            (app.editor.selected_node().unwrap().frame.width - old.width).abs() < 0.1,
            "Resize undo failed"
        );
        app.undo(true);
        Ok::<_, anyhow::Error>(())
    })??;
    // Fixture remains editable; each object is an actual model node.
    window.update(cx, |app, _, cx| {
        let title = app.editor.selected().unwrap();
        app.editor.document.node_mut(title).unwrap().frame.x = 48.;
        app.editor.document.node_mut(title).unwrap().frame.y = 70.;
        app.editor.document.node_mut(title).unwrap().name = "Title".into();
        app.insert_at(
            crate::workspace::Tool::Text,
            document::Point { x: 48., y: 126. },
        );
        app.replace_input(0..0, "Design precisely.");
        app.end_edit();
        app.format(|s| {
            s.size = 34.;
            s.family = "Noto Serif".into();
            s.color = document::Color::rgb(114, 130, 101)
        });
        app.end_edit();
        app.insert_at(
            crate::workspace::Tool::Text,
            document::Point { x: 48., y: 204. },
        );
        app.replace_input(
            0..0,
            "A little room for your next big idea.\nWrite, arrange, and make it your own.",
        );
        app.end_edit();
        app.format(|s| {
            s.size = 16.;
            s.line_height = 1.5;
        });
        app.end_edit();
        let asset =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../brand/logo-original.jpg");
        app.import_image(&asset)?;
        let n = app.editor.selected_node_mut().unwrap();
        n.frame = Rect::new(48., 302., 248., 248.);
        n.radius = 8.;
        n.name = "Compose mark".into();
        app.insert_at(
            crate::workspace::Tool::Shape,
            document::Point { x: 328., y: 302. },
        );
        let n = app.editor.selected_node_mut().unwrap();
        n.frame = Rect::new(328., 302., 216., 248.);
        n.fill = document::Color::rgb(211, 222, 201);
        n.radius = 8.;
        n.name = "Sage block".into();
        app.insert_at(
            crate::workspace::Tool::Text,
            document::Point { x: 350., y: 330. },
        );
        app.replace_input(0..0, "01\nYour words.\nYour layout.");
        app.end_edit();
        app.format(|s| {
            s.size = 22.;
            s.weight = 700.;
        });
        app.editor.selected_node_mut().unwrap().frame.width = 174.;
        app.end_edit();
        app.insert_at(
            crate::workspace::Tool::Text,
            document::Point { x: 48., y: 600. },
        );
        app.replace_input(0..0, "EVERYTHING HERE IS EDITABLE");
        app.end_edit();
        app.format(|s| {
            s.size = 10.;
            s.letter_spacing = 1.5;
            s.color = document::Color::rgb(114, 130, 101);
        });
        app.end_edit();
        app.measure();
        app.editor.document.name = "First composition".into();
        for n in &mut app.editor.document.pages[0].nodes {
            if n.name == "Text"
                && let NodeKind::Text(t) = &n.kind
            {
                n.name = t
                    .content
                    .lines()
                    .next()
                    .unwrap_or("Text")
                    .chars()
                    .take(30)
                    .collect();
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts");
        std::fs::create_dir_all(&dir)?;
        let path = if std::env::args().any(|a| a == "--refinements" || a == "--polish") {
            let checks = dir.join("refinement-checks");
            std::fs::create_dir_all(&checks)?;
            checks.join("base.compose")
        } else {
            dir.join("native-smoke.compose")
        };
        file_format::save(&path, &app.editor.document)?;
        let loaded = file_format::load(&path)?;
        ensure!(
            loaded == app.editor.document,
            "Native edited document did not round-trip"
        );
        app.saved = loaded;
        app.path = Some(path);
        app.editor.select_only(Some(title));
        app.set_status("Ready", false);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    any.update(cx, |_, w, cx| {
        w.draw(cx).clear();
    })?;
    let title = window.update(cx, |app, _, _| app.editor.selected())?;
    let image_id = window.update(cx, |app, _, _| {
        app.editor.document.pages[0]
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Image { .. }))
            .unwrap()
            .id
    })?;
    let (start, old) = window.update(cx, |app, _, _| {
        let r = app.editor.document.node(image_id).unwrap().frame;
        (
            app.screen_rect(Rect::new(r.x + r.width / 2., r.y + r.height / 2., 1., 1.))
                .origin,
            r,
        )
    })?;
    let moved = start + point(px(25. * zoom), px(20. * zoom));
    any.update(cx, |_, w, cx| {
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
                position: moved,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers {
                    alt: true,
                    ..Default::default()
                },
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: moved,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, _| {
        ensure!(
            app.editor.selected() == Some(image_id),
            "Image hit-test failed"
        );
        ensure!(
            (app.editor.document.node(image_id).unwrap().frame.x - old.x - 25.).abs() < 0.1,
            "Image drag failed"
        );
        app.undo(false);
        Ok::<_, anyhow::Error>(())
    })??;
    window.update(cx, |app, _, cx| {
        app.editor.select_only(Some(image_id));
        cx.notify();
    })?;
    any.update(cx, |_, w, cx| {
        w.draw(cx).clear();
    })?;
    let start = window.update(cx, |app, _, _| {
        app.screen_rect(Rect::new(old.x + old.width, old.y + old.height, 1., 1.))
            .origin
    })?;
    let moved = start + point(px(30. * zoom), px(30. * zoom));
    any.update(cx, |_, w, cx| {
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
                position: moved,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers {
                    shift: true,
                    ..Default::default()
                },
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: moved,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, _| {
        let r = app.editor.document.node(image_id).unwrap().frame;
        ensure!(
            (r.width - old.width - 30.).abs() < 0.1 && (r.width - r.height).abs() < 0.1,
            "Image proportional resize failed"
        );
        app.undo(false);
        Ok::<_, anyhow::Error>(())
    })??;
    let before = window.update(cx, |app, _, _| app.to_doc(start))?;
    any.update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position: start,
                delta: ScrollDelta::Pixels(point(px(0.), px(30.))),
                modifiers: Modifiers {
                    control: true,
                    ..Default::default()
                },
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |app, _, cx| {
        let after = app.to_doc(start);
        ensure!(
            (before.x - after.x).abs() < 0.05 && (before.y - after.y).abs() < 0.05,
            "Zoom moved the pointer's document anchor"
        );
        app.fit();
        app.editor.select_only(title);
        app.set_status("Ready", false);
        ensure!(!app.dirty(), "Gesture undo did not restore saved state");
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    Ok(())
}
