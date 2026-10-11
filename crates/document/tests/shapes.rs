use document::{Document, NodeKind, Point, Rect, ShapeKind};

#[test]
fn geometry_has_empty_corners_and_real_line_distance() {
    let r = Rect::new(20., 30., 120., 80.);
    for kind in [
        ShapeKind::Circle,
        ShapeKind::Ellipse,
        ShapeKind::Triangle,
        ShapeKind::Diamond,
        ShapeKind::Pentagon,
        ShapeKind::Hexagon,
        ShapeKind::Star,
        ShapeKind::Arrow,
    ] {
        assert!(
            kind.contains(r, 0., false, 0., Point { x: 80., y: 70. }),
            "{kind:?} center"
        );
        assert!(
            !kind.contains(r, 0., false, 0., Point { x: 21., y: 31. }),
            "{kind:?} corner"
        );
    }
    assert!(ShapeKind::Line.contains(r, 0., false, 2., Point { x: 80., y: 70. }));
    assert!(!ShapeKind::Line.contains(r, 0., false, 2., Point { x: 80., y: 35. }));
    assert!(ShapeKind::Line.contains(r, 0., true, 2., Point { x: 25., y: 107. }));
    assert!(!ShapeKind::Line.contains(r, 0., false, 2., Point { x: 25., y: 107. }));
    assert!(!ShapeKind::Rectangle.contains(r, 20., false, 0., Point { x: 21., y: 31. }));
}
#[test]
fn legacy_shape_nodes_default_to_rectangles() {
    let mut doc = Document::default();
    doc.insert(0, NodeKind::Shape, Rect::new(20., 30., 100., 100.));
    let mut legacy = serde_json::to_value(&doc).unwrap();
    let node = legacy["pages"][0]["nodes"][0].as_object_mut().unwrap();
    node.remove("shape");
    node.remove("line_flipped");
    let reopened: Document = serde_json::from_value(legacy).unwrap();
    assert_eq!(reopened, doc);
}
