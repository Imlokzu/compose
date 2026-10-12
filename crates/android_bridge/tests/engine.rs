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
