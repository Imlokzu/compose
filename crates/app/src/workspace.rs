use document::{Document, NodeId, NodeKind, Point as DocPoint, Rect};
use editor::Editor;
use gpui::*;
use renderer::{TextEngine, TextLayout};
use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use text::{RichText, TextStyle};

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Text,
    Shape,
    Frame,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    X,
    Y,
    W,
    H,
    Size,
    LineHeight,
    Spacing,
    Opacity,
    Radius,
    Name,
    Color,
    DocumentName,
    StrokeWidth,
    ShadowBlur,
    ShadowY,
    ColorHex,
    ColorAlpha,
    FontSearch,
    ImageSearch,
    NewDocumentName,
    LinkUrl,
}
#[derive(Clone)]
pub struct FieldEdit {
    pub source: &'static str,
    pub field: Field,
    pub value: String,
    pub original: String,
    pub anchor: usize,
    pub caret: usize,
}
#[derive(Clone)]
pub enum Pending {
    New,
    NewWord,
    Create(Box<Document>),
    Open(PathBuf),
    Close,
}
pub enum Gesture {
    None,
    Create {
        start: DocPoint,
        current: DocPoint,
        tool: Tool,
    },
    Move {
        start: DocPoint,
        frame: Rect,
        frames: Vec<(NodeId, Rect)>,
        collapse: Option<NodeId>,
        started: bool,
    },
    Marquee {
        start: DocPoint,
        current: DocPoint,
        base: Vec<NodeId>,
    },
    Resize {
        start: DocPoint,
        frame: Rect,
        handle: usize,
        started: bool,
    },
    Text,
    Radius {
        start: DocPoint,
        radius: f32,
        started: bool,
    },
    Pan {
        start: gpui::Point<Pixels>,
        pan: DocPoint,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Link,
    Background,
    Context,
    Modes,
    Export,
    Images,
    Fonts,
    Colors,
    Pages,
    TextStyles,
    TextWeights,
    TextSizes,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Fill,
    Stroke,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorDrag {
    Saturation,
    Hue,
}
pub struct ColorPicker {
    pub target: ColorTarget,
    pub hsv: text::palette::Hsv,
    pub drag: Option<ColorDrag>,
    pub changed: bool,
}
pub struct Workspace {
    pub link_edit: Option<crate::links::LinkEdit>,
    pub link_url: String,
    pub drag_epoch: u64,
    pub background_service: Option<String>,
    pub cutout_target: Option<NodeId>,
    pub cutout_generation: u64,
    pub creation: Option<crate::creation::Creation>,
    pub preset_previews: HashMap<document::presets::Preset, Arc<RenderImage>>,
    pub preset_scroll: ScrollHandle,
    pub context_position: gpui::Point<Pixels>,
    pub object_clipboard: Option<crate::clipboard::LocalClipboard>,
    pub image_picker: crate::image_picker::ImagePicker,
    pub popup: Option<Popup>,
    pub text_popup_anchor: Option<Bounds<Pixels>>,
    pub text_popup_bounds: Bounds<Pixels>,
    pub picker: Option<ColorPicker>,
    pub color_bounds: Bounds<Pixels>,
    pub hue_bounds: Bounds<Pixels>,
    pub font_search: String,
    pub assets_tab: bool,
    pub preview: bool,
    pub export_dpi: u32,
    pub output_resolution_bounds: HashMap<u32, Bounds<Pixels>>,
    pub selection_control_bounds: HashMap<&'static str, Bounds<Pixels>>,
    pub layer_bounds: HashMap<NodeId, Bounds<Pixels>>,
    pub editor: Editor,
    pub engine: TextEngine,
    pub layouts: HashMap<NodeId, (RichText, f32, TextLayout)>,
    pub images: HashMap<String, Arc<RenderImage>>,
    pub focus: FocusHandle,
    pub tool: Tool,
    pub editing: bool,
    pub anchor: usize,
    pub caret: usize,
    pub marked: Option<Range<usize>>,
    pub typing: Option<Instant>,
    pub insertion_style: Option<TextStyle>,
    pub zoom: f32,
    pub pan: DocPoint,
    pub pinch: Option<crate::zoom::Pinch>,
    pub pinch_blocked: bool,
    pub viewport: Bounds<Pixels>,
    pub page_origin: gpui::Point<Pixels>,
    pub fitted: bool,
    pub gesture: Gesture,
    pub guides: Vec<layout::Guide>,
    pub space_held: bool,
    pub last_pointer: Option<gpui::Point<Pixels>>,
    pub field: Option<FieldEdit>,
    pub field_bounds: Bounds<Pixels>,
    pub path: Option<PathBuf>,
    pub saved: Document,
    pub status: String,
    pub error: bool,
    pub busy: bool,
    pub pending: Option<Pending>,
    pub show_pages: bool,
    pub collapsed: bool,
    pub collapsed_nodes: std::collections::HashSet<NodeId>,
    pub caret_visible: bool,
}
impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let engine = TextEngine::new();
        let font_error = engine.register_gpui_fonts(cx).err();
        let mut editor = Editor::default();
        let mut path = None;
        let mut status = "Double-click the page to start writing".to_string();
        let mut error = false;
        if let Some(p) = std::env::args().skip(1).find(|a| a.ends_with(".compose")) {
            match file_format::load(Path::new(&p)) {
                Ok(doc) => {
                    editor = Editor::new(doc);
                    path = Some(PathBuf::from(p));
                }
                Err(e) => {
                    status = format!("Could not open: {e}");
                    error = true
                }
            }
        }
        if let Some(e) = font_error {
            status = format!("Font loading failed: {e}");
            error = true
        }
        let focus = cx.focus_handle();
        focus.focus(window);
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(530))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        if this.editing || this.field.is_some() {
                            this.caret_visible = !this.caret_visible;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let show_creation = path.is_none()
            && !std::env::args()
                .any(|a| a.starts_with("--ui-smoke") || a.starts_with("--image-picker"));
        let mut workspace = Self {
            link_edit: None,
            link_url: String::new(),
            drag_epoch: 0,
            background_service: None,
            cutout_target: None,
            cutout_generation: 0,
            creation: show_creation.then(crate::creation::Creation::default),
            preset_previews: HashMap::new(),
            preset_scroll: ScrollHandle::new(),
            context_position: point(px(0.), px(0.)),
            object_clipboard: None,
            image_picker: Default::default(),
            popup: None,
            text_popup_anchor: None,
            text_popup_bounds: Bounds::default(),
            picker: None,
            color_bounds: Bounds::default(),
            hue_bounds: Bounds::default(),
            font_search: String::new(),
            assets_tab: false,
            preview: false,
            export_dpi: 144,
            output_resolution_bounds: HashMap::new(),
            selection_control_bounds: HashMap::new(),
            layer_bounds: HashMap::new(),
            saved: editor.document.clone(),
            editor,
            engine,
            layouts: HashMap::new(),
            images: HashMap::new(),
            focus,
            tool: Tool::Select,
            editing: false,
            anchor: 0,
            caret: 0,
            marked: None,
            typing: None,
            insertion_style: None,
            zoom: 1.,
            pan: DocPoint::default(),
            pinch: None,
            pinch_blocked: false,
            viewport: Bounds::default(),
            page_origin: point(px(0.), px(0.)),
            fitted: false,
            gesture: Gesture::None,
            guides: vec![],
            space_held: false,
            last_pointer: None,
            field: None,
            field_bounds: Bounds::default(),
            path,
            status,
            error,
            busy: false,
            pending: None,
            show_pages: false,
            collapsed: false,
            collapsed_nodes: Default::default(),
            caret_visible: true,
        };
        workspace.load_preset_previews(cx);
        workspace.load_background_service();
        workspace
    }
    pub fn dirty(&self) -> bool {
        self.editor.document != self.saved
    }
    pub fn text(&self) -> Option<&RichText> {
        match &self.editor.selected_node()?.kind {
            NodeKind::Text(t) => Some(t),
            _ => None,
        }
    }
    pub fn text_mut(&mut self) -> Option<&mut RichText> {
        match &mut self.editor.selected_node_mut()?.kind {
            NodeKind::Text(t) => Some(t),
            _ => None,
        }
    }
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }
    pub fn layout(&self) -> Option<&TextLayout> {
        self.layouts
            .get(&self.editor.selected()?)
            .map(|(_, _, l)| l)
    }
    pub fn end_edit(&mut self) {
        self.editing = false;
        self.marked = None;
        self.typing = None;
        self.insertion_style = None;
    }
    pub fn set_status(&mut self, s: impl Into<String>, error: bool) {
        self.status = s.into();
        self.error = error;
    }
    pub fn measure(&mut self) {
        if self.word_mode() {
            self.reflow_word();
            return;
        }
        let nodes = self.editor.document.pages[self.editor.page].nodes.clone();
        fn gather(nodes: &[document::Node], out: &mut Vec<(NodeId, RichText, f32)>) {
            for n in nodes {
                if let NodeKind::Text(t) = &n.kind {
                    out.push((n.id, t.clone(), n.frame.width));
                }
                gather(&n.children, out)
            }
        }
        let mut texts = vec![];
        gather(&nodes, &mut texts);
        for (id, t, w) in texts {
            if self
                .layouts
                .get(&id)
                .is_none_or(|(old, ow, _)| *old != t || *ow != w)
            {
                let l = self.engine.layout(&t, w);
                let height = l.height.max(24.);
                if let Some(node) = self.editor.document.node_mut(id) {
                    node.frame.height = node.frame.height.max(height);
                }
                self.layouts.insert(id, (t, w, l));
            }
        }
        self.layouts
            .retain(|id, _| self.editor.document.node(*id).is_some());
        self.editor.normalize_groups();
    }
    pub fn fit(&mut self) {
        self.pinch = None;
        self.pinch_blocked = false;
        let p = &self.editor.document.pages[self.editor.page];
        let width: f32 = self.viewport.size.width.into();
        let height: f32 = self.viewport.size.height.into();
        self.zoom = ((width - 96.) / p.width)
            .min((height - 96.) / p.height)
            .clamp(0.2, 1.5);
        self.pan = DocPoint::default();
        self.fitted = true;
    }
    pub fn zoom_by(&mut self, factor: f32) {
        if !factor.is_finite() || factor <= 0. {
            return;
        }
        self.pinch = None;
        self.pinch_blocked = false;
        self.zoom = (self.zoom * factor).clamp(layout::MIN_ZOOM, layout::MAX_ZOOM);
        self.fitted = true;
    }
    pub fn insert_at(&mut self, tool: Tool, p: DocPoint) {
        let page = &self.editor.document.pages[self.editor.page];
        let (w, h): (f32, f32) = match tool {
            Tool::Text => (300., 28.),
            Tool::Frame => (280., 180.),
            _ => (160., 120.),
        };
        let x = p.x.clamp(0., (page.width - 16.).max(0.));
        let y = p.y.clamp(0., (page.height - 16.).max(0.));
        self.insert_box(
            tool,
            Rect::new(x, y, w.min(page.width - x), h.min(page.height - y)),
        );
    }
    pub fn insert_box(&mut self, tool: Tool, rect: Rect) {
        if self.word_mode() && tool != Tool::Text {
            self.set_status("Shapes and frames are available in Compose mode", true);
            return;
        }
        self.commit_field();
        self.end_edit();
        let kind = match tool {
            Tool::Text => NodeKind::Text(RichText::new("")),
            Tool::Frame => NodeKind::Frame,
            _ => NodeKind::Shape,
        };
        self.editor.insert(kind, rect);
        if self.word_mode() {
            self.measure();
        }
        self.tool = Tool::Select;
        if tool == Tool::Text {
            self.editing = true;
            self.anchor = 0;
            self.caret = 0;
        }
        self.set_status(
            if tool == Tool::Text {
                "Start writing"
            } else {
                "Drag to move · Shift to keep proportions"
            },
            false,
        );
    }
    pub fn undo(&mut self, redo: bool) {
        self.drag_epoch = self.drag_epoch.wrapping_add(1);
        self.cutout_generation = self.cutout_generation.wrapping_add(1);
        self.cutout_target = None;
        self.image_picker.invalidate();
        if self.preview {
            return;
        }
        self.popup = None;
        self.text_popup_anchor = None;
        self.picker = None;
        self.field = None;
        self.end_edit();
        self.gesture = Gesture::None;
        self.guides.clear();
        if redo {
            self.editor.redo();
        } else {
            self.editor.undo();
        }
        self.set_status(if redo { "Redone" } else { "Undone" }, false);
    }
    pub fn format(&mut self, f: impl Fn(&mut TextStyle)) {
        self.format_with_history(f, true);
    }
    pub fn format_with_history(&mut self, f: impl Fn(&mut TextStyle), history: bool) {
        if self.selection_locked() {
            return;
        }
        self.commit_field();
        self.typing = None;
        let Some(t) = self.text() else { return };
        let range = if self.editing {
            self.range()
        } else {
            0..t.content.len()
        };
        if t.content.is_empty() {
            if history {
                self.editor.checkpoint();
            }
            f(&mut self.text_mut().unwrap().style);
            renderer::normalize_style(&mut self.text_mut().unwrap().style);
            self.insertion_style = None;
            self.measure();
            return;
        }
        if self.editing && range.is_empty() {
            let mut s = self
                .insertion_style
                .clone()
                .unwrap_or_else(|| t.style_at(self.caret.saturating_sub(1)));
            f(&mut s);
            renderer::normalize_style(&mut s);
            self.insertion_style = Some(s);
            return;
        }
        if history {
            self.editor.checkpoint();
        }
        if let Some(t) = self.text_mut() {
            let mut boundaries = vec![range.start, range.end];
            for s in &t.spans {
                for i in [s.range.start, s.range.end] {
                    if i > range.start && i < range.end {
                        boundaries.push(i)
                    }
                }
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            if t.content.is_empty() {
                f(&mut t.style);
            }
            for w in boundaries.windows(2) {
                let mut s = t.style_at(w[0]);
                f(&mut s);
                renderer::normalize_style(&mut s);
                t.apply_style(w[0]..w[1], s);
            }
        }
        self.measure();
    }
    pub fn active_style(&self) -> TextStyle {
        self.insertion_style
            .clone()
            .or_else(|| {
                self.text().map(|t| {
                    t.style_at(if self.editing {
                        if self.range().is_empty() {
                            text::previous_grapheme(&t.content, self.caret)
                        } else {
                            self.range().start
                        }
                    } else {
                        0
                    })
                })
            })
            .unwrap_or_default()
    }
    pub fn begin_field(&mut self, field: Field, value: String) {
        self.commit_field();
        self.typing = None;
        self.marked = None;
        let end = value.len();
        self.field = Some(FieldEdit {
            source: "inspector",
            field,
            original: value.clone(),
            value,
            anchor: 0,
            caret: end,
        });
    }
    pub fn commit_field(&mut self) {
        let Some(edit) = self.field.take() else {
            return;
        };
        if edit.value == edit.original {
            return;
        }
        if edit.field == Field::NewDocumentName {
            if let Some(creation) = &mut self.creation {
                creation.name = edit.value.trim().to_string();
            }
            return;
        }
        if self.word_mode()
            && matches!(
                edit.field,
                Field::X | Field::Y | Field::H | Field::Opacity | Field::Radius | Field::Color
            )
        {
            self.set_status(
                "Word mode manages block layout and uses opaque content",
                true,
            );
            return;
        }
        if self.commit_special_field(&edit) {
            return;
        }
        if self.selection_locked() {
            return;
        }
        let Some(node) = self.editor.selected_node() else {
            return;
        };
        if edit.field == Field::Name {
            if !edit.value.trim().is_empty() {
                self.editor.checkpoint();
                self.editor.selected_node_mut().unwrap().name = edit.value.trim().to_string();
            }
            return;
        }
        if edit.field == Field::Color {
            let hex = edit.value.trim().trim_start_matches('#');
            if hex.len() != 6 {
                self.set_status("Use a six-digit hex color", true);
                return;
            }
            let Ok(value) = u32::from_str_radix(hex, 16) else {
                self.set_status("Use a valid hex color", true);
                return;
            };
            let color = document::Color::rgb((value >> 16) as u8, (value >> 8) as u8, value as u8);
            if self.text().is_some() {
                self.format(|s| s.color = color)
            } else {
                self.editor.checkpoint();
                self.editor.selected_node_mut().unwrap().fill = color;
            }
            return;
        }
        let Ok(v) = edit.value.trim().parse::<f32>() else {
            self.set_status("Enter a valid number", true);
            return;
        };
        if !v.is_finite() {
            self.set_status("Enter a finite number", true);
            return;
        }
        let valid = match edit.field {
            Field::W | Field::H => (1.0..=10000.).contains(&v),
            Field::Size => (1.0..=1000.).contains(&v),
            Field::LineHeight => (0.5..=10.).contains(&v),
            Field::Spacing => v.abs() <= 100.,
            Field::Opacity => (0.0..=100.).contains(&v),
            Field::Radius => (0.0..=1000.).contains(&v),
            _ => v.abs() < 100000.,
        };
        if !valid {
            self.set_status("That value is outside the allowed range", true);
            return;
        }
        if matches!(edit.field, Field::Size | Field::LineHeight | Field::Spacing) {
            self.format(|s| match edit.field {
                Field::Size => s.size = v,
                Field::LineHeight => s.line_height = v,
                _ => s.letter_spacing = v,
            });
            return;
        }
        let _ = node;
        if edit.field == Field::Radius {
            self.set_radius(v);
            return;
        }
        let ratio = if self.image_scales() {
            self.image_ratio()
        } else {
            None
        };
        let paired = match (edit.field, ratio) {
            (Field::W, Some(r)) => Some(v / r),
            (Field::H, Some(r)) => Some(v * r),
            _ => None,
        };
        if paired.is_some_and(|p| !(1.0..=10000.).contains(&p)) {
            self.set_status("The proportional size is outside the allowed range", true);
            return;
        }
        self.editor.checkpoint();
        let node = self.editor.selected_node_mut().unwrap();
        match edit.field {
            Field::X => node.frame.x = v,
            Field::Y => node.frame.y = v,
            Field::W => {
                node.frame.width = v;
                if let Some(h) = paired {
                    node.frame.height = h
                }
            }
            Field::H => {
                node.frame.height = v;
                if let Some(w) = paired {
                    node.frame.width = w
                }
            }
            Field::Opacity => node.opacity = v / 100.,
            Field::Radius => node.radius = v,
            _ => {}
        }
        self.set_status("Updated", false);
    }
    pub fn request(&mut self, p: Pending, cx: &mut Context<Self>) {
        if matches!(p, Pending::New | Pending::NewWord) {
            self.open_creation(if matches!(p, Pending::NewWord) {
                document::DocumentMode::Word
            } else {
                document::DocumentMode::Compose
            });
            cx.notify();
            return;
        }
        if self.popup == Some(Popup::Images) {
            self.close_popup();
        }
        self.commit_field();
        if self.dirty() {
            self.pending = Some(p)
        } else {
            self.execute(p, cx)
        }
        cx.notify();
    }
    pub fn execute(&mut self, p: Pending, cx: &mut Context<Self>) {
        self.pending = None;
        match p {
            Pending::New => self.open_creation(document::DocumentMode::Compose),
            Pending::NewWord => self.open_creation(document::DocumentMode::Word),
            Pending::Create(doc) => {
                self.replace_document(*doc, None);
                self.measure();
                self.set_status(
                    if self.word_mode() {
                        "Word-compatible · editable .docx export · text flows in reading order"
                    } else {
                        "Compose · drag to draw a text box · double-click text to edit"
                    },
                    false,
                );
            }
            Pending::Open(p) => match file_format::load(&p) {
                Ok(d) => {
                    if let Err(e) = self.validate_images(&d) {
                        self.set_status(format!("Could not open image: {e}"), true)
                    } else {
                        self.replace_document(d, Some(p));
                    }
                }
                Err(e) => self.set_status(format!("Could not open: {e}"), true),
            },
            Pending::Close => cx.quit(),
        }
        cx.notify();
    }
    pub fn replace_document(&mut self, doc: Document, path: Option<PathBuf>) {
        self.link_edit = None;
        self.link_url.clear();
        self.drag_epoch = self.drag_epoch.wrapping_add(1);
        self.cutout_generation = self.cutout_generation.wrapping_add(1);
        self.cutout_target = None;
        self.creation = None;
        self.gesture = Gesture::None;
        self.image_picker.invalidate();
        self.popup = None;
        self.text_popup_anchor = None;
        self.picker = None;
        self.preview = false;
        self.end_edit();
        self.field = None;
        self.saved = doc.clone();
        self.editor = Editor::new(doc);
        self.layouts.clear();
        self.images.clear();
        self.layer_bounds.clear();
        self.selection_control_bounds.clear();
        self.collapsed_nodes.clear();
        self.collapsed = false;
        self.assets_tab = false;
        self.path = path;
        self.pan = DocPoint::default();
        self.fitted = false;
        self.pinch = None;
        self.pinch_blocked = false;
        self.set_status("Ready", false);
    }
    pub fn save(&mut self, save_as: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.commit_field();
        self.typing = None;
        self.measure();
        if !save_as && let Some(p) = self.path.clone() {
            self.save_to(p, cx);
            return;
        }
        let default_dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .map(|p| p.join("Documents"))
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let dir = self
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or(&default_dir);
        let name = format!("{}.compose", self.editor.document.name);
        let prompt = cx.prompt_for_new_path(dir, Some(&name));
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let result = prompt.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(Ok(Some(mut p))) => {
                        if p.extension().is_none() {
                            p.set_extension("compose");
                        }
                        this.save_to(p, cx);
                    }
                    Ok(Ok(None)) => {
                        this.pending = None;
                    }
                    _ => this.set_status("Could not open the save dialog", true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn save_to(&mut self, p: PathBuf, cx: &mut Context<Self>) {
        self.busy = true;
        self.set_status("Saving…", false);
        let snapshot = self.editor.document.clone();
        let task = cx.background_executor().spawn({
            let p = p.clone();
            let doc = snapshot.clone();
            async move { file_format::save(&p, &doc) }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => {
                        this.saved = snapshot;
                        this.path = Some(p);
                        this.set_status("Saved", false);
                        if !this.dirty()
                            && let Some(pending) = this.pending.take()
                        {
                            this.execute(pending, cx)
                        }
                    }
                    Err(e) => this.set_status(format!("Save failed: {e}"), true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub fn choose_file(&mut self, image: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.commit_field();
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                if image {
                    "Insert image"
                } else {
                    "Open Compose document"
                }
                .into(),
            ),
        });
        self.busy = true;
        let generation = self.image_picker.generation;
        cx.spawn(async move |this, cx| {
            let result = prompt.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                if image
                    && (this.popup != Some(Popup::Images) || !this.image_picker.accepts(generation))
                {
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(p) = paths.first() {
                            if image {
                                this.load_local_image(p.clone(), cx);
                            } else {
                                this.request(Pending::Open(p.clone()), cx)
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => this.set_status("Could not open the file dialog", true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub fn decode_image(bytes: &[u8]) -> anyhow::Result<(Arc<RenderImage>, u32, u32)> {
        use anyhow::ensure;
        ensure!(bytes.len() <= 64 * 1024 * 1024, "Image exceeds 64 MB");
        let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
        let (w, h) = reader.into_dimensions()?;
        ensure!(
            w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 32_000_000,
            "Image exceeds 32 megapixels"
        );
        let mut data = image::load_from_memory(bytes)?.into_rgba8();
        for p in data.pixels_mut() {
            p.0.swap(0, 2)
        }
        Ok((
            Arc::new(RenderImage::new(smallvec::smallvec![image::Frame::new(
                data
            )])),
            w,
            h,
        ))
    }
    fn validate_images(&self, doc: &Document) -> anyhow::Result<()> {
        for bytes in doc.assets.values() {
            Self::decode_image(bytes)?;
        }
        Ok(())
    }
    pub fn import_image(&mut self, path: &Path) -> anyhow::Result<()> {
        anyhow::ensure!(
            std::fs::metadata(path)?.len() <= 64 * 1024 * 1024,
            "Image exceeds 64 MB"
        );
        let bytes = std::fs::read(path)?;
        let (image, w, h) = Self::decode_image(&bytes)?;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into();
        self.insert_decoded_image(bytes, image, w, h, name, None);
        self.set_status("Image inserted · drag any handle to scale", false);
        Ok(())
    }
}

impl Workspace {
    pub fn align_object(&mut self, align: u8) {
        if self.word_mode() {
            return;
        }
        self.commit_field();
        self.end_edit();
        let Some(id) = self.editor.selected() else {
            return;
        };
        let Some(r) = self.absolute_frame(id) else {
            return;
        };
        let local = self.editor.selected_node().unwrap().frame;
        let page_width = self.editor.document.pages[self.editor.page].width;
        let x = match align {
            0 => 40.,
            1 => (page_width - r.width) / 2.,
            _ => page_width - 40. - r.width,
        };
        self.editor.checkpoint();
        self.editor.selected_node_mut().unwrap().frame.x = local.x + x - r.x;
    }
}
