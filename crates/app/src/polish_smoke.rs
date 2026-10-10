use crate::workspace::{ColorTarget, Field, Popup, Tool, Workspace};
use anyhow::{Result, ensure};
use document::{Color, Document, NodeKind, Point as DocPoint};
use editor::Component;
use gpui::*;
fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
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
fn field(
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
        if field != Field::FontSearch {
            w.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        }
        w.draw(cx).clear();
    })
}
pub fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    window.update(cx, |s, _, cx| {
        s.replace_document(Document::default(), None);
        s.insert_at(Tool::Text, DocPoint { x: 48., y: 80. });
        s.replace_input(0..0, "Make text yours");
        s.anchor = 5;
        s.caret = 9;
        s.toggle_popup(Popup::Fonts);
        cx.notify();
    })?;
    draw(window, cx)?;
    field(window, cx, Field::FontSearch, "mono")?;
    // One filtered result, selected through the native pointer route.
    let pos = window.update(cx, |_, w, _| {
        point(w.bounds().size.width - px(180.), px(172.))
    })?;
    click(window, cx, pos)?;
    window.update(cx, |s, _, _| {
        let t = s.text().unwrap();
        ensure!(
            t.style_at(5).family == "Noto Sans Mono" && t.style_at(0).family == "Lato",
            "Filtered font choice did not preserve selection"
        );
        ensure!(s.range() == (5..9), "Typeface choice lost selected text");
        s.open_color(ColorTarget::Fill);
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    let (bounds, hue) = window.update(cx, |s, _, _| (s.color_bounds, s.hue_bounds))?;
    ensure!(
        bounds.size.width > px(200.) && hue.size.height > px(0.),
        "Color surfaces did not render"
    );
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        let start = bounds.origin + point(bounds.size.width * 0.2, bounds.size.height * 0.3);
        let end = bounds.origin + point(bounds.size.width * 0.8, bounds.size.height * 0.1);
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        for i in 1..=5 {
            w.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: start + (end - start) * (i as f32 / 5.),
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Modifiers::default(),
                }),
                cx,
            );
        }
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.text().unwrap().style_at(5).color != s.text().unwrap().style_at(0).color,
            "Color drag escaped or did not affect selection"
        );
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Color drag required multiple undo steps"
        );
        s.editor.redo();
        s.editing = true;
        s.anchor = 5;
        s.caret = 9;
        s.open_color(ColorTarget::Fill);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    field(window, cx, Field::ColorHex, "615CC680")?;
    window.update(cx, |s, _, _| {
        let c = s.text().unwrap().style_at(5).color;
        ensure!(
            c == Color {
                r: 97,
                g: 92,
                b: 198,
                a: 128
            },
            "RGBA hex did not apply"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    field(window, cx, Field::ColorAlpha, "75")?;
    window.update(cx, |s, _, cx| {
        ensure!(
            s.text().unwrap().style_at(5).color.a == 191,
            "Color alpha did not apply"
        );
        s.close_popup();
        s.end_edit();
        s.assets_tab = true;
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    draw(window, cx)?;
    // Click the first asset tile in the actual left panel.
    click(window, cx, point(px(112.), px(250.)))?;
    window.update(cx, |s, _, _| {
        ensure!(
            matches!(s.editor.selected_node().unwrap().kind, NodeKind::Frame),
            "Assets tile did not insert component"
        );
        ensure!(
            !s.editor.selected_node().unwrap().children.is_empty(),
            "Inserted component has no editable children"
        );
        s.toggle_shadow();
        s.toggle_stroke();
        Ok::<_, anyhow::Error>(())
    })??;
    field(window, cx, Field::StrokeWidth, "2.5")?;
    field(window, cx, Field::ShadowBlur, "18")?;
    window.update(cx, |s, _, cx| {
        let a = &s.editor.selected_node().unwrap().appearance;
        ensure!(
            a.shadow && a.stroke_width == 2.5 && a.shadow_blur == 18.,
            "Appearance controls did not apply"
        );
        let id = s.editor.selected().unwrap();
        s.editor.toggle_lock(id);
        let before = s.editor.document.clone();
        s.begin_field(Field::W, "400".into());
        s.replace_input(0..3, "100");
        s.commit_field();
        ensure!(
            s.editor.document == before,
            "Locked component changed via inspector"
        );
        s.editor.toggle_lock(id);
        s.editor.toggle_visibility(id);
        ensure!(
            !s.editor.document.node(id).unwrap().visible,
            "Visibility toggle failed"
        );
        s.undo(false);
        s.toggle_preview();
        let before = s.editor.document.clone();
        s.undo(false);
        ensure!(s.editor.document == before, "Preview allowed mutation");
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse("escape").unwrap(), cx);
        w.draw(cx).clear();
    })?;
    window.update(cx, |s, _, _| {
        ensure!(!s.preview, "Escape did not exit preview");
        s.set_page_size(960., 540.);
        ensure!(
            s.editor.document.pages[0].width == 960.,
            "Page preset failed"
        );
        s.undo(false);
        Ok::<_, anyhow::Error>(())
    })??;
    // Build a fully editable editorial showcase, then save and verify every style.
    window.update(cx,|s,_,cx|{
  s.replace_document(Document::default(),None);s.editor.document.name="The thoughtful edit".into();
  let title=s.editor.insert_component(Component::Title,48.,54.);let nodes=&mut s.editor.document.node_mut(title).unwrap().children;
  if let NodeKind::Text(t)=&mut nodes[0].kind{t.content="COMPOSE   /   FIELD NOTES 001".into();}
  if let NodeKind::Text(t)=&mut nodes[1].kind{t.content="Good ideas need space.".into();t.style.size=35.;t.style.family="Noto Serif".into();}
  if let NodeKind::Text(t)=&mut nodes[2].kind{t.content="A quieter way to write, arrange, and make something yours.\nSix typefaces. Every color. A few thoughtful starting points.".into();t.style.size=13.;}
  let callout=s.editor.insert_component(Component::Callout,48.,245.);s.editor.document.node_mut(callout).unwrap().frame.width=499.;
  let stat=s.editor.insert_component(Component::Stat,48.,395.);s.editor.document.node_mut(stat).unwrap().fill=Color::rgb(246,226,188);
  let card=s.editor.insert_component(Component::Card,291.,395.);s.editor.document.node_mut(card).unwrap().fill=Color::rgb(255,254,251);
  let quote=s.editor.insert_component(Component::Quote,48.,627.);let n=s.editor.document.node_mut(quote).unwrap();n.frame.width=499.;
  for node in &mut n.children{if let NodeKind::Text(t)=&mut node.kind{t.style.size=if t.content.starts_with('“'){20.}else{11.};}}
  s.measure();let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/polish/showcase.compose");file_format::save(&path,&s.editor.document)?;let loaded=file_format::load(&path)?;ensure!(loaded==s.editor.document,"Styled components failed roundtrip");s.saved=loaded;s.path=Some(path);s.editor.select_only(Some(card));s.assets_tab=true;s.set_status("Every detail is editable",false);
  if std::env::args().any(|a|a=="--show-fonts"){s.editor.select_only(Some(s.editor.document.node(title).unwrap().children[1].id));s.toggle_popup(Popup::Fonts);}
  if std::env::args().any(|a|a=="--show-colors"){s.open_color(ColorTarget::Fill);}
  cx.notify();Ok::<_,anyhow::Error>(())
 })??;
    draw(window, cx)?;
    println!(
        "POLISH_SMOKE_PASS: filtered native typeface picker, range-preserving colors, coalesced color drag, RGBA/alpha, asset tile insertion, appearance fields, locks, visibility, preview, page presets, styled-component roundtrip"
    );
    Ok(())
}
