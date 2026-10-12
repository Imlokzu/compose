use compose_android_bridge::Session;
use document::{DocumentMode, NodeKind, ShapeKind};
use serde_json::json;
#[test]
fn mobile_unicode_range_format_history_and_desktop_roundtrip() {
    let mut engine = Session::new();
    engine
        .command(&json!({"op":"insert","kind":"Text"}))
        .unwrap();
    engine.command(&json!({"op":"begin"})).unwrap();
    engine
        .command(&json!({"op":"text","content":"Hi 👩‍💻 café Україна"}))
        .unwrap();
    engine
        .command(&json!({"op":"format","start":9,"end":13,"weight":900}))
        .unwrap();
    let NodeKind::Text(text) = &engine.editor.selected_node().unwrap().kind else {
        panic!()
    };
    assert_eq!(
        text.style_at(text.content.find("café").unwrap()).weight,
        900.
    );
    assert_eq!(text.style_at(0).weight, 400.);
    engine.command(&json!({"op":"end"})).unwrap();
    let result = engine.editor.document.clone();
    engine.command(&json!({"op":"undo"})).unwrap();
    let NodeKind::Text(text) = &engine.editor.selected_node().unwrap().kind else {
        panic!()
    };
    assert!(text.content.is_empty());
    engine.command(&json!({"op":"redo"})).unwrap();
    assert_eq!(engine.editor.document, result);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("phone.compose");
    engine.command(&json!({"op":"save","path":file})).unwrap();
    assert_eq!(file_format::load(&file).unwrap(), result);
    let id = engine.editor.selected().unwrap();
    let png = engine.asset(&format!("text:{id}")).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    assert!(image.pixels().any(|p| p[3] == 0) && image.pixels().any(|p| p[3] > 0));
}
#[test]
fn figures_hit_testing_resize_locks_and_rejected_commands_preserve_content() {
    let mut engine = Session::new();
    engine
        .command(&json!({"op":"insert","kind":"Circle"}))
        .unwrap();
    assert_eq!(
        engine.editor.selected_node().unwrap().shape,
        ShapeKind::Circle
    );
    engine
        .command(&json!({"op":"frame","frame":{"x":40,"y":50,"width":170,"height":90}}))
        .unwrap();
    assert_eq!(engine.editor.selected_node().unwrap().frame.height, 170.);
    engine
        .command(
            &json!({"op":"frame","axis":"height","frame":{"x":40,"y":50,"width":170,"height":100}}),
        )
        .unwrap();
    assert_eq!(engine.editor.selected_node().unwrap().frame.width, 100.);
    engine
        .command(&json!({"op":"frame","frame":{"x":40,"y":50,"width":170,"height":90}}))
        .unwrap();
    engine.command(&json!({"op":"hit","x":41,"y":51})).unwrap();
    assert!(engine.editor.selected().is_none());
    engine
        .command(&json!({"op":"hit","x":125,"y":135}))
        .unwrap();
    let before = engine.editor.document.clone();
    assert!(
        engine
            .command(&json!({"op":"style","color":{"r":10,"g":20,"b":30,"a":255},"shape":"bad"}))
            .is_err()
    );
    assert_eq!(engine.editor.document, before);
    engine.editor.selected_node_mut().unwrap().locked = true;
    assert!(engine.command(&json!({"op":"style","radius":12})).is_err());
}
#[test]
fn word_preset_exports_real_docx_and_rejects_figures() {
    let mut engine = Session::new();
    engine
        .command(&json!({"op":"new","preset":"Resume","mode":"Word"}))
        .unwrap();
    assert_eq!(engine.editor.document.mode, DocumentMode::Word);
    let before = engine.editor.document.clone();
    assert!(
        engine
            .command(&json!({"op":"insert","kind":"Star"}))
            .is_err()
    );
    assert_eq!(engine.editor.document, before);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("resume.docx");
    engine.command(&json!({"op":"docx","path":file})).unwrap();
    assert!(std::fs::metadata(file).unwrap().len() > 1000);
}

