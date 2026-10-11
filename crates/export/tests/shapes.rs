use document::{Color, Document, NodeKind, Rect, ShapeKind};
use export::PageRenderer;
#[test]
fn exported_curves_and_polygons_have_clear_corners_filled_centers_and_real_strokes() {
    for kind in ShapeKind::ALL {
        let mut doc = Document::default();
        doc.pages[0].width = 120.;
        doc.pages[0].height = 120.;
        let id = doc.insert(0, NodeKind::Shape, Rect::new(20., 20., 80., 80.));
        let n = doc.node_mut(id).unwrap();
        n.shape = kind;
        n.fill = Color::rgb(40, 130, 210);
        n.appearance.stroke_width = 2.;
        n.appearance.stroke = Color::rgb(220, 50, 60);
        let canvas = PageRenderer::default().render(&doc, 0, 72).unwrap();
        let center = canvas.pixel(60, 60).unwrap();
        assert!(
            center.blue() > 180 && center.red() < 80,
            "{kind:?}: center {center:?}"
        );
        if kind != ShapeKind::Rectangle && kind != ShapeKind::Line {
            let corner = canvas.pixel(21, 21).unwrap();
            assert_eq!(
                [corner.red(), corner.green(), corner.blue()],
                [255, 254, 251],
                "{kind:?}"
            );
        }
        if matches!(kind, ShapeKind::Circle | ShapeKind::Ellipse) {
            let top = canvas.pixel(60, 20).unwrap();
            assert!(top.red() > 180 && top.green() < 100);
        }
    }
}
#[test]
fn diagonal_line_orientation_and_shadows_use_the_line_silhouette() {
    let mut doc = Document::default();
    doc.pages[0].width = 120.;
    doc.pages[0].height = 120.;
    let id = doc.insert(0, NodeKind::Shape, Rect::new(20., 20., 80., 80.));
    let n = doc.node_mut(id).unwrap();
    n.shape = ShapeKind::Line;
    n.line_flipped = true;
    n.fill = Color::rgb(40, 130, 210);
    n.appearance.stroke_width = 4.;
    n.appearance.shadow = true;
    let canvas = PageRenderer::default().render(&doc, 0, 72).unwrap();
    assert!(canvas.pixel(30, 90).unwrap().blue() > 180);
    let clear = canvas.pixel(30, 30).unwrap();
    assert_eq!([clear.red(), clear.green(), clear.blue()], [255, 254, 251]);
}
