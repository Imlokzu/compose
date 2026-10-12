//! Android transport only. The shared Rust engine owns editable documents and history.
use anyhow::{Context, Result, bail, ensure};
use document::{Document, DocumentMode, Node, NodeKind, PathCommand, Point, Rect, ShapeKind};
use editor::{Component, Editor};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path, sync::Arc};

pub struct Session {
    pub editor: Editor,
    text: renderer::TextEngine,
    output: export::PageRenderer,
    saved: Document,
    bitmaps: HashMap<u64, (Node, Vec<u8>)>,
    previews: HashMap<String, Vec<u8>>,
}
impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
impl Session {
    pub fn new() -> Self {
        let document = Document::default();
        Self {
            editor: Editor::new(document.clone()),
            saved: document,
            text: renderer::TextEngine::new(),
            output: export::PageRenderer::default(),
            bitmaps: HashMap::new(),
            previews: HashMap::new(),
        }
    }
    fn measure(&mut self) {
        Self::measure_document(&mut self.editor.document, &mut self.text);
        self.editor.normalize_groups();
    }
    fn measure_document(document: &mut Document, engine: &mut renderer::TextEngine) {
        fn texts(nodes: &mut [Node], engine: &mut renderer::TextEngine, word: bool) {
            for n in nodes {
                if let NodeKind::Text(t) = &n.kind {
                    let height = engine
                        .layout(t, n.frame.width)
                        .height
                        .max(t.style.size * t.style.line_height);
                    n.frame.height = if word {
                        height
                    } else {
                        n.frame.height.max(height)
                    };
                }
                texts(&mut n.children, engine, word);
            }
        }
        let word = document.mode == DocumentMode::Word;
        for page in &mut document.pages {
            if word {
                let mut y = document::WORD_MARGIN;
                for n in &mut page.nodes {
                    n.frame.x = document::WORD_MARGIN;
                    n.frame.y = y;
                    if matches!(n.kind, NodeKind::Text(_)) {
                        n.frame.width = page.width - 2. * document::WORD_MARGIN;
                    }
                    texts(std::slice::from_mut(n), engine, true);
                    if n.visible {
                        y += n.frame.height + document::WORD_BLOCK_GAP;
                    }
                }
            } else {
                texts(&mut page.nodes, engine, false);
            }
        }
    }
    pub fn snapshot(&mut self) -> Value {
        self.measure();
        let mut scene = Vec::new();
        fn collect(
            nodes: &[Node],
            origin: Point,
            opacity: f32,
            depth: usize,
            locked: bool,
            visible: bool,
            scene: &mut Vec<Value>,
        ) {
            for n in nodes {
                let r = Rect::new(
                    origin.x + n.frame.x,
                    origin.y + n.frame.y,
                    n.frame.width,
                    n.frame.height,
                );
                let mut item = json!({"node":n,"bounds":r,"opacity":opacity*n.opacity,"depth":depth,"locked":locked||n.locked,"visible":visible&&n.visible});
                if matches!(n.kind, NodeKind::Shape) {
                    item["path"] = json!(
                        n.shape
                            .path(r, n.radius, n.line_flipped)
                            .into_iter()
                            .map(|cmd| match cmd {
                                PathCommand::Move(p) => json!(["M", p.x, p.y]),
                                PathCommand::Line(p) => json!(["L", p.x, p.y]),
                                PathCommand::Quad(c, p) => json!(["Q", c.x, c.y, p.x, p.y]),
                                PathCommand::Cubic(a, b, p) =>
                                    json!(["C", a.x, a.y, b.x, b.y, p.x, p.y]),
                                PathCommand::Close => json!(["Z"]),
                            })
                            .collect::<Vec<_>>()
                    );
                }
                scene.push(item);
                collect(
                    &n.children,
                    Point { x: r.x, y: r.y },
                    opacity * n.opacity,
                    depth + 1,
                    locked || n.locked,
                    visible && n.visible,
                    scene,
                );
            }
        }
        let page = &self.editor.document.pages[self.editor.page];
        collect(
            &page.nodes,
            Point { x: 0., y: 0. },
            1.,
            0,
            false,
            true,
            &mut scene,
        );
        json!({"ok":true,"document":self.editor.document,"scene":scene,"page":self.editor.page,
            "selected":self.editor.selected(),"undo":self.editor.can_undo(),"redo":self.editor.can_redo(),
            "dirty":self.editor.document!=self.saved})
    }
    pub fn asset(&mut self, key: &str) -> Result<Vec<u8>> {
        if let Some(name) = key.strip_prefix("preset:") {
            if let Some(bytes) = self.previews.get(key) {
                return Ok(bytes.clone());
            }
            let (mode, name) = name.split_once(':').context("Missing preset mode")?;
            let mut doc = preset(name)
                .create(if mode == "Word" {
                    DocumentMode::Word
                } else {
                    DocumentMode::Compose
                })
                .map_err(anyhow::Error::msg)?;
            Self::measure_document(&mut doc, &mut self.text);
            let bytes = self.output.render(&doc, 0, 72)?.encode_png()?;
            self.previews.insert(key.into(), bytes.clone());
            Ok(bytes)
        } else if let Some(id) = key.strip_prefix("text:") {
            let id = id.parse::<u64>()?;
            let node = self
                .editor
                .document
                .node(id)
                .context("Text layer was removed")?
                .clone();
            if let Some((old, bytes)) = self.bitmaps.get(&id)
                && old == &node
            {
                return Ok(bytes.clone());
            }
            let bitmap =
                self.output
                    .render_text(&node.kind, node.frame.width, node.frame.height, 144)?;
            let bytes = bitmap.encode_png()?;
            self.bitmaps.insert(id, (node, bytes.clone()));
            Ok(bytes)
        } else {
            Ok(self
                .editor
                .document
                .assets
                .get(key)
                .context("Image unavailable")?
                .as_ref()
                .clone())
        }
    }
    pub fn command(&mut self, arg: &Value) -> Result<Value> {
        let before = self.editor.document.clone();
        let page = self.editor.page;
        let selected = self.editor.selected();
        let saved = self.saved.clone();
        let result = self.apply_command(arg);
        if result.is_err() {
            self.editor.document = before;
            self.editor.page = page;
            self.editor.select_only(selected);
            self.saved = saved;
        }
        result
    }
    fn apply_command(&mut self, arg: &Value) -> Result<Value> {
        let op = arg["op"].as_str().context("Missing command")?;
        match op {
            "state" => {}
            "new" => {
                let preset = preset(arg["preset"].as_str().unwrap_or("Blank"));
                let mode = if arg["mode"] == "Word" {
                    DocumentMode::Word
                } else {
                    DocumentMode::Compose
                };
                ensure!(preset.supports(mode), "This layout needs Compose mode");
                let doc = preset.create(mode).map_err(anyhow::Error::msg)?;
                self.editor = Editor::new(doc.clone());
                self.saved = doc;
                self.bitmaps.clear();
            }
            "select" => self.editor.select_only(arg["id"].as_u64()),
            "hit" => self.editor.select_only(self.editor.hit_test(Point {
                x: num(arg, "x")?,
                y: num(arg, "y")?,
            })),
            "page" => {
                let page = arg["index"].as_u64().context("Missing page")? as usize;
                ensure!(page < self.editor.document.pages.len(), "Page unavailable");
                self.editor.page = page;
                self.editor.select_only(None);
            }
            "add_page" => {
                self.editor.checkpoint();
                let id = self.editor.document.next_id;
                self.editor.document.next_id += 1;
                self.editor.document.pages.push(document::Page {
                    id,
                    name: format!("Page {}", self.editor.document.pages.len() + 1),
                    width: 595.28,
                    height: 841.89,
                    nodes: vec![],
                });
                self.editor.page = self.editor.document.pages.len() - 1;
                self.editor.select_only(None);
            }
            "begin" => self.editor.begin_batch(),
            "end" => self.editor.end_batch(),
            "undo" => {
                self.editor.end_batch();
                self.editor.undo();
            }
            "redo" => {
                self.editor.end_batch();
                self.editor.redo();
            }
            "delete" => self.editor.delete_selected(),
            "duplicate" => self.editor.duplicate_selected(),
            "order" => self
                .editor
                .reorder_selected(arg["forward"].as_bool().unwrap_or(true)),
            "visibility" | "lock" => {
                let id = arg["id"].as_u64().context("Select a layer")?;
                ensure!(self.editor.document.node(id).is_some(), "Layer unavailable");
                if op == "lock" {
                    self.editor.toggle_lock(id);
                } else {
                    self.editor.toggle_visibility(id);
                }
            }
            "component" => {
                ensure!(
                    self.editor.document.mode == DocumentMode::Compose,
                    "Components need Compose mode"
                );
                let component = match arg["kind"].as_str().context("Choose a component")? {
                    "Title" => Component::Title,
                    "Callout" => Component::Callout,
                    "Quote" => Component::Quote,
                    "Stat" => Component::Stat,
                    "Card" => Component::Card,
                    "Divider" => Component::Divider,
                    _ => bail!("Unknown component"),
                };
                let id = self.editor.insert_component(component, 56., 100.);
                self.editor.select_only(Some(id));
            }
            "image_sizing" => {
                self.guard()?;
                ensure!(
                    self.editor.document.mode == DocumentMode::Compose,
                    "Canvas sizing needs Compose mode"
                );
                let NodeKind::Image { asset } = &self.editor.selected_node().unwrap().kind else {
                    bail!("Select an image")
                };
                let sizing: document::ImageSizing = serde_json::from_value(arg["mode"].clone())?;
                let (_, w, h) = image_tools::validate_input(&self.editor.document.assets[asset])?;
                self.editor.checkpoint();
                let n = self.editor.selected_node_mut().unwrap();
                n.image_sizing = sizing;
                if sizing == document::ImageSizing::Scale {
                    n.frame.height = n.frame.width * h as f32 / w as f32;
                }
            }
            "scale" => {
                self.guard()?;
                ensure!(
                    self.editor.document.mode == DocumentMode::Compose,
                    "Canvas sizing needs Compose mode"
                );
                let factor = num(arg, "factor")?;
                ensure!((0.25..=4.).contains(&factor), "Invalid scale");
                let id = self.editor.selected().unwrap();
                let mut r = self.editor.selection_bounds().context("Select a layer")?;
                r.x -= r.width * (factor - 1.) / 2.;
                r.y -= r.height * (factor - 1.) / 2.;
                r.width *= factor;
                r.height *= factor;
                ensure!(r.valid(), "Invalid size");
                self.editor.checkpoint();
                self.editor.apply_frames(&[(id, r)]);
            }
            "align_page" => {
                self.guard()?;
                ensure!(
                    self.editor.document.mode == DocumentMode::Compose,
                    "Word blocks follow reading order"
                );
                let id = self.editor.selected().unwrap();
                let mut r = self.editor.selection_bounds().context("Select a layer")?;
                let page = &self.editor.document.pages[self.editor.page];
                match arg["alignment"].as_str() {
                    Some("left") => r.x = 0.,
                    Some("center_x") => r.x = (page.width - r.width) / 2.,
                    Some("right") => r.x = page.width - r.width,
                    Some("top") => r.y = 0.,
                    Some("center_y") => r.y = (page.height - r.height) / 2.,
                    Some("bottom") => r.y = page.height - r.height,
                    _ => bail!("Unknown alignment"),
                }
                self.editor.checkpoint();
                self.editor.apply_frames(&[(id, r)]);
            }
            "insert" => {
                let ty = arg["kind"].as_str().unwrap_or("Text");
                let word = self.editor.document.mode == DocumentMode::Word;
                ensure!(
                    !word || ty == "Text",
                    "Figures are available in Compose mode"
                );
                let shape: ShapeKind = serde_json::from_value(json!(ty)).unwrap_or_default();
                let (w, h) = if ty == "Text" {
                    (280., 32.)
                } else {
                    shape.default_size()
                };
                let r = Rect::new(
                    arg["x"].as_f64().unwrap_or(64.) as f32,
                    arg["y"].as_f64().unwrap_or(100.) as f32,
                    w,
                    h,
                );
                let kind = if ty == "Text" {
                    NodeKind::Text(text::RichText::new(""))
                } else if ty == "Frame" {
                    NodeKind::Frame
                } else {
                    NodeKind::Shape
                };
                self.editor.insert(kind, r);
                let n = self.editor.selected_node_mut().unwrap();
                if matches!(n.kind, NodeKind::Shape) {
                    n.shape = shape;
                    n.name = shape.label().into();
                    if shape == ShapeKind::Line {
                        n.fill = document::Color::rgb(35, 38, 36);
                        n.appearance.stroke_width = 2.;
                    }
                }
            }
            "frame" => {
                self.guard()?;
                ensure!(
                    self.editor.document.mode == DocumentMode::Compose,
                    "Word blocks follow reading order"
                );
                let id = self.editor.selected().unwrap();
                let mut r: Rect = serde_json::from_value(arg["frame"].clone())?;
                ensure!(r.valid(), "Invalid size");
                let n = self.editor.selected_node().unwrap();
                if matches!(n.kind, NodeKind::Shape) && n.shape == ShapeKind::Circle {
                    if arg["axis"] == "height" {
                        r.width = r.height;
                    } else {
                        r.height = r.width;
                    }
                }
                if matches!(n.kind, NodeKind::Image { .. })
                    && n.image_sizing == document::ImageSizing::Scale
                {
                    if arg["axis"] == "height" {
                        r.width = r.height / n.frame.height * n.frame.width;
                    } else {
                        r.height = r.width / n.frame.width * n.frame.height;
                    }
                }
                self.editor.checkpoint();
                self.editor.apply_frames(&[(id, r)]);
            }
            "text" => {
                self.guard()?;
                let content = arg["content"].as_str().context("Missing text")?;
                ensure!(content.len() <= 1_000_000, "Text is too long");
                self.editor.checkpoint();
                let n = self.editor.selected_node_mut().unwrap();
                let NodeKind::Text(t) = &mut n.kind else {
                    bail!("Select text")
                };
                let prefix = t
                    .content
                    .chars()
                    .zip(content.chars())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum::<usize>();
                let suffix = t.content[prefix..]
                    .chars()
                    .rev()
                    .zip(content[prefix..].chars().rev())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum::<usize>();
                t.replace(
                    prefix..t.content.len() - suffix,
                    &content[prefix..content.len() - suffix],
                );
            }
            "format" => {
                self.guard()?;
                self.editor.checkpoint();
                let n = self.editor.selected_node_mut().unwrap();
                let NodeKind::Text(t) = &mut n.kind else {
                    bail!("Select text")
                };
                let start =
                    text::utf16_to_byte(&t.content, arg["start"].as_u64().unwrap_or(0) as usize);
                let end = text::utf16_to_byte(
                    &t.content,
                    arg["end"]
                        .as_u64()
                        .unwrap_or(t.content.encode_utf16().count() as u64)
                        as usize,
                );
                if let Some(alignment) = arg["alignment"].as_str() {
                    t.align = serde_json::from_value(json!(alignment))?;
                }
                let initial = t.style_at(start.min(end));
                let patch = |mut style: text::TextStyle| -> Result<text::TextStyle> {
                    if arg["bold_toggle"] == true {
                        style.weight = if initial.weight >= 600. { 400. } else { 700. };
                    }
                    if arg["italic_toggle"] == true {
                        style.italic = !initial.italic;
                    }
                    if arg["underline_toggle"] == true {
                        style.underline = !initial.underline;
                    }
                    if let Some(v) = arg["size"].as_f64() {
                        style.size = v as f32;
                    }
                    if let Some(v) = arg["family"].as_str() {
                        style.family = v.into();
                    }
                    if let Some(v) = arg["weight"].as_f64() {
                        style.weight = v as f32;
                    }
                    if let Some(v) = arg["italic"].as_bool() {
                        style.italic = v;
                    }
                    if let Some(v) = arg["underline"].as_bool() {
                        style.underline = v;
                    }
                    if arg["color"].is_object() {
                        style.color = serde_json::from_value(arg["color"].clone())?;
                    }
                    if let Some(v) = arg["link"].as_str() {
                        style.link = if v.is_empty() {
                            None
                        } else {
                            Some(text::links::normalize(v).map_err(anyhow::Error::msg)?)
                        };
                    }
                    renderer::normalize_style(&mut style);
                    ensure!(style.valid(), "Invalid text style");
                    Ok(style)
                };
                if start == end {
                    t.style = patch(t.style.clone())?;
                } else {
                    // Patch each existing run, retaining unrelated fonts, links and emphasis.
                    let mut boundaries = vec![start.min(end), start.max(end)];
                    for span in &t.spans {
                        for b in [span.range.start, span.range.end] {
                            if b > start.min(end) && b < start.max(end) {
                                boundaries.push(b);
                            }
                        }
                    }
                    boundaries.sort_unstable();
                    boundaries.dedup();
                    let runs = boundaries
                        .windows(2)
                        .map(|b| Ok((b[0]..b[1], patch(t.style_at(b[0]))?)))
                        .collect::<Result<Vec<_>>>()?;
                    for (range, style) in runs {
                        t.apply_style(range, style);
                    }
                }
            }
            "style" => {
                self.guard()?;
                ensure!(
                    self.editor.document.mode == DocumentMode::Compose,
                    "Canvas styles need Compose mode"
                );
                self.editor.checkpoint();
                let n = self.editor.selected_node_mut().unwrap();
                if let Some(v) = arg["radius"].as_f64() {
                    n.radius = (v as f32).clamp(0., n.frame.width.min(n.frame.height) / 2.);
                }
                if let Some(v) = arg["opacity"].as_f64() {
                    n.opacity = (v as f32).clamp(0., 1.);
                }
                if arg["color"].is_object() {
                    n.fill = serde_json::from_value(arg["color"].clone())?;
                }
                if let Some(v) = arg["stroke"].as_f64() {
                    n.appearance.stroke_width = (v as f32).clamp(0., 100.);
                }
                if let Some(v) = arg["shadow"].as_bool() {
                    n.appearance.shadow = v;
                }
                if let Some(v) = arg["name"].as_str() {
                    n.name = v.chars().take(160).collect();
                }
                if let Some(v) = arg["shape"].as_str() {
                    n.shape = serde_json::from_value(json!(v))?;
                    if n.shape == ShapeKind::Circle {
                        n.frame.height = n.frame.width;
                    }
                    if n.shape == ShapeKind::Line {
                        n.appearance.stroke_width = n.appearance.stroke_width.max(2.);
                    }
                }
            }
            "image" => {
                let path = arg["path"].as_str().context("Missing image")?;
                ensure!(
                    std::fs::metadata(path)?.len() <= 25 * 1024 * 1024,
                    "Image exceeds 25 MB"
                );
                let bytes = std::fs::read(path)?;
                let (_, w, h) = image_tools::validate_input(&bytes)?;
                let key = format!("{:x}", Sha256::digest(&bytes));
                self.editor.checkpoint();
                self.editor
                    .document
                    .assets
                    .insert(key.clone(), Arc::new(bytes));
                let width = 240.;
                self.editor.insert(
                    NodeKind::Image { asset: key },
                    Rect::new(64., 120., width, width * h as f32 / w as f32),
                );
            }
            "remove_background" => {
                self.guard()?;
                let NodeKind::Image { asset } = &self.editor.selected_node().unwrap().kind else {
                    bail!("Select an image")
                };
                let bytes = self.editor.document.assets[asset].clone();
                let result = image_tools::remove_shared(
                    &bytes,
                    "https://compose-background.onrender.com/v1/background/remove",
                )?;
                let key = format!("{:x}", Sha256::digest(&result));
                self.editor.checkpoint();
                self.editor
                    .document
                    .assets
                    .insert(key.clone(), Arc::new(result));
                self.editor.selected_node_mut().unwrap().kind = NodeKind::Image { asset: key };
            }
            "mark_saved" => {
                self.saved = self.editor.document.clone();
            }
            "save" => {
                self.measure();
                let path = arg["path"].as_str().context("Missing save location")?;
                file_format::save(Path::new(path), &self.editor.document)?;
                if !arg["copy"].as_bool().unwrap_or(false) {
                    self.saved = self.editor.document.clone();
                }
            }
            "load" => {
                let path = arg["path"].as_str().context("Missing file")?;
                let doc = file_format::load(Path::new(path))?;
                self.editor = Editor::new(doc.clone());
                self.saved = doc;
                self.bitmaps.clear();
            }
            "png" => {
                self.measure();
                export::write_png(
                    Path::new(arg["path"].as_str().context("Missing export path")?),
                    &self.editor.document,
                    self.editor.page,
                    144,
                )?;
            }
            "docx" => {
                self.measure();
                export::word::write_docx(
                    Path::new(arg["path"].as_str().context("Missing export path")?),
                    &self.editor.document,
                )?;
            }
            _ => bail!("Unknown command"),
        }
        self.measure();
        self.editor
            .document
            .validate()
            .map_err(anyhow::Error::msg)?;
        Ok(self.snapshot())
    }
    fn guard(&self) -> Result<()> {
        ensure!(self.editor.selected_node().is_some(), "Select one layer");
        ensure!(!self.editor.selection_locked(), "This layer is locked");
        Ok(())
    }
}
fn preset(name: &str) -> document::presets::Preset {
    use document::presets::Preset;
    match name {
        "Resume" => Preset::Resume,
        "PhotoResume" => Preset::PhotoResume,
        "PhotoCover" => Preset::PhotoCover,
        "Portfolio" => Preset::Portfolio,
        "Report" => Preset::Report,
        "Letter" => Preset::Letter,
        "Poster" => Preset::Poster,
        _ => Preset::Blank,
    }
}
fn num(value: &Value, key: &str) -> Result<f32> {
    let n = value[key].as_f64().context("Missing coordinate")? as f32;
    ensure!(n.is_finite(), "Invalid coordinate");
    Ok(n)
}