#[test]
fn mobile_components_hidden_layers_locks_and_previews_preserve_editor() {
    let mut engine = Session::new();
    engine
        .command(&json!({"op":"component","kind":"Callout"}))
        .unwrap();
    let id = engine.editor.selected().unwrap();
    assert!(!engine.editor.document.node(id).unwrap().children.is_empty());
    let before = engine.editor.document.clone();
    let preview = engine.asset("preset:Compose:PhotoResume").unwrap();
    assert!(image::load_from_memory(&preview).is_ok());
    assert_eq!(engine.editor.document, before);
    assert_eq!(engine.editor.selected(), Some(id));
    let snapshot = engine.command(&json!({"op":"visibility","id":id})).unwrap();
    assert!(
        snapshot["scene"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["visible"] == false)
    );
    engine.command(&json!({"op":"visibility","id":id})).unwrap();
    engine.command(&json!({"op":"lock","id":id})).unwrap();
    assert!(
        engine
            .command(&json!({"op":"scale","factor":1.25}))
            .is_err()
    );
    engine.command(&json!({"op":"lock","id":id})).unwrap();
    assert_eq!(engine.editor.document, before);
    engine.command(&json!({"op":"new","mode":"Word"})).unwrap();
    let before = engine.editor.document.clone();
    assert!(
        engine
            .command(&json!({"op":"component","kind":"Title"}))
            .is_err()
    );
    assert_eq!(engine.editor.document, before);
}

#[test]
fn image_sizing_height_alignment_and_undo_keep_original_bytes() {
    let mut engine = Session::new();
    engine
        .command(&json!({"op":"new","preset":"PhotoResume"}))
        .unwrap();
    let photo = engine.editor.document.pages[0]
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Image { .. }))
        .unwrap()
        .clone();
    engine
        .command(&json!({"op":"select","id":photo.id}))
        .unwrap();
    let assets = engine.editor.document.assets.clone();
    let ratio = photo.frame.width / photo.frame.height;
    engine.command(&json!({"op":"frame","axis":"height","frame":{"x":photo.frame.x,"y":photo.frame.y,"width":photo.frame.width,"height":120.}})).unwrap();
    assert!((engine.editor.selected_node().unwrap().frame.width / 120. - ratio).abs() < 0.001);
    engine
        .command(&json!({"op":"image_sizing","mode":"Stretch"}))
        .unwrap();
    engine
        .command(&json!({"op":"frame","frame":{"x":30.,"y":40.,"width":250.,"height":90.}}))
        .unwrap();
    assert_eq!(engine.editor.selected_node().unwrap().frame.height, 90.);
    engine
        .command(&json!({"op":"image_sizing","mode":"Scale"}))
        .unwrap();
    assert!(
        (engine.editor.selected_node().unwrap().frame.width
            / engine.editor.selected_node().unwrap().frame.height
            - ratio)
            .abs()
            < 0.001
    );
    let before = engine.editor.document.clone();
    engine.command(&json!({"op":"scale","factor":0.8})).unwrap();
    engine.command(&json!({"op":"undo"})).unwrap();
    assert_eq!(engine.editor.document, before);
    engine
        .command(&json!({"op":"align_page","alignment":"center_x"}))
        .unwrap();
    let r = engine.editor.selected_node().unwrap().frame;
    assert!((r.x - (engine.editor.document.pages[0].width - r.width) / 2.).abs() < 0.001);
    assert_eq!(engine.editor.document.assets, assets);
}

#[test]
fn coloring_mixed_text_preserves_fonts_weights_links_and_alignment() {
    let mut engine = Session::new();
    engine
        .command(&json!({"op":"insert","kind":"Text"}))
        .unwrap();
    engine
        .command(&json!({"op":"text","content":"light black"}))
        .unwrap();
    engine
        .command(&json!({"op":"format","start":0,"end":5,"weight":300}))
        .unwrap();
    engine.command(&json!({"op":"format","start":6,"end":11,"weight":900,"family":"Lato","link":"example.com"})).unwrap();
    engine.command(&json!({"op":"format","start":0,"end":11,"color":{"r":40,"g":120,"b":237,"a":255},"alignment":"Center"})).unwrap();
    let NodeKind::Text(t) = &engine.editor.selected_node().unwrap().kind else {
        panic!()
    };
    assert_eq!(t.style_at(0).weight, 300.);
    assert_eq!(t.style_at(6).weight, 900.);
    assert_eq!(t.style_at(6).link.as_deref(), Some("https://example.com/"));
    assert_eq!(t.style_at(0).color, t.style_at(6).color);
    assert_eq!(t.align, text::TextAlign::Center);
}
