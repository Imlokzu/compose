use crate::workspace::{Tool, Workspace};
use anyhow::{Result, ensure};
use document::{Color, Document, NodeKind, Rect};
use gpui::*;
use std::sync::Arc;

fn draw(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| w.draw(cx).clear())
}
fn click(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    p: Point<Pixels>,
    shift: bool,
    count: usize,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        let modifiers = Modifiers {
            shift,
            ..Default::default()
        };
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: p,
                modifiers,
                click_count: count,
                ..Default::default()
            }),
            cx,
        );
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: p,
                modifiers,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
fn key(window: WindowHandle<Workspace>, cx: &mut AsyncApp, key: &str) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        w.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
        w.draw(cx).clear();
    })
}
fn center(window: WindowHandle<Workspace>, cx: &mut AsyncApp, id: u64) -> Result<Point<Pixels>> {
    window.update(cx, |s, _, _| {
        s.screen_rect(s.absolute_frame(id).unwrap()).center()
    })
}
fn button(window: WindowHandle<Workspace>, cx: &mut AsyncApp, id: &'static str) -> Result<()> {
    draw(window, cx)?;
    let p = window
        .update(cx, |s, _, _| {
            s.selection_control_bounds.get(id).map(|b| b.center())
        })?
        .ok_or_else(|| anyhow::anyhow!("Missing selection control: {id}"))?;
    click(window, cx, p, false, 1)
}
fn drag(
    window: WindowHandle<Workspace>,
    cx: &mut AsyncApp,
    start: Point<Pixels>,
    end: Point<Pixels>,
    alt: bool,
) -> Result<()> {
    AnyWindowHandle::from(window).update(cx, |_, w, cx| {
        let modifiers = Modifiers {
            alt,
            ..Default::default()
        };
        w.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                position: start,
                click_count: 1,
                modifiers,
                ..Default::default()
            }),
            cx,
        );
        for fraction in [0.3, 0.7, 1.] {
            w.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: start + (end - start) * fraction,
                    pressed_button: Some(MouseButton::Left),
                    modifiers,
                }),
                cx,
            );
            w.draw(cx).clear();
        }
        w.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                position: end,
                modifiers,
                ..Default::default()
            }),
            cx,
        );
        w.draw(cx).clear();
    })
}
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn scene() -> (Document, [u64; 3]) {
    let mut doc = Document {
        name: "Keep it together".into(),
        ..Default::default()
    };
    let mut title = text::RichText::new("Build a page.\nKeep it together.");
    title.style.size = 40.;
    title.style.weight = 900.;
    doc.insert(0, NodeKind::Text(title), Rect::new(48., 64., 500., 125.));
    let a = doc.insert(0, NodeKind::Shape, Rect::new(48., 255., 140., 160.));
    doc.node_mut(a).unwrap().fill = Color::rgb(211, 225, 203);
    doc.assets.insert(
        "selection-photo".into(),
        Arc::new(include_bytes!("../../../brand/reference-ui.jpg").to_vec()),
    );
    let b = doc.insert(
        0,
        NodeKind::Image {
            asset: "selection-photo".into(),
        },
        Rect::new(212., 265., 150., 145.),
    );
    let c = doc.insert(0, NodeKind::Shape, Rect::new(416., 255., 140., 160.));
    doc.node_mut(c).unwrap().fill = Color::rgb(219, 230, 241);
    for id in [a, b, c] {
        let n = doc.node_mut(id).unwrap();
        n.radius = 12.;
        n.appearance.shadow = true;
    }
    for (x, copy) in [(48., "WRITE"), (212., "DESIGN"), (416., "TOGETHER")] {
        let mut text = text::RichText::new(copy);
        text.style.size = 11.;
        text.style.weight = 700.;
        text.style.letter_spacing = 1.2;
        doc.insert(0, NodeKind::Text(text), Rect::new(x, 443., 150., 25.));
    }
    (doc, [a, b, c])
}
pub async fn run(window: WindowHandle<Workspace>, cx: &mut AsyncApp) -> Result<()> {
    let (doc, ids) = scene();
    let [a, b, c] = ids;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/selection");
    std::fs::create_dir_all(&dir)?;
    window.update(cx, |s, _, cx| {
        s.replace_document(doc.clone(), None);
        s.zoom = 0.7;
        s.fitted = true;
        s.tool = Tool::Select;
        cx.notify();
    })?;
    draw(window, cx)?;
    let pa = center(window, cx, a)?;
    click(window, cx, pa, false, 1)?;
    let pb = center(window, cx, b)?;
    click(window, cx, pb, true, 1)?;
    let pc = center(window, cx, c)?;
    click(window, cx, pc, true, 1)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.editor.selection_count() == 3,
            "Shift-click did not select three objects"
        );
        ensure!(
            s.text().is_none(),
            "Multiple selection leaked into single-object text controls"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    button(window, cx, "selection-top")?;
    window.update(cx, |s, _, _| {
        ensure!(
            ids.iter()
                .all(|id| close(s.absolute_frame(*id).unwrap().y, 255.)),
            "Native alignment did not align every object"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    key(window, cx, "cmd-z")?;
    ensure!(
        window.update(cx, |s, _, _| close(s.absolute_frame(b).unwrap().y, 265.)
            && s.editor.selection_count() == 3)?,
        "Alignment undo lost the selected objects"
    );
    button(window, cx, "selection-distribute-x")?;
    ensure!(
        window.update(cx, |s, _, _| close(s.absolute_frame(b).unwrap().x, 227.))?,
        "Equal-spacing control did not space the image"
    );
    key(window, cx, "cmd-z")?;
    let before = window.update(cx, |s, _, _| s.editor.document.clone())?;
    let positions = window.update(cx, |s, _, _| s.editor.selection_frames())?;
    let start = center(window, cx, a)?;
    let delta = window.update(cx, |s, _, _| point(px(22. * s.zoom), px(17. * s.zoom)))?;
    drag(window, cx, start, start + delta, true)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.editor.selection_count() == 3,
            "Dragging collapsed a multiple selection"
        );
        for (id, r) in &positions {
            let actual = s.absolute_frame(*id).unwrap();
            ensure!(
                close(actual.x, r.x + 22.) && close(actual.y, r.y + 17.),
                "An object did not move with the selection"
            );
        }
        Ok::<_, anyhow::Error>(())
    })??;
    key(window, cx, "cmd-z")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.document == before)?,
        "Move was not a single undo transaction"
    );
    key(window, cx, "cmd-shift-z")?;
    let before_nudge = window.update(cx, |s, _, _| s.editor.document.clone())?;
    key(window, cx, "shift-right")?;
    ensure!(
        window.update(cx, |s, _, _| close(
            s.absolute_frame(a).unwrap().x,
            before_nudge.node(a).unwrap().frame.x + 10.
        ))?,
        "Keyboard nudge did not move the selection"
    );
    key(window, cx, "cmd-z")?;
    let before_group = window.update(cx, |s, _, _| s.editor.document.clone())?;
    button(window, cx, "selection-group")?;
    let (group, grouped) = window.update(cx, |s, _, _| {
        let group = s
            .editor
            .selected_node()
            .ok_or_else(|| anyhow::anyhow!("Grouping left no selection"))?;
        ensure!(
            group.is_group && group.children.len() == 3,
            "Native group control did not create a real group"
        );
        ensure!(
            !s.resizable_selection(),
            "Unsupported group scaling exposed resize handles"
        );
        Ok::<_, anyhow::Error>((group.id, s.editor.document.clone()))
    })??;
    let paint = cx.background_executor().spawn({
        let dir = dir.clone();
        let grouped = grouped.clone();
        async move {
            let mut renderer = export::PageRenderer::default();
            let before = renderer.render(&before_group, 0, 144)?;
            let after = renderer.render(&grouped, 0, 144)?;
            ensure!(
                before.data() == after.data(),
                "Grouping changed exported page pixels"
            );
            export::save_png(&dir.join("grouped.png"), &after, 144)?;
            file_format::save(&dir.join("grouped.compose"), &grouped)?;
            ensure!(
                file_format::load(&dir.join("grouped.compose"))? == grouped,
                "Group structure did not reopen exactly"
            );
            Ok::<_, anyhow::Error>(())
        }
    });
    paint.await?;
    let world = window.update(cx, |s, _, _| ids.map(|id| s.absolute_frame(id).unwrap()))?;
    let start = center(window, cx, group)?;
    let delta = window.update(cx, |s, _, _| point(px(24. * s.zoom), px(8. * s.zoom)))?;
    drag(window, cx, start, start + delta, true)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.editor.selected() == Some(group),
            "Clicking inside a group selected its child"
        );
        for (id, r) in ids.iter().zip(world) {
            let actual = s.absolute_frame(*id).unwrap();
            ensure!(
                close(actual.x, r.x + 24.) && close(actual.y, r.y + 8.),
                "Group drag did not move its contents"
            );
        }
        Ok::<_, anyhow::Error>(())
    })??;
    key(window, cx, "cmd-z")?;
    let start = center(window, cx, a)?;
    click(window, cx, start, false, 2)?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.selected() == Some(a))?,
        "Double-click did not drill into a group"
    );
    let stable = window.update(cx, |s, _, _| s.absolute_frame(b).unwrap())?;
    let delta = window.update(cx, |s, _, _| point(px(-50. * s.zoom), px(-20. * s.zoom)))?;
    drag(window, cx, start, start + delta, true)?;
    ensure!(
        window.update(cx, |s, _, _| {
            let actual = s.absolute_frame(b).unwrap();
            s.editor.selected() == Some(a) && close(actual.x, stable.x) && close(actual.y, stable.y)
        })?,
        "Editing a grouped child moved its sibling"
    );
    key(window, cx, "cmd-z")?;
    key(window, cx, "escape")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.selected() == Some(group))?,
        "Escape did not return from a child to its group"
    );
    key(window, cx, "cmd-shift-g")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.selection_count() == 3
            && s.editor.document.node(group).is_none())?,
        "Ungroup shortcut did not restore selected children"
    );
    key(window, cx, "escape")?;
    let (start, end) = window.update(cx, |s, _, _| {
        (
            s.screen_rect(Rect::new(30., 235., 0., 0.)).origin,
            s.screen_rect(Rect::new(580., 440., 0., 0.)).origin,
        )
    })?;
    drag(window, cx, start, end, false)?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.selection_count() == 3
            && ids.iter().all(|id| s.editor.is_selected(*id)))?,
        "Marquee did not select the intended three objects"
    );
    key(window, cx, "cmd-a")?;
    ensure!(
        window.update(cx, |s, _, _| s.editor.selection_count()
            == s.editor.document.pages[0].nodes.len())?,
        "Select All did not select page objects"
    );
    key(window, cx, "escape")?;
    button(window, cx, "layers-tab")?;
    let layer = window
        .update(cx, |s, _, _| s.layer_bounds.get(&a).map(|b| b.center()))?
        .ok_or_else(|| anyhow::anyhow!("Layer row was not laid out"))?;
    click(window, cx, layer, false, 1)?;
    let layer = window
        .update(cx, |s, _, _| s.layer_bounds.get(&b).map(|b| b.center()))?
        .ok_or_else(|| anyhow::anyhow!("Second layer row was not laid out"))?;
    click(window, cx, layer, true, 1)?;
    window.update(cx, |s, _, _| {
        ensure!(
            s.editor.selection_count() == 2,
            "Layers panel Shift-click did not preserve the selection"
        );
        let before = s.editor.document.clone();
        let item = s
            .copy_item(false)
            .ok_or_else(|| anyhow::anyhow!("Multiple-object copy returned nothing"))?;
        s.paste_item(&item);
        ensure!(
            s.editor.selection_count() == 2,
            "Multiple-object paste selected only one object"
        );
        s.undo(false);
        ensure!(
            s.editor.document == before,
            "Multiple paste was not one undo step"
        );
        Ok::<_, anyhow::Error>(())
    })??;
    let title = window.update(cx, |s, _, _| s.editor.document.pages[0].nodes[0].id)?;
    window.update(cx, |s, _, cx| {
        s.end_edit();
        s.editor.set_selection(vec![title, a]);
        cx.notify();
    })?;
    key(window, cx, "cmd-g")?;
    let word = window.update(cx, |s, _, _| {
        s.measure();
        let frame = s.absolute_frame(title).unwrap();
        let selected = s.layouts[&title].2.selection(0..5)[0];
        s.screen_rect(Rect::new(
            frame.x + selected.x,
            frame.y + selected.y,
            selected.width,
            selected.height,
        ))
        .center()
    })?;
    click(window, cx, word, false, 2)?;
    ensure!(
        window.update(cx, |s, _, _| s.editing
            && s.editor.selected() == Some(title)
            && s.range() == (0..5))?,
        "Grouped text did not select the clicked word"
    );
    key(window, cx, "cmd-b")?;
    ensure!(
        window.update(cx, |s, _, _| s.text().is_some_and(|t| t.style_at(0).weight
            == 400.
            && t.style_at(6).weight == 900.))?,
        "Grouped text formatting changed unselected text"
    );
    key(window, cx, "cmd-z")?;
    window.update(cx, |s, _, cx| {
        s.replace_document(doc, None);
        s.zoom = 0.7;
        s.fitted = true;
        s.editor.set_selection(ids.to_vec());
        s.set_status("Shift-click or drag a selection · group with ⌘G", false);
        if std::env::args().any(|a| a == "--show-selection-group") {
            s.group_selection();
        }
        let path = dir.join("showcase.compose");
        file_format::save(&path, &s.editor.document)?;
        s.saved = s.editor.document.clone();
        s.path = Some(path);
        cx.notify();
        Ok::<_, anyhow::Error>(())
    })??;
    println!(
        "SELECTION_PASS: native Shift-click and marquee, batch drag/nudge, inspector align/distribute/group, group drill-down/bounds, grouped text selection/formatting, ungroup shortcut, layers Shift-click, multi-object clipboard, one-step history, unchanged PNG pixels and exact group reopen"
    );
    Ok(())
}