#[cfg(target_os = "android")]
mod jni_api {
    use super::*;
    use jni::{
        JNIEnv,
        objects::{JClass, JString},
        sys::{jbyteArray, jlong, jstring},
    };
    use std::sync::{
        Mutex, OnceLock,
        atomic::{AtomicI64, Ordering},
    };
    static SESSIONS: OnceLock<Mutex<HashMap<i64, Session>>> = OnceLock::new();
    static NEXT: AtomicI64 = AtomicI64::new(1);
    fn sessions() -> &'static Mutex<HashMap<i64, Session>> {
        SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_app_compose_mobile_NativeBridge_create(
        _env: JNIEnv,
        _class: JClass,
    ) -> jlong {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut sessions) = sessions().lock() {
            sessions.insert(id, Session::new());
            id
        } else {
            0
        }
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_app_compose_mobile_NativeBridge_call(
        mut env: JNIEnv,
        _class: JClass,
        id: jlong,
        arg: JString,
    ) -> jstring {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Value> {
            let source: String = env.get_string(&arg)?.into();
            ensure!(source.len() < 2_000_000, "Command too large");
            let value = serde_json::from_str(&source)?;
            sessions()
                .lock()
                .map_err(|_| anyhow::anyhow!("Engine unavailable"))?
                .get_mut(&id)
                .context("Session closed")?
                .command(&value)
        }));
        let response = match result {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => json!({"ok":false,"error":e.to_string()}),
            Err(_) => json!({"ok":false,"error":"The operation could not complete"}),
        };
        env.new_string(response.to_string())
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_app_compose_mobile_NativeBridge_asset(
        mut env: JNIEnv,
        _class: JClass,
        id: jlong,
        arg: JString,
    ) -> jbyteArray {
        let result = (|| -> Result<Vec<u8>> {
            let key: String = env.get_string(&arg)?.into();
            sessions()
                .lock()
                .map_err(|_| anyhow::anyhow!("Engine unavailable"))?
                .get_mut(&id)
                .context("Session closed")?
                .asset(&key)
        })();
        env.byte_array_from_slice(&result.unwrap_or_default())
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_app_compose_mobile_NativeBridge_close(
        _env: JNIEnv,
        _class: JClass,
        id: jlong,
    ) {
        if let Ok(mut sessions) = sessions().lock() {
            sessions.remove(&id);
        }
    }
}
