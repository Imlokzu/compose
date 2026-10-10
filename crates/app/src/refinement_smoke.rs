//! Native event regressions for image scaling, corner rounding and selected text.
use crate::workspace::{Field, Tool, Workspace};
use anyhow::{Result, ensure};
use document::{Document, ImageSizing, NodeKind, Point as DocPoint, Rect};
use gpui::*;

fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}
fn pointer(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    start: Point<Pixels>,
    end: Point<Pixels>,
    count: usize,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                click_count: count,
                ..Default::default()
            }),
            cx,
        );
        if start != end {
            w.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: end,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Modifiers::default(),
                }),
                cx,
            );
        }
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                click_count: count,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
fn edit_field(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    field: Field,
    value: &str,
) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.begin_field(field, String::new());
        cx.notify();
    })?;
    draw(window, cx)?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        for ch in value.chars() {
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
    })
}
fn text_point(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    index: usize,
) -> Result<Point<Pixels>> {
    window.update(cx, |s, _, _| {
        let n = s.absolute_frame(s.editor.selected().unwrap()).unwrap();
        let c = s.layout().unwrap().caret(index);
        s.screen_rect(Rect::new(n.x + c.x, n.y + c.y + c.height / 2., 1., 1.))
            .origin
    })
}
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 64. });
        s.replace_input(0..0, "Small words, BIG ideas.");
        s.end_edit();
        s.format(|v| v.size = 24.);
        s.editor.selected_node_mut().unwrap().frame.width = 470.;
        s.tool = Tool::Text;
        s.measure();
        cx.notify();
    })?;
    draw(window, cx)?;
    let title = window.update(cx, |s, _, _| s.editor.selected().unwrap())?;
    let start = text_point(window, cx, 13)?;
    let end = text_point(window, cx, 16)?;
    pointer(window, cx, start, end, 1)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.editor.document.pages[0].nodes.len() == 1,
            "Text tool inserted a box over existing text"
        );
        ensure!(s.range() == (13..16), "Mouse selection was {:?}", s.range());
        Ok::<_, anyhow::Error>(())
    })??;
    // Click the actual floating B control. This must not fall through to the page.
    let bold = window.update(cx, |s, _, _| {
        let b = s.text_toolbar_bounds().unwrap();
        b.origin + point(px(212.), px(60.))
    })?;
    pointer(window, cx, bold, bold, 1)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.range() == (13..16),
            "Floating toolbar lost the text selection"
        );
        let t = s.text().unwrap();
        ensure!(
            t.style_at(13).weight == 700.
                && t.style_at(0).weight == 400.
                && t.style_at(16).weight == 400.,
            "Bold escaped the selected range"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    edit_field(window, cx, Field::Size, "40")?;
    edit_field(window, cx, Field::Color, "2878ED")?;
    window.update(cx, |s, _, _| {
        ensure!(s.range() == (13..16), "Inspector lost the selected range");
        let t = s.text().unwrap();
        ensure!(
            t.style_at(13).size == 40. && t.style_at(0).size == 24. && t.style_at(16).size == 24.,
            "Size escaped the selected range"
        );
        ensure!(
            t.style_at(13).color == document::Color::rgb(40, 120, 237)
                && t.style_at(0).color != t.style_at(13).color,
            "Color escaped the range"
        );
        let len = t.content.len();
        s.anchor = 0;
        s.caret = len;
        ensure!(
            s.style_value(Field::Size) == "Mixed",
            "Mixed font sizes were hidden"
        );
        s.anchor = 13;
        s.caret = 16;
        Ok::<_, anyhow::Error>(())
    })??;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(
            Keystroke {
                modifiers: Modifiers::default(),
                key: "WIDE".into(),
                key_char: Some("WIDE".into()),
            },
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.text().unwrap().content == "Small words, WIDE ideas.",
            "Selected-word replacement failed"
        );
        ensure!(
            s.text().unwrap().style_at(13).size == 40.
                && s.text().unwrap().style_at(17).size == 24.,
            "Replacement damaged neighboring styles"
        );
        s.undo(false);
        s.tool = Tool::Select;
        s.measure();
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let word = text_point(window, cx, 14)?;
    pointer(window, cx, word, word, 2)?;
    window.update(cx, |s, _, _| {
        ensure!(s.range() == (13..16), "Double-click did not select a word");
        s.end_edit();
        Ok::<_, anyhow::Error>(())
    })??;
    // Shift-clicking into a different box must not carry an out-of-range anchor.
    let second = window.update(cx, |s, _, cx| {
        // Keep the target clear of the expanded contextual toolbar.
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 360. });
        s.replace_input(0..0, "Hi");
        s.end_edit();
        let second = s.editor.selected().unwrap();
        s.editor.select_only(Some(title));
        s.editing = true;
        s.anchor = 13;
        s.caret = 16;
        s.tool = Tool::Text;
        s.measure();
        cx.notify();
        second
    })?;
    draw(window, cx)?;
    let pos = window.update(cx, |s, _, _| {
        let r = s.absolute_frame(second).unwrap();
        s.screen_rect(Rect::new(r.x + 3., r.y + 10., 0., 0.)).origin
    })?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: pos,
                click_count: 1,
                modifiers: Modifiers {
                    shift: true,
                    ..Default::default()
                },
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: pos,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.editor.selected() == Some(second) && s.range().end <= 2,
            "Shift-click carried the old text anchor"
        );
        s.end_edit();
        s.editor.document.remove(second);
        s.editor.select_only(Some(title));
        s.begin_field(Field::Size, "Mixed".into());
        s.commit_field();
        ensure!(!s.error, "Unchanged mixed field produced an error");
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    // Deliberately non-square pixels make accidental stretching detectable.
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("Color study.png");
    let mut pixels = image::RgbaImage::new(480, 240);
    for (x, y, p) in pixels.enumerate_pixels_mut() {
        let inside = (x as i32 - 240).pow(2) + (y as i32 - 120).pow(2) < 78 * 78;
        *p = image::Rgba(if inside {
            [63, 91, 74, 255]
        } else if x < 160 {
            [217, 228, 208, 255]
        } else if x > 320 {
            [209, 144, 115, 255]
        } else {
            [239, 231, 210, 255]
        });
    }
    pixels.save(&path)?;
    window.update(cx, |s, _, cx| {
        s.import_image(&path)?;
        s.editor.selected_node_mut().unwrap().frame = Rect::new(48., 260., 400., 200.);
        s.set_radius(24.);
        s.tool = Tool::Select;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let image = window.update(cx, |s, _, _| s.editor.selected().unwrap())?;
    let original_bytes = window.update(cx, |s, _, _| {
        let NodeKind::Image { asset } = &s.editor.selected_node().unwrap().kind else {
            unreachable!()
        };
        s.editor.document.assets[asset].clone()
    })?;
    for handle in 0..8 {
        let (start, zoom) = window.update(cx, |s, _, _| {
            let n = s.absolute_frame(image).unwrap();
            let (x, y) = Workspace::handle_points(n)[handle];
            (s.screen_rect(Rect::new(x, y, 0., 0.)).origin, s.zoom)
        })?;
        pointer(
            window,
            cx,
            start,
            start + point(px(19. * zoom), px(27. * zoom)),
            1,
        )?;
        window.update(cx, |s, _, cx| {
            let r = s.editor.selected_node().unwrap().frame;
            ensure!(
                near(r.width / r.height, 2.),
                "Image handle {handle} stretched the image: {r:?}"
            );
            s.undo(false);
            s.editor.select_only(Some(image));
            cx.notify();
            Ok::<_, anyhow::Error>(())
        })??;
        draw(window, cx)?;
    }
    edit_field(window, cx, Field::W, "320")?;
    window.update(cx, |s, _, _| {
        ensure!(
            near(s.editor.selected_node().unwrap().frame.height, 160.),
            "Width did not update proportional height"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    edit_field(window, cx, Field::H, "120")?;
    window.update(cx, |s, _, cx| {
        ensure!(
            near(s.editor.selected_node().unwrap().frame.width, 240.),
            "Height did not update width"
        );
        s.scale_image(1.1);
        let big = s.editor.selected_node().unwrap().frame;
        ensure!(
            near(big.width, 264.) && near(big.height, 132.),
            "Bigger distorted the image"
        );
        s.scale_image(0.9);
        let small = s.editor.selected_node().unwrap().frame;
        ensure!(
            near(small.width, 237.6) && near(small.height, 118.8),
            "Smaller distorted the image"
        );
        s.set_image_sizing(ImageSizing::Stretch);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    edit_field(window, cx, Field::W, "360")?;
    window.update(cx, |s, _, _| {
        ensure!(
            near(s.editor.selected_node().unwrap().frame.height, 118.8),
            "Stretch unexpectedly changed the other dimension"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    let (start, zoom) = window.update(cx, |s, _, _| {
        let r = s.absolute_frame(image).unwrap();
        (
            s.screen_rect(Rect::new(r.x + r.width, r.y + r.height / 2., 0., 0.))
                .origin,
            s.zoom,
        )
    })?;
    pointer(window, cx, start, start + point(px(24. * zoom), px(0.)), 1)?;
    window.update(cx, |s, _, cx| {
        let n = s.editor.selected_node().unwrap();
        ensure!(
            near(n.frame.width, 384.) && near(n.frame.height, 118.8),
            "Stretch handle constrained dimensions"
        );
        let saved = temp.path().join("stretch.compose");
        file_format::save(&saved, &s.editor.document)?;
        let loaded = file_format::load(&saved)?;
        ensure!(loaded == s.editor.document, "Image mode did not roundtrip");
        s.set_image_sizing(ImageSizing::Scale);
        ensure!(
            near(
                s.editor.selected_node().unwrap().frame.width
                    / s.editor.selected_node().unwrap().frame.height,
                2.
            ),
            "Scale did not restore source proportions"
        );
        s.set_radius(24.);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let (start, zoom) = window.update(cx, |s, _, _| {
        let r = s.absolute_frame(image).unwrap();
        let h = s.radius_handle(r).unwrap();
        (s.screen_rect(Rect::new(h.x, h.y, 0., 0.)).origin, s.zoom)
    })?;
    pointer(
        window,
        cx,
        start,
        start + point(px(10. * zoom), px(10. * zoom)),
        1,
    )?;
    window.update(cx, |s, _, cx| {
        ensure!(
            near(s.editor.selected_node().unwrap().radius, 44.),
            "Radius handle did not round the image"
        );
        s.undo(false);
        ensure!(
            near(s.editor.selected_node().unwrap().radius, 24.),
            "Radius undo failed"
        );
        s.set_radius(10000.);
        let n = s.editor.selected_node().unwrap();
        ensure!(
            near(n.radius, n.frame.height / 2.),
            "Radius exceeded half the shortest edge"
        );
        s.set_radius(24.);
        let n = s.editor.selected_node_mut().unwrap();
        n.frame = Rect::new(48., 260., 480., 240.);
        n.name = "Color study".into();
        let NodeKind::Image { asset } = &n.kind else {
            unreachable!()
        };
        let asset = asset.clone();
        ensure!(
            *s.editor.document.assets[&asset] == *original_bytes,
            "Scaling resampled the original image"
        );
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 156. });
        s.replace_input(
            0..0,
            "Highlight a few words. Make them yours.\nScale an image. Give it softer corners.",
        );
        s.end_edit();
        s.editor.selected_node_mut().unwrap().frame.width = 460.;
        s.format(|v| {
            v.size = 17.;
            v.line_height = 1.55;
            v.color = document::Color::rgb(114, 130, 101)
        });
        s.insert_at(Tool::Shape, DocPoint { x: 48., y: 546. });
        let n = s.editor.selected_node_mut().unwrap();
        n.frame = Rect::new(48., 546., 224., 72.);
        n.radius = 16.;
        n.name = "Rounded component".into();
        s.insert_at(Tool::Text, DocPoint { x: 64., y: 568. });
        s.replace_input(0..0, "Room for your ideas");
        s.end_edit();
        s.format(|v| {
            v.size = 15.;
            v.weight = 700.;
        });
        s.editor.selected_node_mut().unwrap().frame.width = 198.;
        s.editor.document.name = "A more considered canvas".into();
        s.measure();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts");
        let dir = dir.join("refinement-checks");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("refined-example.compose");
        file_format::save(&path, &s.editor.document)?;
        ensure!(
            file_format::load(&path)? == s.editor.document,
            "Refined document did not reopen exactly"
        );
        s.saved = s.editor.document.clone();
        s.path = Some(path);
        s.editor.select_only(Some(image));
        s.set_status(
            "Scale keeps proportions · use Stretch for free dimensions",
            false,
        );
        if std::env::args().any(|a| a == "--show-text-selection") {
            s.editor.select_only(Some(title));
            s.editing = true;
            s.anchor = 13;
            s.caret = 16;
        }
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    println!(
        "REFINEMENT_SMOKE_PASS: mouse-selected word, floating formatting, partial size/color, mixed values, replacement, eight proportional handles, width/height linking, bigger/smaller, stretch, radius drag/undo, original asset and save/reopen"
    );
    Ok(())
}
