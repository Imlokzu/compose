use document::{Document, NodeKind, Point, Rect, ShapeKind};
use editor::Editor;
#[test]
fn empty_shape_corners_select_the_layer_underneath_and_nested_hits_keep_coordinates() {
    let mut editor = Editor::new(Document::default());
    let underneath = editor.insert(NodeKind::Shape, Rect::new(30., 40., 160., 120.));
    let oval = editor.insert(NodeKind::Shape, Rect::new(30., 40., 160., 120.));
    editor.document.node_mut(oval).unwrap().shape = ShapeKind::Ellipse;
    assert_eq!(editor.hit_test(Point { x: 31., y: 41. }), Some(underneath));
    assert_eq!(editor.hit_test(Point { x: 110., y: 100. }), Some(oval));
    let frame = editor.insert(NodeKind::Frame, Rect::new(250., 300., 180., 150.));
    let child = editor.document.pages[0].nodes.remove(1);
    editor
        .document
        .node_mut(frame)
        .unwrap()
        .children
        .push(child);
    assert_eq!(editor.hit_test_deep(Point { x: 360., y: 400. }), Some(oval));
    assert_eq!(
        editor.hit_test_deep(Point { x: 281., y: 341. }),
        Some(frame)
    );
    editor.select_only(Some(frame));
    editor.duplicate_selected();
    let duplicate = editor.selected_node().unwrap();
    assert_eq!(duplicate.children[0].shape, ShapeKind::Ellipse);
}
