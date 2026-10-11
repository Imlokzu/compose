use anyhow::{Context, Result, bail, ensure};
use document::Document;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
const MAX_JSON: u64 = 16 * 1024 * 1024;
const MAX_ASSET: u64 = 64 * 1024 * 1024;
const MAX_TOTAL: u64 = 256 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    app_version: String,
}
fn asset_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() < 128
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
/// Write to a sibling temporary file, sync it, then atomically replace the destination.
pub fn save(path: &Path, doc: &Document) -> Result<()> {
    doc.validate().map_err(anyhow::Error::msg)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let json = serde_json::to_vec(doc)?;
    ensure!(json.len() as u64 <= MAX_JSON, "Document data is too large");
    let total = doc.assets.values().map(|b| b.len() as u64).sum::<u64>();
    ensure!(
        total <= MAX_TOTAL,
        "Images exceed the 256 MB document limit"
    );
    for (key, bytes) in &doc.assets {
        ensure!(asset_key(key), "Invalid image identifier");
        ensure!(bytes.len() as u64 <= MAX_ASSET, "Image exceeds 64 MB");
    }
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).context("Cannot create a temporary save file")?;
    {
        let mut zip = ZipWriter::new(temp.as_file_mut());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)?;
        zip.write_all(&serde_json::to_vec(&Manifest {
            format: "compose".into(),
            version: 1,
            app_version: env!("CARGO_PKG_VERSION").into(),
        })?)?;
        zip.start_file("document.json", options)?;
        zip.write_all(&json)?;
        for (key, bytes) in &doc.assets {
            zip.start_file(
                format!("assets/{key}"),
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
    }
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .context("Cannot replace the destination file")?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn load(path: &Path) -> Result<Document> {
    let file = File::open(path).context("Cannot open this document")?;
    let mut zip = ZipArchive::new(file).context("Not a valid Compose container")?;
    ensure!(zip.len() <= 10_002, "Too many files in document");
    let mut total = 0u64;
    let mut names = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let f = zip.by_index(i)?;
        ensure!(
            names.insert(f.name().to_string()),
            "Duplicate container entry"
        );
        let limit = if f.name().starts_with("assets/") {
            MAX_ASSET
        } else {
            MAX_JSON
        };
        ensure!(f.size() <= limit, "Container entry exceeds size limit");
        total = total.checked_add(f.size()).context("Invalid size")?;
        ensure!(total <= MAX_TOTAL + 2 * MAX_JSON, "Document is too large");
    }
    fn entry(zip: &mut ZipArchive<File>, name: &str, limit: u64) -> Result<Vec<u8>> {
        let f = zip
            .by_name(name)
            .with_context(|| format!("Missing {name}"))?;
        ensure!(f.size() <= limit, "Entry too large");
        let mut bytes = vec![];
        f.take(limit + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 <= limit, "Entry too large");
        Ok(bytes)
    }
    let manifest: Manifest = serde_json::from_slice(&entry(&mut zip, "manifest.json", 64 * 1024)?)?;
    ensure!(manifest.format == "compose", "Not a Compose document");
    ensure!(
        manifest.version == 1,
        "This document uses unsupported format version {}",
        manifest.version
    );
    let mut doc: Document = serde_json::from_slice(&entry(&mut zip, "document.json", MAX_JSON)?)?;
    for name in names {
        if let Some(key) = name.strip_prefix("assets/") {
            if !asset_key(key) {
                bail!("Invalid image identifier")
            }
            let bytes = entry(&mut zip, &name, MAX_ASSET)?;
            doc.assets.insert(key.to_string(), Arc::new(bytes));
        }
    }
    doc.validate().map_err(anyhow::Error::msg)?;
    Ok(doc)
}
#[cfg(test)]
mod tests {
    use super::*;
    use document::{NodeKind, Rect};
    #[test]
    fn roundtrip_and_failed_save_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.compose");
        let mut d = Document::default();
        d.assets.insert("image1".into(), Arc::new(vec![1, 2, 3, 4]));
        d.insert(
            0,
            NodeKind::Image {
                asset: "image1".into(),
            },
            Rect::new(41., 52., 128., 96.),
        );
        save(&p, &d).unwrap();
        assert_eq!(load(&p).unwrap(), d);
        let bytes = std::fs::read(&p).unwrap();
        d.pages[0].nodes[0].frame.width = f32::NAN;
        assert!(save(&p, &d).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), bytes);
    }
    #[test]
    fn corrupt_input_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.compose");
        std::fs::write(&p, "not a zip").unwrap();
        assert!(load(&p).is_err());
    }
    #[test]
    fn missing_asset_rejected() {
        let mut d = Document::default();
        d.insert(
            0,
            NodeKind::Image {
                asset: "missing".into(),
            },
            Rect::new(0., 0., 20., 20.),
        );
        let dir = tempfile::tempdir().unwrap();
        assert!(save(&dir.path().join("a.compose"), &d).is_err());
    }
}

