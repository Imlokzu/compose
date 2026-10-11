use serde::{Deserialize, Serialize};
pub mod presets;
mod shapes;
pub use shapes::{PathCommand, ShapeKind};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
pub use text::Color;
pub type NodeId = u64;
pub const WORD_MARGIN: f32 = 72.;
pub const WORD_BLOCK_GAP: f32 = 12.;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentMode {
    #[default]
    Compose,
    Word,
}
impl DocumentMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compose => "Compose",
            Self::Word => "Word",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.x + self.width && p.y <= self.y + self.height
    }
    pub fn valid(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite() && v.abs() < 1e6)
            && self.width > 0.
            && self.height > 0.
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    Text(text::RichText),
    Image { asset: String },
    Shape,
    Frame,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageSizing {
    #[default]
    Scale,
    Stretch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub stroke: Color,
    pub stroke_width: f32,
    pub shadow: bool,
    pub shadow_blur: f32,
    pub shadow_y: f32,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            stroke: Color::rgb(35, 38, 36),
            stroke_width: 0.,
            shadow: false,
            shadow_blur: 12.,
            shadow_y: 4.,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageSource {
    pub provider: String,
    pub page_url: String,
    pub original_url: String,
    pub author: String,
    pub license: String,
    pub license_url: String,
    pub credit: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub frame: Rect,
    pub opacity: f32,
    pub fill: Color,
    pub radius: f32,
    #[serde(default)]
    pub shape: ShapeKind,
    #[serde(default)]
    pub line_flipped: bool,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub image_sizing: ImageSizing,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_source: Option<ImageSource>,
    pub visible: bool,
    pub locked: bool,
    pub kind: NodeKind,
    #[serde(default)]
    pub is_group: bool,
    pub children: Vec<Node>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub nodes: Vec<Node>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    #[serde(default)]
    pub mode: DocumentMode,
    pub name: String,
    pub pages: Vec<Page>,
    #[serde(skip)]
    pub assets: BTreeMap<String, Arc<Vec<u8>>>,
    pub next_id: u64,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            mode: DocumentMode::Compose,
            name: "Untitled".into(),
            pages: vec![Page {
                id: 1,
                name: "Page 1".into(),
                width: 595.28,
                height: 841.89,
                nodes: vec![],
            }],
            assets: BTreeMap::new(),
            next_id: 2,
        }
    }
}
fn find(nodes: &[Node], id: NodeId) -> Option<&Node> {
    for n in nodes {
        if n.id == id {
            return Some(n);
        }
        if let Some(found) = find(&n.children, id) {
            return Some(found);
        }
    }
    None
}
fn find_mut(nodes: &mut [Node], id: NodeId) -> Option<&mut Node> {
    for n in nodes {
        if n.id == id {
            return Some(n);
        }
        if let Some(found) = find_mut(&mut n.children, id) {
            return Some(found);
        }
    }
    None
}
impl Document {
    pub fn word_overflows(&self, page: usize) -> bool {
        self.mode == DocumentMode::Word
            && self.pages.get(page).is_some_and(|p| {
                p.nodes
                    .iter()
                    .any(|n| n.visible && n.frame.y + n.frame.height > p.height - WORD_MARGIN + 0.1)
            })
    }
    pub fn word_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for (index, page) in self.pages.iter().enumerate() {
            if page.width <= 2. * WORD_MARGIN + 40.
                || page.height <= 2. * WORD_MARGIN + 40.
                || page.width > 1584.
                || page.height > 1584.
            {
                issues.push(format!(
                    "Page {} must fit Word's page sizes with 1-inch margins",
                    index + 1
                ));
            }
            for node in &page.nodes {
                if !matches!(node.kind, NodeKind::Text(_) | NodeKind::Image { .. })
                    || !node.children.is_empty()
                    || node.is_group
                {
                    issues.push(format!(
                        "{} uses canvas shapes, frames or groups",
                        node.name
                    ));
                    continue;
                }
                if node.opacity != 1.
                    || node.radius != 0.
                    || node.appearance.stroke_width != 0.
                    || node.appearance.shadow
                    || node.image_sizing != ImageSizing::Scale
                {
                    issues.push(format!(
                        "{} uses canvas effects or image stretching",
                        node.name
                    ));
                }
                if let NodeKind::Text(text) = &node.kind {
                    if text.style.color.a != 255
                        || text.spans.iter().any(|s| s.style.color.a != 255)
                    {
                        issues.push(format!("{} uses transparent text color", node.name));
                    }
                    if text
                        .content
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                    {
                        issues.push(format!(
                            "{} contains text characters Word cannot store",
                            node.name
                        ));
                    }
                }
            }
            if issues.len() > 20 {
                break;
            }
        }
        issues
    }
    pub fn validate_word(&self) -> Result<(), String> {
        if self.mode != DocumentMode::Word {
            return Err("Word export is available only in Word mode".into());
        }
        if let Some(issue) = self.word_issues().first() {
            return Err(issue.clone());
        }
        Ok(())
    }
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.pages.iter().find_map(|p| find(&p.nodes, id))
    }
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.pages
            .iter_mut()
            .find_map(|p| find_mut(&mut p.nodes, id))
    }
    pub fn insert(&mut self, page: usize, kind: NodeKind, frame: Rect) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let name = match &kind {
            NodeKind::Text(_) => "Text",
            NodeKind::Image { .. } => "Image",
            NodeKind::Shape => "Rectangle",
            NodeKind::Frame => "Frame",
        }
        .into();
        self.pages[page].nodes.push(Node {
            id,
            name,
            frame,
            opacity: 1.,
            fill: Color::rgb(211, 222, 201),
            radius: 0.,
            shape: ShapeKind::Rectangle,
            line_flipped: false,
            appearance: Appearance::default(),
            image_sizing: ImageSizing::Scale,
            image_source: None,
            visible: true,
            locked: false,
            kind,
            is_group: false,
            children: vec![],
        });
        id
    }
    pub fn remove(&mut self, id: NodeId) {
        fn remove(nodes: &mut Vec<Node>, id: NodeId) {
            nodes.retain(|n| n.id != id);
            for n in nodes {
                remove(&mut n.children, id)
            }
        }
        for p in &mut self.pages {
            remove(&mut p.nodes, id)
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.mode == DocumentMode::Word {
            self.validate_word()?;
        }
        if self.pages.is_empty() || self.pages.len() > 500 {
            return Err("Document must contain 1–500 pages".into());
        }
        let mut ids = HashSet::new();
        let mut count = 0;
        fn check(
            nodes: &[Node],
            depth: usize,
            ids: &mut HashSet<u64>,
            count: &mut usize,
            doc: &Document,
        ) -> Result<(), String> {
            if depth > 64 {
                return Err("Layers are nested too deeply".into());
            }
            for n in nodes {
                *count += 1;
                if n.is_group && (!matches!(n.kind, NodeKind::Frame) || n.children.is_empty()) {
                    return Err("A group must contain layers in a frame".into());
                }
                if *count > 100_000 {
                    return Err("Too many objects".into());
                }
                if matches!(n.kind, NodeKind::Shape)
                    && n.shape == ShapeKind::Circle
                    && (n.frame.width - n.frame.height).abs() > 0.001
                {
                    return Err("A circle must have equal width and height".into());
                }
                if !ids.insert(n.id)
                    || !n.frame.valid()
                    || !n.opacity.is_finite()
                    || !(0.0..=1.).contains(&n.opacity)
                    || !n.radius.is_finite()
                    || n.radius < 0.
                    || n.radius > 1e6
                {
                    return Err("Invalid object geometry or ID".into());
                }
                let a = &n.appearance;
                if !a.stroke_width.is_finite()
                    || !(0.0..=100.).contains(&a.stroke_width)
                    || !a.shadow_blur.is_finite()
                    || !(0.0..=100.).contains(&a.shadow_blur)
                    || !a.shadow_y.is_finite()
                    || a.shadow_y.abs() > 100.
                {
                    return Err("Invalid object appearance".into());
                }
                match &n.kind {
                    NodeKind::Text(t) if !t.validate() => return Err("Invalid text styles".into()),
                    NodeKind::Image { asset } if !doc.assets.contains_key(asset) => {
                        return Err(format!("Missing image {asset}"));
                    }
                    _ => {}
                }
                check(&n.children, depth + 1, ids, count, doc)?;
            }
            Ok(())
        }
        for p in &self.pages {
            if !ids.insert(p.id) || !Rect::new(0., 0., p.width, p.height).valid() {
                return Err("Invalid page".into());
            }
            check(&p.nodes, 0, &mut ids, &mut count, self)?
        }
        if ids.iter().any(|id| *id >= self.next_id) || self.next_id == u64::MAX {
            return Err("Invalid next object ID".into());
        }
        Ok(())
    }
}
