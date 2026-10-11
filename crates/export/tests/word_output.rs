use document::{Document, DocumentMode, NodeKind, Page, Rect};
use std::{
    io::{Cursor, Read},
    sync::Arc,
};
fn text(document: &mut Document, page: usize, value: &str) -> u64 {
    document.insert(
        page,
        NodeKind::Text(text::RichText::new(value)),
        Rect::new(72., 72., 451., 40.),
    )
}
fn xml(bytes: &[u8], path: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name(path).unwrap().read_to_string(&mut xml).unwrap();
    xml
}

#[test]
fn native_mode_rejected_without_truncating_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.docx");
    std::fs::write(&path, b"existing").unwrap();
    let doc = Document::default();
    let error = export::word::write_docx(&path, &doc).unwrap_err();
    assert!(error.to_string().contains("only in Word mode"));
    assert_eq!(std::fs::read(path).unwrap(), b"existing");
}
#[test]
fn editable_unicode_runs_alignment_images_page_sections_and_safe_xml() {
    let mut doc = Document {
        mode: DocumentMode::Word,
        ..Default::default()
    };
    let id = text(
        &mut doc,
        0,
        "Hello & <world> café Українська\n\tSecond paragraph",
    );
    let NodeKind::Text(t) = &mut doc.node_mut(id).unwrap().kind else {
        unreachable!()
    };
    t.align = text::TextAlign::Center;
    let mut style = t.style.clone();
    style.family = "Noto Serif".into();
    style.size = 26.;
    style.weight = 700.;
    style.italic = true;
    style.underline = true;
    style.color = text::Color::rgb(46, 113, 83);
    t.apply_style(0..5, style);
    let mut png = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(100, 50, image::Rgba([40, 120, 80, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    doc.assets
        .insert("picture".into(), Arc::new(png.into_inner()));
    doc.insert(
        0,
        NodeKind::Image {
            asset: "picture".into(),
        },
        Rect::new(72., 240., 200., 100.),
    );
    let id = doc.next_id;
    doc.next_id += 1;
    doc.pages.push(Page {
        id,
        name: "Second page".into(),
        width: 612.,
        height: 792.,
        nodes: vec![],
    });
    text(&mut doc, 1, "Second page preserved");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.docx");
    export::word::write_docx(&path, &doc).unwrap();
    let bytes = std::fs::read(path).unwrap();
    let content = xml(&bytes, "word/document.xml");
    assert!(content.contains("&amp;") && content.contains("&lt;world&gt;"));
    assert!(content.contains("Українська"));
    assert!(content.contains("<w:b") && content.contains("<w:i") && content.contains("<w:u"));
    assert!(content.contains("2E7153"));
    assert!(content.contains("<w:tab"));
    assert!(content.contains("w:val=\"center\""));
    assert!(content.contains("cx=\"2540000\"") && content.contains("cy=\"1270000\""));
    assert!(content.contains("<w:sectPr") && content.contains("nextPage"));
    assert!(content.contains("w:top=\"1440\""));
    let types = xml(&bytes, "[Content_Types].xml");
    assert!(types.contains("image/png"));
    let rels = xml(&bytes, "word/_rels/document.xml.rels");
    assert!(rels.contains("relationships/image"));
    let parsed = docx_rs::read_docx(&bytes).unwrap();
    let value = serde_json::to_string(&parsed).unwrap();
    assert!(value.contains("Second page preserved") && value.contains("Українська"));
    assert!(value.contains("pic"));
}
#[test]
fn effects_and_nested_content_cannot_leak_into_word_mode() {
    let mut doc = Document {
        mode: DocumentMode::Word,
        ..Default::default()
    };
    let id = text(&mut doc, 0, "Editable");
    assert!(doc.validate_word().is_ok());
    doc.node_mut(id).unwrap().opacity = 0.5;
    assert!(doc.validate_word().is_err());
    doc.node_mut(id).unwrap().opacity = 1.;
    let NodeKind::Text(t) = &mut doc.node_mut(id).unwrap().kind else {
        unreachable!()
    };
    t.style.color.a = 128;
    assert!(doc.validate().is_err());
    let NodeKind::Text(t) = &mut doc.node_mut(id).unwrap().kind else {
        unreachable!()
    };
    t.style.color.a = 255;
    t.content = "Invalid \u{1}".into();
    assert!(doc.validate_word().is_err());
    doc.node_mut(id).unwrap().kind = NodeKind::Shape;
    assert!(doc.validate_word().is_err());
}
#[test]
fn oversized_word_text_is_exportable_but_raster_output_never_silently_clips_it() {
    let mut doc = Document {
        mode: DocumentMode::Word,
        ..Default::default()
    };
    let id = text(&mut doc, 0, "Long document\n\nThe text stays editable.");
    doc.node_mut(id).unwrap().frame.height = 900.;
    assert!(doc.validate_word().is_ok());
    assert!(
        export::PageRenderer::default()
            .render(&doc, 0, 144)
            .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    assert!(export::word::write_docx(&dir.path().join("long.docx"), &doc).is_ok());
}

#[test]
fn local_presets_are_editable_and_respect_output_profiles() {
    use document::presets::Preset;
    let dir = tempfile::tempdir().unwrap();
    for preset in Preset::ALL {
        let compose = preset.create(DocumentMode::Compose).unwrap();
        assert!(compose.validate().is_ok());
        assert!(
            export::PageRenderer::default()
                .render(&compose, 0, 72)
                .is_ok()
        );
        let path = dir.path().join(format!("{}.docx", preset.name()));
        assert!(export::word::write_docx(&path, &compose).is_err());
        if !preset.supports(DocumentMode::Word) {
            assert!(preset.create(DocumentMode::Word).is_err());
            assert!(
                compose.pages[0]
                    .nodes
                    .iter()
                    .any(|n| matches!(n.kind, NodeKind::Shape))
            );
        } else {
            let word = preset.create(DocumentMode::Word).unwrap();
            export::word::write_docx(&path, &word).unwrap();
            let parsed = docx_rs::read_docx(&std::fs::read(path).unwrap()).unwrap();
            if let Some(node) = word.pages[0].nodes.first() {
                let NodeKind::Text(text) = &node.kind else {
                    panic!("Preset should start with editable text");
                };
                assert!(
                    serde_json::to_string(&parsed)
                        .unwrap()
                        .contains(&text.content)
                );
            }
        }
    }
}

#[test]
fn preset_text_fits_its_frames_and_photos_keep_original_proportions() {
    use document::{ImageSizing, presets::Preset};
    let mut engine = renderer::TextEngine::new();
    for preset in Preset::ALL {
        let doc = preset.create(DocumentMode::Compose).unwrap();
        let page = &doc.pages[0];
        for node in &page.nodes {
            assert!(
                node.frame.x >= 0.
                    && node.frame.y >= 0.
                    && node.frame.x + node.frame.width <= page.width + 0.1
                    && node.frame.y + node.frame.height <= page.height + 0.1,
                "{}: {} outside page",
                preset.name(),
                node.name
            );
            match &node.kind {
                NodeKind::Text(text) => {
                    let height = engine.layout(text, node.frame.width).height;
                    assert!(
                        height <= node.frame.height + 0.1,
                        "{} / {}: text needs {height} pt, frame only {}",
                        preset.name(),
                        node.name,
                        node.frame.height
                    );
                }
                NodeKind::Image { asset } => {
                    let bytes = &doc.assets[asset];
                    let (w, h) = image::ImageReader::new(Cursor::new(bytes.as_slice()))
                        .with_guessed_format()
                        .unwrap()
                        .into_dimensions()
                        .unwrap();
                    assert_eq!(node.image_sizing, ImageSizing::Scale);
                    assert!(
                        (node.frame.width / node.frame.height - w as f32 / h as f32).abs() < 0.001,
                        "{}: distorted sample image",
                        preset.name()
                    );
                }
                _ => {}
            }
        }
        assert_eq!(
            doc.pages[0]
                .nodes
                .iter()
                .any(|n| matches!(n.kind, NodeKind::Image { .. })),
            preset.has_photo()
        );
    }
}

#[test]
fn text_links_export_as_clickable_word_relationships() {
    let mut doc = Document {
        mode: DocumentMode::Word,
        ..Default::default()
    };
    let id = text(&mut doc, 0, "Read the project page");
    let NodeKind::Text(t) = &mut doc.node_mut(id).unwrap().kind else {
        unreachable!()
    };
    let url = "https://example.com/project?a=1&b=2".to_string();
    t.apply_link(9..21, Some(url));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("links.docx");
    export::word::write_docx(&path, &doc).unwrap();
    let bytes = std::fs::read(path).unwrap();
    assert!(xml(&bytes, "word/document.xml").contains("<w:hyperlink"));
    let relationships = xml(&bytes, "word/_rels/document.xml.rels");
    assert!(relationships.contains("relationships/hyperlink"));
    assert!(relationships.contains("TargetMode=\"External\""));
    assert!(relationships.contains("example.com/project?a=1&amp;b=2"));
}