#[cfg(test)]
mod format_validation_tests {
    use super::*;
    use document::{NodeKind, Rect};
    #[test]
    fn word_mode_is_saved_and_old_json_defaults_to_compose() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("word.compose");
        let doc = Document {
            mode: document::DocumentMode::Word,
            ..Default::default()
        };
        save(&path, &doc).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
        let mut data = serde_json::to_value(Document::default()).unwrap();
        data.as_object_mut().unwrap().remove("mode");
        let legacy: Document = serde_json::from_value(data).unwrap();
        assert_eq!(legacy.mode, document::DocumentMode::Compose);
    }
    #[test]
    fn mixed_text_roundtrip() {
        let mut doc = Document::default();
        let mut rich = text::RichText::new("Small BIG 👩‍💻\nSecond paragraph");
        let mut style = rich.style.clone();
        style.size = 42.;
        style.weight = 700.;
        style.italic = true;
        style.underline = true;
        style.color = text::Color::rgb(20, 80, 130);
        rich.apply_style(6..9, style);
        rich.align = text::TextAlign::Center;
        doc.insert(0, NodeKind::Text(rich), Rect::new(42., 36., 400., 180.));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.compose");
        save(&path, &doc).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
    }
    fn raw_file(version: u32, missing_asset: bool) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        {
            let mut zip = ZipWriter::new(file.as_file_mut());
            zip.start_file("manifest.json", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(
                format!(r#"{{"format":"compose","version":{version},"app_version":"test"}}"#)
                    .as_bytes(),
            )
            .unwrap();
            let mut doc = Document::default();
            if missing_asset {
                doc.insert(
                    0,
                    NodeKind::Image {
                        asset: "absent".into(),
                    },
                    Rect::new(0., 0., 10., 10.),
                );
            }
            zip.start_file("document.json", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&serde_json::to_vec(&doc).unwrap()).unwrap();
            zip.finish().unwrap();
        }
        file
    }
    #[test]
    fn future_version_rejected() {
        let file = raw_file(999, false);
        assert!(
            load(file.path())
                .unwrap_err()
                .to_string()
                .contains("unsupported format version")
        );
    }
    #[test]
    fn missing_embedded_asset_rejected() {
        let file = raw_file(1, true);
        assert!(
            load(file.path())
                .unwrap_err()
                .to_string()
                .contains("Missing image")
        );
    }
}

#[cfg(test)]
mod image_property_tests {
    use super::*;
    use document::{ImageSizing, NodeKind, Rect};
    #[test]
    fn image_resize_mode_and_radius_roundtrip() {
        let mut doc = Document::default();
        doc.assets.insert("image".into(), Arc::new(vec![1, 2, 3]));
        let id = doc.insert(
            0,
            NodeKind::Image {
                asset: "image".into(),
            },
            Rect::new(40., 80., 320., 160.),
        );
        let n = doc.node_mut(id).unwrap();
        n.image_sizing = ImageSizing::Stretch;
        n.radius = 24.;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.compose");
        save(&path, &doc).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
    }
    #[test]
    fn older_nodes_default_to_proportional_scaling() {
        let mut doc = Document::default();
        doc.insert(0, NodeKind::Shape, Rect::new(0., 0., 40., 20.));
        let mut json = serde_json::to_value(doc).unwrap();
        json["pages"][0]["nodes"][0]
            .as_object_mut()
            .unwrap()
            .remove("image_sizing");
        let legacy: Document = serde_json::from_value(json).unwrap();
        assert_eq!(legacy.pages[0].nodes[0].image_sizing, ImageSizing::Scale);
        assert!(legacy.validate().is_ok());
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use document::{Color, NodeKind, Rect};
    #[test]
    fn appearance_roundtrip_and_validation() {
        let mut doc = Document::default();
        let id = doc.insert(0, NodeKind::Shape, Rect::new(10., 20., 120., 80.));
        let n = doc.node_mut(id).unwrap();
        n.fill = Color {
            r: 10,
            g: 90,
            b: 170,
            a: 127,
        };
        n.appearance.stroke_width = 2.5;
        n.appearance.stroke = Color::rgb(60, 30, 90);
        n.appearance.shadow = true;
        n.appearance.shadow_blur = 18.;
        n.appearance.shadow_y = -3.;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("styled.compose");
        save(&path, &doc).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
        doc.node_mut(id).unwrap().appearance.shadow_blur = f32::NAN;
        assert!(save(&path, &doc).is_err());
    }
    #[test]
    fn legacy_appearance_has_safe_defaults() {
        let mut doc = Document::default();
        doc.insert(0, NodeKind::Shape, Rect::new(0., 0., 100., 100.));
        let mut value = serde_json::to_value(doc).unwrap();
        value["pages"][0]["nodes"][0]
            .as_object_mut()
            .unwrap()
            .remove("appearance");
        let doc: Document = serde_json::from_value(value).unwrap();
        assert_eq!(doc.pages[0].nodes[0].appearance.stroke_width, 0.);
        assert!(!doc.pages[0].nodes[0].appearance.shadow);
        assert!(doc.validate().is_ok());
    }
}

#[cfg(test)]
mod attribution_tests {
    use super::*;
    use document::{ImageSource, NodeKind, Rect};
    #[test]
    fn image_pixels_and_source_metadata_survive_reopening() {
        let mut d = Document::default();
        d.assets.insert("photo".into(), Arc::new(vec![1, 2, 3, 4]));
        let id = d.insert(
            0,
            NodeKind::Image {
                asset: "photo".into(),
            },
            Rect::new(64., 120., 300., 200.),
        );
        d.node_mut(id).unwrap().image_source = Some(ImageSource {
            provider: "Wikimedia Commons".into(),
            page_url: "https://commons.wikimedia.org/wiki/File:Photo.jpg".into(),
            original_url: "https://upload.wikimedia.org/photo.jpg".into(),
            author: "A photographer".into(),
            license: "CC BY 4.0".into(),
            license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
            credit: "Own work".into(),
        });
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("photo.compose");
        save(&p, &d).unwrap();
        assert_eq!(load(&p).unwrap(), d);
    }
}
