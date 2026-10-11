//! Real native pointer/input checks for the selection toolbar and its menus.
use crate::workspace::{Field, Popup, Tool, Workspace};
use anyhow::{Result, ensure};
use document::{Color, Document, Point as DocPoint, Rect};
use gpui::*;

fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.draw(cx).clear();
        w.draw(cx).clear();
    })
}
fn click(window: WindowHandle<Workspace>, cx: &mut AsyncApp, at: Point<Pixels>) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: at,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: at,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
fn toolbar_click(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    id: &'static str,
) -> Result<()> {
    draw(window, cx)?;
    let at = window.update(cx, |s, _, _| s.selection_control_bounds[id].center())?;
    click(window, cx, at)
}
fn menu_click(window: WindowHandle<Workspace>, cx: &mut AsyncApp, x: f32, y: f32) -> Result<()> {
    let at = window.update(cx, |s, _, _| {
        s.text_popup_bounds.origin + point(px(x), px(y))
    })?;
    click(window, cx, at)
}
fn key(window: WindowHandle<Workspace>, cx: &mut AsyncApp, key: &str) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
        w.draw(cx).clear();
    })
}
pub fn font_weights(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 280. });
        s.replace_input(0..0, "For sample text.");
        s.end_edit();
        s.format(|v| v.size = 32.);
        s.editor.selected_node_mut().unwrap().frame.width = 480.;
        s.editing = true;
        s.anchor = 4;
        s.caret = 10;
        s.measure();
        cx.notify();
    })?;
    draw(window, cx)?;
    for i in [0, 3, 1, 2, 0, 3] {
        let weight = renderer::FONT_CHOICES[0].weights[i].0;
        toolbar_click(window, cx, "float-weight")?;
        menu_click(window, cx, 80., 74. + 39. * i as f32)?;
        window.update(cx, |s, _, _| {
            let t = s.text().unwrap();
            ensure!(
                s.popup.is_none() && s.range() == (4..10),
                "Weight choice lost the selected passage"
            );
            ensure!(
                t.style_at(4).weight == weight
                    && t.style_at(0).weight == 400.
                    && t.style_at(11).weight == 400.,
                "Weight {weight} escaped selection"
            );
            Ok::<_, anyhow::Error>(())
        })??;
        toolbar_click(window, cx, "float-italic")?;
        window.update(cx, |s, _, _| {
            ensure!(
                s.text().unwrap().style_at(4).italic && !s.text().unwrap().style_at(0).italic,
                "Italic weight {weight} escaped selection"
            );
            Ok::<_, anyhow::Error>(())
        })??;
        toolbar_click(window, cx, "float-italic")?;
    }
    window.update(cx, |s, _, cx| {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/font-weights");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("selected-weight.compose");
        let before = s.editor.document.clone();
        file_format::save(&path, &before)?;
        ensure!(
            file_format::load(&path)? == before,
            "Extended weight did not roundtrip"
        );
        s.undo(false);
        s.undo(true);
        ensure!(
            s.editor.document == before,
            "Weight/italic history changed the document"
        );
        // An editable proof sheet also exercises glyph painting at every Lato face.
        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 52. });
        s.replace_input(0..0, "Every weight, every word.");
        s.end_edit();
        s.format(|v| {
            v.family = "Noto Serif".into();
            v.size = 30.;
            v.weight = 700.;
        });
        s.editor.selected_node_mut().unwrap().frame.width = 500.;
        for (i, (weight, name)) in renderer::FONT_CHOICES[0].weights.iter().enumerate() {
            for italic in [false, true] {
                let x = if italic { 310. } else { 48. };
                let y = 152. + 128. * i as f32;
                s.insert_at(Tool::Text, DocPoint { x, y });
                s.replace_input(
                    0..0,
                    &format!(
                        "{name} {weight:.0}{}",
                        if italic { " · italic" } else { "" }
                    ),
                );
                s.end_edit();
                s.format(|v| {
                    v.size = 11.;
                    v.color = Color::rgb(110, 124, 103);
                });
                s.editor.selected_node_mut().unwrap().frame.width = 240.;
                s.insert_at(Tool::Text, DocPoint { x, y: y + 28. });
                s.replace_input(0..0, "Compose. Café 0123");
                s.end_edit();
                s.format(|v| {
                    v.size = 25.;
                    v.weight = *weight;
                    v.italic = italic;
                });
                let n = s.editor.selected_node_mut().unwrap();
                n.frame.width = 240.;
                n.name = format!("{name}{}", if italic { " italic" } else { "" });
            }
        }
        s.editor.document.name = "Font weights, corrected".into();
        s.measure();
        let path = dir.join("weights.compose");
        file_format::save(&path, &s.editor.document)?;
        ensure!(
            file_format::load(&path)? == s.editor.document,
            "Weight proof sheet did not reopen exactly"
        );
        s.saved = s.editor.document.clone();
        s.path = Some(path);
        s.editor.select_only(None);
        s.set_status(
            "Light · Regular · Bold · Black — original font faces",
            false,
        );
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    println!(
        "FONT_WEIGHTS_UI_PASS: actual Light/Black/Regular/Bold menu clicks, italic variants, range isolation, repeated switches, undo/redo and native save/reopen"
    );
    Ok(())
}
pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 280. });
        s.replace_input(0..0, "Small words. Big ideas.");
        s.end_edit();
        s.format(|v| v.size = 24.);
        s.editor.selected_node_mut().unwrap().frame.width = 480.;
        s.editing = true;
        s.anchor = 13;
        s.caret = 16;
        s.measure();
        cx.notify();
    })?;
    draw(window, cx)?;
    window.update(cx, |s, _, _| {
        let b = s.text_toolbar_bounds().unwrap();
        ensure!(
            b.left() >= s.viewport.left() + px(8.) && b.right() <= s.viewport.right() - px(8.),
            "Toolbar overflowed horizontally"
        );
        let selection = s.layout().unwrap().selection(s.range())[0];
        let n = s.absolute_frame(s.editor.selected().unwrap()).unwrap();
        let r = s.screen_rect(Rect::new(
            n.x + selection.x,
            n.y + selection.y,
            selection.width,
            selection.height,
        ));
        let expected = ((f32::from(r.left() + r.right()) / 2.)
            - crate::text_menu::TEXT_TOOLBAR_WIDTH / 2.)
            .clamp(
                f32::from(s.viewport.left()) + 8.,
                f32::from(s.viewport.right()) - crate::text_menu::TEXT_TOOLBAR_WIDTH - 8.,
            );
        ensure!(
            (f32::from(b.left()) - expected).abs() < 0.5,
            "Toolbar did not center on the selected text"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    toolbar_click(window, cx, "float-font")?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.popup == Some(Popup::Fonts) && s.text_popup_anchor.is_some(),
            "Font menu was not opened near selection"
        );
        ensure!(
            s.field
                .as_ref()
                .is_some_and(|f| f.field == Field::FontSearch),
            "Font search did not receive focus"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        for ch in "serif".chars() {
            w.dispatch_keystroke(
                Keystroke {
                    key: ch.to_string(),
                    key_char: Some(ch.to_string()),
                    ..Default::default()
                },
                cx,
            );
        }
        w.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, _| {
        let t = s.text().unwrap();
        ensure!(
            t.content == "Small words. Big ideas."
                && t.style_at(13).family == "Noto Serif"
                && t.style_at(0).family == "Lato",
            "Font search replaced content or formatted neighboring text"
        );
        ensure!(
            s.range() == (13..16) && s.popup.is_none(),
            "Font selection lost the passage"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    toolbar_click(window, cx, "float-weight")?;
    menu_click(window, cx, 70., 113.)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.popup.is_none()
                && s.text().unwrap().style_at(13).weight == 700.
                && s.text().unwrap().style_at(0).weight == 400.,
            "Native weight choice escaped selection"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    toolbar_click(window, cx, "float-sizes")?;
    menu_click(window, cx, 60., 190.)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.popup.is_none()
                && s.text().unwrap().style_at(13).size == 32.
                && s.text().unwrap().style_at(0).size == 24.,
            "Native size preset escaped selection"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    toolbar_click(window, cx, "float-styles")?;
    key(window, cx, "right")?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(
            Keystroke {
                key: "x".into(),
                key_char: Some("x".into()),
                ..Default::default()
            },
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, w, cx| {
        s.replace_and_mark_text_in_range(None, "か", Some(1..1), w, cx);
        ensure!(
            s.range() == (13..16) && s.text().unwrap().content == "Small words. Big ideas.",
            "Menu typing changed selected text"
        );
        ensure!(
            s.marked.is_none(),
            "Menu typing started document IME composition"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    menu_click(window, cx, 90., 145.)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.popup.is_none(),
            "Style menu stayed open after choosing a style"
        );
        let t = s.text().unwrap();
        ensure!(
            t.style_at(13).size == 28.
                && t.style_at(13).weight == 700.
                && t.style_at(0).size == 24.,
            "Heading preset escaped selected passage"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    toolbar_click(window, cx, "float-color")?;
    let color_at = window.update(cx, |s, _, _| {
        ensure!(
            s.popup == Some(Popup::Colors) && s.text_popup_anchor.is_some(),
            "Color menu was not anchored to the toolbar"
        );
        Ok::<_, anyhow::Error>(
            s.color_bounds.origin
                + point(
                    s.color_bounds.size.width * 0.8,
                    s.color_bounds.size.height * 0.2,
                ),
        )
    })??;
    click(window, cx, color_at)?;
    key(window, cx, "escape")?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.range() == (13..16)
                && s.text().unwrap().style_at(13).color != s.text().unwrap().style_at(0).color,
            "Native color edit escaped the selected text"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    // Rejected field input must keep focus so correction cannot delete the passage.
    toolbar_click(window, cx, "float-size-field")?;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(
            Keystroke {
                key: "0".into(),
                key_char: Some("0".into()),
                ..Default::default()
            },
            cx,
        );
        w.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.error
                && s.field
                    .as_ref()
                    .is_some_and(|f| f.field == Field::Size && f.value == "0"),
            "Invalid size lost field focus"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    key(window, cx, "backspace")?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.range() == (13..16)
                && s.text().unwrap().content == "Small words. Big ideas."
                && s.field.as_ref().is_some_and(|f| f.value.is_empty()),
            "Correcting invalid size deleted document text"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    key(window, cx, "escape")?;
    toolbar_click(window, cx, "float-reset")?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.range() == (13..16)
                && s.text().unwrap().style_at(13) == text::TextStyle::default()
                && s.text().unwrap().style_at(0).size == 24.,
            "Clear formatting changed neighboring text"
        );
        for preset in 0..crate::text_menu::TEXT_PRESETS.len() {
            let before = s.editor.document.clone();
            s.type_preset(preset);
            ensure!(
                s.editor.document.validate().is_ok(),
                "Style {preset} produced invalid rich text"
            );
            let after = s.editor.document.clone();
            s.undo(false);
            ensure!(
                s.editor.document == before,
                "Style {preset} did not undo in one step"
            );
            s.undo(true);
            ensure!(s.editor.document == after, "Style {preset} did not redo");
            s.editing = true;
            s.anchor = 13;
            s.caret = 16;
        }
        // Multiline selections and camera changes must stay inside the canvas.
        s.anchor = 0;
        s.caret = s.text().unwrap().content.len();
        s.editor.selected_node_mut().unwrap().frame.width = 140.;
        s.measure();
        ensure!(
            s.layout().unwrap().selection(s.range()).len() > 1,
            "Multiline fixture did not wrap"
        );
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    for (zoom, pan) in [(0.4, -80.), (1., 0.), (2., -200.)] {
        window.update(cx, |s, _, cx| {
            s.zoom = zoom;
            s.pan.x = pan;
            s.pan.y = -100.;
            cx.notify();
        })?;
        draw(window, cx)?;
        window.update(cx, |s, _, _| {
            if let Some(b) = s.text_toolbar_bounds() {
                ensure!(
                    b.left() >= s.viewport.left() + px(8.)
                        && b.right() <= s.viewport.right() - px(8.)
                        && b.top() >= s.viewport.top() + px(8.)
                        && b.bottom() <= s.viewport.bottom() - px(8.),
                    "Multiline toolbar overflowed at zoom {zoom}"
                );
            }
            Ok::<_, anyhow::Error>(())
        })??;
    }
    fixture(window, cx)?;
    println!(
        "TEXT_MENU_PASS: centered native toolbar, searched font via Enter, weight/size/style/color/reset clicks, preserved text range/neighbors, menu typing isolation, all style history, multiline placement and save/reopen"
    );
    Ok(())
}

fn fixture(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 58. });
        s.replace_input(0..0, "COMPOSE  /  A LITTLE MORE YOU");
        s.end_edit();
        s.format(|v| { v.size = 10.; v.letter_spacing = 1.6; v.weight = 700.; v.color = Color::rgb(110, 124, 103); });
        s.editor.selected_node_mut().unwrap().frame.width = 490.;
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 104. });
        s.replace_input(0..0, "Make every word\nyour own.");
        s.end_edit();
        s.format(|v| { v.family = "Noto Serif".into(); v.size = 38.; v.line_height = 1.25; });
        s.editor.selected_node_mut().unwrap().frame.width = 490.;
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 372. });
        s.replace_input(0..0, "Write a little. Shape the details.\nChoose a font, try a style, find your color.\nYour words deserve a little room.");
        s.end_edit();
        s.format(|v| { v.size = 18.; v.line_height = 1.65; });
        s.editor.selected_node_mut().unwrap().frame.width = 490.;
        let body = s.editor.selected().unwrap();
        let start = s.text().unwrap().content.find("Shape the details.").unwrap();
        let end = start + "Shape the details.".len();
        s.editing = true;
        s.anchor = start;
        s.caret = end;
        s.format(|v| { v.weight = 700.; v.color = Color::rgb(40, 120, 237); });
        s.end_edit();
        s.insert_at(Tool::Shape, DocPoint { x: 48., y: 546. });
        let n = s.editor.selected_node_mut().unwrap();
        n.frame = Rect::new(48., 546., 499., 116.);
        n.radius = 14.;
        n.fill = Color::rgb(235, 240, 225);
        n.name = "A softer space".into();
        s.insert_at(Tool::Text, DocPoint { x: 68., y: 570. });
        s.replace_input(0..0, "A calmer way to create");
        s.end_edit();
        s.format(|v| { v.size = 19.; v.weight = 700.; });
        s.editor.selected_node_mut().unwrap().frame.width = 450.;
        s.insert_at(Tool::Text, DocPoint { x: 68., y: 610. });
        s.replace_input(0..0, "Less reaching for panels. More shaping your ideas.");
        s.end_edit();
        s.format(|v| { v.size = 13.; v.color = Color::rgb(97, 111, 89); });
        s.editor.selected_node_mut().unwrap().frame.width = 450.;
        s.editor.document.name = "Make every word your own".into();
        s.editor.select_only(Some(body));
        s.editing = true;
        s.anchor = start;
        s.caret = end;
        s.measure();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/text-menu");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("text-styles.compose");
        file_format::save(&path, &s.editor.document)?;
        ensure!(file_format::load(&path)? == s.editor.document, "Text menu styles did not reopen exactly");
        s.saved = s.editor.document.clone();
        s.path = Some(path);
        s.set_status("Select a passage. Make it yours.", false);
        if std::env::args().any(|a| a == "--show-text-styles") { s.open_text_popup(Popup::TextStyles); }
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)
}
