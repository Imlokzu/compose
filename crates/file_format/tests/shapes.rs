use document::{Document, NodeKind, Rect, ShapeKind};
#[test]
fn all_figures_and_line_direction_survive_native_container_roundtrip() {
    let mut doc = Document::default();
    for (i, kind) in ShapeKind::ALL.into_iter().enumerate() {
        let id = doc.insert(
            0,
            NodeKind::Shape,
            Rect::new(20. + i as f32 * 4., 40. + i as f32 * 20., 100., 100.),
        );
        let n = doc.node_mut(id).unwrap();
        n.shape = kind;
        n.line_flipped = kind == ShapeKind::Line;
        n.name = kind.label().into();
        n.radius = if kind.supports_radius() { 8. } else { 0. };
        n.appearance.stroke_width = 2.;
        n.appearance.shadow = true;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("figures.compose");
    file_format::save(&path, &doc).unwrap();
    assert_eq!(file_format::load(&path).unwrap(), doc);
}
