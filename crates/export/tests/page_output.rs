use document::{Color, Document, ImageSizing, NodeKind, Rect};
use export::PageRenderer;
use std::sync::Arc;

fn pixel(p: &tiny_skia::Pixmap, x: u32, y: u32) -> [u8; 4] {
    let p = p.pixel(x, y).unwrap();
    [p.red(), p.green(), p.blue(), p.alpha()]
}
#[test]
fn nesting_hidden_layers_opacity_rounding_and_order() {
    let mut doc = Document::default();
    doc.pages[0].width = 120.;
    doc.pages[0].height = 120.;
    let root = doc.insert(0, NodeKind::Frame, Rect::new(10., 20., 80., 80.));
    doc.node_mut(root).unwrap().fill = Color::rgb(255, 0, 0);
    let child = doc.insert(0, NodeKind::Shape, Rect::new(5., 5., 30., 30.));
    doc.node_mut(child).unwrap().fill = Color::rgb(0, 0, 255);
    doc.node_mut(child).unwrap().opacity = 0.5;
    doc.node_mut(child).unwrap().radius = 10.;
    let node = doc.pages[0].nodes.pop().unwrap();
    doc.node_mut(root).unwrap().children.push(node);
    let hidden = doc.insert(0, NodeKind::Shape, Rect::new(0., 0., 120., 120.));
    doc.node_mut(hidden).unwrap().visible = false;
    let canvas = PageRenderer::default().render(&doc, 0, 72).unwrap();
    assert_eq!(pixel(&canvas, 0, 0), [255, 254, 251, 255]);
    assert_eq!(pixel(&canvas, 15, 25), [255, 0, 0, 255]);
    let center = pixel(&canvas, 30, 40);
    assert!(
        center[0].abs_diff(127) <= 1 && center[2].abs_diff(128) <= 1,
        "{center:?}"
    );
    assert_eq!(pixel(&canvas, 16, 26), [255, 0, 0, 255]);
}
#[test]
fn images_keep_proportions_rounding_and_stretch() {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(20, 10, image::Rgba([10, 120, 200, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let mut doc = Document::default();
    doc.pages[0].width = 120.;
    doc.pages[0].height = 120.;
    doc.assets
        .insert("photo".into(), Arc::new(bytes.into_inner()));
    let id = doc.insert(
        0,
        NodeKind::Image {
            asset: "photo".into(),
        },
        Rect::new(10., 10., 80., 80.),
    );
    doc.node_mut(id).unwrap().radius = 10.;
    let mut renderer = PageRenderer::default();
    let fit = renderer.render(&doc, 0, 72).unwrap();
    assert_eq!(pixel(&fit, 50, 15), [255, 254, 251, 255]);
    assert_eq!(pixel(&fit, 50, 40), [10, 120, 200, 255]);
    doc.node_mut(id).unwrap().image_sizing = ImageSizing::Stretch;
    let stretched = renderer.render(&doc, 0, 72).unwrap();
    assert_eq!(pixel(&stretched, 50, 15), [10, 120, 200, 255]);
    assert_eq!(pixel(&stretched, 11, 11), [255, 254, 251, 255]);
}
#[test]
fn mixed_fonts_are_rasterized_and_text_is_clipped() {
    let mut doc = Document::default();
    doc.pages[0].width = 250.;
    doc.pages[0].height = 180.;
    let mut text = text::RichText::new("Light Bold Black\nExtra clipped line");
    text.style.size = 30.;
    text.style.weight = 300.;
    let mut bold = text.style.clone();
    bold.weight = 700.;
    text.apply_style(6..10, bold);
    let mut black = text.style.clone();
    black.weight = 900.;
    black.italic = true;
    text.apply_style(11..16, black);
    doc.insert(0, NodeKind::Text(text), Rect::new(10., 10., 220., 42.));
    let pixels = PageRenderer::default().render(&doc, 0, 144).unwrap();
    let ink = (20..104)
        .flat_map(|y| (20..460).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&pixels, x, y)[0] < 180)
        .count();
    assert!(ink > 900, "Document glyphs were not rendered: {ink}");
    assert_eq!(pixel(&pixels, 80, 120), [255, 254, 251, 255]);
}
#[test]
fn dpi_metadata_page_index_and_failed_export_preserve_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("page.png");
    let doc = Document::default();
    assert_eq!(
        export::write_png(&path, &doc, 0, 300).unwrap(),
        (2481, 3508)
    );
    let bytes = std::fs::read(&path).unwrap();
    let reader = png::Decoder::new(std::io::Cursor::new(&bytes))
        .read_info()
        .unwrap();
    let dims = reader.info().pixel_dims.unwrap();
    assert_eq!(
        (dims.xppu, dims.yppu, dims.unit),
        (11811, 11811, png::Unit::Meter)
    );
    assert!(export::write_png(&path, &doc, 2, 300).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(export::dimensions(1_000_000., 1_000_000., 300).is_err());
}
