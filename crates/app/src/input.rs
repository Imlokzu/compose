use crate::workspace::{Field, Pending, Tool, Workspace};
use gpui::*;
use std::{
    ops::Range,
    time::{Duration, Instant},
};
use text::{byte_to_utf16, next_grapheme, previous_grapheme, utf16_to_byte};
impl Workspace {
    fn popup_blocks_text_input(&self) -> bool {
        self.popup.is_some() && self.field.is_none()
    }
    pub(crate) fn active_content(&self) -> &str {
        if let Some(f) = &self.field {
            &f.value
        } else {
            self.text().map(|t| t.content.as_str()).unwrap_or("")
        }
    }
    pub(crate) fn active_range(&self) -> Range<usize> {
        if let Some(f) = &self.field {
            f.anchor.min(f.caret)..f.anchor.max(f.caret)
        } else {
            self.range()
        }
    }
    fn utf16_range_to_bytes(&self, r: Range<usize>) -> Range<usize> {
        let s = self.active_content();
        utf16_to_byte(s, r.start)..utf16_to_byte(s, r.end)
    }
    fn to_utf16(&self, r: Range<usize>) -> Range<usize> {
        let s = self.active_content();
        byte_to_utf16(s, r.start)..byte_to_utf16(s, r.end)
    }
    pub fn replace_input(&mut self, range: Range<usize>, s: &str) {
        if self.creation.is_some() && self.field.is_none() {
            return;
        }
        if self.word_mode()
            && self.field.is_none()
            && s.chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            self.set_status("Word mode cannot store these control characters", true);
            return;
        }
        if let Some(f) = &mut self.field {
            let start = text::floor_boundary(&f.value, range.start);
            let end = text::floor_boundary(&f.value, range.end).max(start);
            let s = s.replace(['\n', '\r'], "");
            f.value.replace_range(start..end, &s);
            f.caret = start + s.len();
            f.anchor = f.caret;
            if f.field == Field::ImageSearch {
                self.image_picker.query = f.value.clone();
            }
            if f.field == Field::FontSearch {
                self.font_search = f.value.clone();
            }
            return;
        }
        if self.popup_blocks_text_input() {
            return;
        }
        if self.selection_locked() || self.preview || !self.editing || self.text().is_none() {
            return;
        }
        if self
            .typing
            .is_none_or(|t| t.elapsed() > Duration::from_millis(800))
        {
            self.editor.checkpoint();
        }
        self.typing = Some(Instant::now());
        let style = self.insertion_style.clone();
        let range = self.text().unwrap().range(range);
        let caret = range.start + s.len();
        let start = range.start;
        let t = self.text_mut().unwrap();
        t.replace(range, s);
        if let Some(style) = style {
            t.apply_style(start..caret, style);
        }
        self.caret = caret;
        self.anchor = caret;
        self.caret_visible = true;
        self.measure();
        if let Some(l) = self.layout() {
            let h = l.height.max(24.);
            let node = self.editor.selected_node_mut().unwrap();
            node.frame.height = node.frame.height.max(h);
        }
    }
    fn move_caret(&mut self, index: usize, extend: bool) {
        if let Some(f) = &mut self.field {
            f.caret = index.min(f.value.len());
            if !extend {
                f.anchor = f.caret
            }
        } else {
            self.caret = index.min(self.active_content().len());
            if !extend {
                self.anchor = self.caret
            }
        }
        self.typing = None;
        self.insertion_style = None;
        self.marked = None;
        self.caret_visible = true;
    }
    pub fn key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = e.keystroke.key.as_str();
        let m = e.keystroke.modifiers;
        let command = m.platform || m.control;
        if self.pending.is_some() {
            if key == "escape" {
                self.pending = None;
                cx.notify()
            }
            cx.stop_propagation();
            return;
        }
        if self.creation.is_some() && self.creation_key(key, command, m.shift, window, cx) {
            cx.stop_propagation();
            return;
        }
        if command && key == "k" && self.field.is_none() {
            self.open_link_editor();
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if key == "enter" && self.popup == Some(crate::workspace::Popup::Link) {
            self.apply_text_link(false);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if key == "escape" && self.popup.is_some() {
            self.field = None;
            self.close_popup();
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if self.popup == Some(crate::workspace::Popup::Context)
            && command
            && matches!(key, "c" | "x" | "v" | "a" | "d" | "[" | "]" | "g")
        {
            use crate::context_menu::ContextAction;
            let action = match key {
                "c" => ContextAction::Copy,
                "x" => ContextAction::Cut,
                "v" => ContextAction::Paste,
                "a" => ContextAction::SelectAll,
                "d" => ContextAction::Duplicate,
                "[" => ContextAction::Down,
                "]" => ContextAction::Up,
                "g" if m.shift => ContextAction::Ungroup,
                _ => ContextAction::Group,
            };
            self.context_action(action, cx);
            cx.stop_propagation();
            return;
        }
        if command && (key == "p" || (m.shift && key == "e")) {
            if key == "p" {
                self.print_document(cx);
            } else {
                self.toggle_popup(crate::workspace::Popup::Export);
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if self.preview {
            if key == "escape" {
                self.toggle_preview();
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if self.selection_locked()
            && self.popup != Some(crate::workspace::Popup::Images)
            && !(command
                && matches!(
                    key,
                    "s" | "o" | "n" | "q" | "z" | "y" | "c" | "v" | "a" | "=" | "+" | "-" | "0"
                ))
            && key != "escape"
        {
            cx.stop_propagation();
            return;
        }
        if key == "enter"
            && self.popup == Some(crate::workspace::Popup::Images)
            && self
                .field
                .as_ref()
                .is_some_and(|f| f.field == Field::ImageSearch)
        {
            self.search_images(false, cx);
            cx.stop_propagation();
            return;
        }
        let input = self.field.is_some() || self.editing;
        if self.popup == Some(crate::workspace::Popup::Fonts) && key == "enter" {
            let search = self.font_search.to_lowercase();
            if let Some(font) = renderer::FONT_CHOICES.iter().find(|f| {
                format!("{} {}", f.label, f.category)
                    .to_lowercase()
                    .contains(&search)
            }) {
                self.set_font(font.family);
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if self.popup_blocks_text_input()
            && !(command
                && matches!(
                    key,
                    "s" | "o" | "n" | "q" | "z" | "y" | "=" | "+" | "-" | "0"
                ))
        {
            cx.stop_propagation();
            return;
        }
        if self.popup == Some(crate::workspace::Popup::Images) && self.field.is_none() {
            cx.stop_propagation();
            return;
        }
        if command && matches!(key, "b" | "i") && self.text().is_some() {
            let style = self.active_style();
            if let Some(f) = renderer::FONT_CHOICES
                .iter()
                .find(|f| f.family == style.family)
                && ((key == "b" && !f.weights.iter().any(|w| w.0 >= 700.))
                    || (key == "i" && !f.italic))
            {
                cx.stop_propagation();
                return;
            }
        }
        if command {
            match key {
                "s" => self.save(m.shift, cx),
                "o" => self.choose_file(false, cx),
                "n" => self.request(Pending::New, cx),
                "q" => self.request(Pending::Close, cx),
                "z" => self.undo(m.shift),
                "y" => self.undo(true),
                "b" if self.text().is_some() && self.field.is_none() => {
                    let bold = !self.format_values().iter().all(|s| s.weight >= 600.);
                    self.format(|s| s.weight = if bold { 700. } else { 400. });
                }
                "i" if self.text().is_some() && self.field.is_none() => {
                    let italic = !self.format_values().iter().all(|s| s.italic);
                    self.format(|s| s.italic = italic);
                }
                "u" if self.text().is_some() && self.field.is_none() => {
                    let under = !self.format_values().iter().all(|s| s.underline);
                    self.format(|s| s.underline = under);
                }
                "a" if input => {
                    let len = self.active_content().len();
                    if let Some(f) = &mut self.field {
                        f.anchor = 0;
                        f.caret = len
                    } else {
                        self.anchor = 0;
                        self.caret = len
                    }
                }
                "a" => {
                    self.end_edit();
                    self.editor.select_all();
                }
                "g" if !input => {
                    if m.shift {
                        self.ungroup_selection();
                    } else {
                        self.group_selection();
                    }
                }
                "c" | "x" => {
                    if let Some(item) = self.copy_item(key == "x") {
                        cx.write_to_clipboard(item);
                    }
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        self.paste_item(&item);
                    }
                }
                "d" if !input => {
                    self.editor.duplicate_selected();
                }
                "[" if !input => self.editor.reorder_selected(false),
                "]" if !input => self.editor.reorder_selected(true),
                "=" | "+" => self.zoom_by(1.15),
                "-" => self.zoom_by(1. / 1.15),
                "0" => self.fit(),
                "left" | "right" if input => {
                    let end = key == "right";
                    let index = if self.field.is_some() {
                        if end { self.active_content().len() } else { 0 }
                    } else {
                        self.layout()
                            .map(|l| l.line_boundary(self.caret, end))
                            .unwrap_or(self.caret)
                    };
                    self.move_caret(index, m.shift);
                }
                _ => return,
            }
        } else if input {
            match key {
                "escape" => {
                    self.gesture = crate::workspace::Gesture::None;
                    self.guides.clear();
                    if self.field.is_some() {
                        self.field = None;
                        self.set_status("Ready", false);
                    } else {
                        self.end_edit()
                    }
                }
                "enter" => {
                    if self.field.is_some() {
                        let edit = self.field.clone();
                        let previous_status = self.status.clone();
                        self.error = false;
                        self.commit_field();
                        if self.error {
                            self.field = edit;
                            self.caret_visible = true;
                        } else if self.status == previous_status {
                            self.set_status("Ready", false);
                        }
                    } else {
                        self.replace_input(self.range(), "\n")
                    }
                }
                "tab" => {
                    if self.field.is_some() {
                        let edit = self.field.clone();
                        self.error = false;
                        self.commit_field();
                        if self.error {
                            self.field = edit;
                            self.caret_visible = true;
                        }
                    } else {
                        self.replace_input(self.range(), "\t")
                    }
                }
                "backspace" | "delete" => {
                    let mut r = self.active_range();
                    if r.is_empty() {
                        if key == "backspace" {
                            r.start = previous_grapheme(self.active_content(), r.start)
                        } else {
                            r.end = next_grapheme(self.active_content(), r.end)
                        }
                    }
                    self.replace_input(r, "");
                }
                "left" | "right" => {
                    let right = key == "right";
                    let range = self.active_range();
                    let cursor = self.field.as_ref().map(|f| f.caret).unwrap_or(self.caret);
                    let index = if !m.shift && !range.is_empty() {
                        if right { range.end } else { range.start }
                    } else if self.field.is_none() {
                        self.layout()
                            .map(|l| l.move_horizontal(cursor, right, m.alt))
                            .unwrap_or(cursor)
                    } else if right {
                        next_grapheme(self.active_content(), cursor)
                    } else {
                        previous_grapheme(self.active_content(), cursor)
                    };
                    self.move_caret(index, m.shift);
                }
                "up" | "down" => {
                    if self.field.is_none()
                        && let Some(l) = self.layout()
                    {
                        let index = l.move_vertical(self.caret, key == "down");
                        self.move_caret(index, m.shift);
                    }
                }
                "home" | "end" => {
                    let end = key == "end";
                    let index = if let Some(l) = self.layout().filter(|_| self.field.is_none()) {
                        l.line_boundary(self.caret, end)
                    } else if end {
                        self.active_content().len()
                    } else {
                        0
                    };
                    self.move_caret(index, m.shift);
                }
                _ => return,
            }
        } else {
            match key {
                "escape" => {
                    self.tool = Tool::Select;
                    self.gesture = crate::workspace::Gesture::None;
                    self.guides.clear();
                    let parent = self
                        .editor
                        .selected()
                        .and_then(|id| self.editor.parent_of(id))
                        .filter(|id| self.editor.document.node(*id).is_some_and(|n| n.is_group))
                        .filter(|id| {
                            self.editor
                                .selection_ids()
                                .iter()
                                .all(|selected| self.editor.parent_of(*selected) == Some(*id))
                        });
                    self.editor.select_only(parent);
                    self.end_edit();
                }
                "v" => self.tool = Tool::Select,
                "t" => self.tool = Tool::Text,
                "r" if !self.word_mode() => {
                    self.shape_kind = document::ShapeKind::Rectangle;
                    self.tool = Tool::Shape;
                }
                "o" if !self.word_mode() => {
                    self.shape_kind = document::ShapeKind::Ellipse;
                    self.tool = Tool::Shape;
                }
                "f" if !self.word_mode() => self.tool = Tool::Frame,
                "enter" => {
                    if self.text().is_some() {
                        self.editing = true;
                        self.caret = self.text().unwrap().content.len();
                        self.anchor = self.caret;
                        self.typing = None;
                    }
                }
                "backspace" | "delete" => self.editor.delete_selected(),
                "left" | "right" | "up" | "down" => {
                    if self.word_mode() {
                        return;
                    }
                    if self.editor.selected().is_some() {
                        if self
                            .typing
                            .is_none_or(|t| t.elapsed() > Duration::from_millis(500))
                        {
                            self.editor.checkpoint()
                        }
                        self.typing = Some(Instant::now());
                        let amount = if m.shift { 10. } else { 1. };
                        let delta = match key {
                            "left" => document::Point { x: -amount, y: 0. },
                            "right" => document::Point { x: amount, y: 0. },
                            "up" => document::Point { x: 0., y: -amount },
                            _ => document::Point { x: 0., y: amount },
                        };
                        let frames: Vec<_> = self
                            .editor
                            .selection_frames()
                            .into_iter()
                            .map(|(id, r)| {
                                (
                                    id,
                                    document::Rect::new(
                                        r.x + delta.x,
                                        r.y + delta.y,
                                        r.width,
                                        r.height,
                                    ),
                                )
                            })
                            .collect();
                        self.editor.apply_frames(&frames);
                    }
                }
                "space" => self.space_held = true,
                _ => return,
            }
        }
        self.focus.focus(window);
        cx.stop_propagation();
        cx.notify();
    }
    pub fn key_up(&mut self, e: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if e.keystroke.key == "space" {
            self.space_held = false
        }
    }
    pub fn paint_field(
        &mut self,
        field: Field,
        source: &'static str,
        display: &str,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let active = self
            .field
            .as_ref()
            .filter(|f| f.field == field && f.source == source);
        if field == Field::Size && source == "floating" {
            self.selection_control_bounds
                .insert("float-size-field", bounds);
        }
        let content = active.map(|f| f.value.as_str()).unwrap_or(display);
        let run = TextRun {
            len: content.len(),
            font: font(".SystemUIFont"),
            color: rgb(0x303530).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line =
            window
                .text_system()
                .shape_line(content.to_string().into(), px(12.), &[run], None);
        if let Some(f) = active {
            self.field_bounds = bounds;
            let r = f.anchor.min(f.caret)..f.anchor.max(f.caret);
            if !r.is_empty() {
                window.paint_quad(fill(
                    Bounds::new(
                        point(bounds.left() + line.x_for_index(r.start), bounds.top()),
                        size(
                            line.x_for_index(r.end) - line.x_for_index(r.start),
                            bounds.size.height,
                        ),
                    ),
                    rgba(0x2878ed35),
                ));
            } else if self.caret_visible {
                window.paint_quad(fill(
                    Bounds::new(
                        point(
                            bounds.left() + line.x_for_index(f.caret),
                            bounds.top() + px(2.),
                        ),
                        size(px(1.), bounds.size.height - px(4.)),
                    ),
                    rgb(0x2878ed),
                ));
            }
        }
        let _ = line.paint(bounds.origin, px(20.), window, cx);
    }
}
impl EntityInputHandler for Workspace {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        if (!self.editing && self.field.is_none()) || self.popup_blocks_text_input() {
            return None;
        }
        let r = self.utf16_range_to_bytes(range);
        *actual = Some(self.to_utf16(r.clone()));
        Some(self.active_content()[r].to_string())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if (!self.editing && self.field.is_none()) || self.popup_blocks_text_input() {
            return None;
        }
        Some(UTF16Selection {
            range: self.to_utf16(self.active_range()),
            reversed: self
                .field
                .as_ref()
                .map(|f| f.caret < f.anchor)
                .unwrap_or(self.caret < self.anchor),
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.clone().map(|r| self.to_utf16(r))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        s: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending.is_some() || self.popup_blocks_text_input() {
            return;
        }
        let r = range
            .map(|r| self.utf16_range_to_bytes(r))
            .or(self.marked.clone())
            .unwrap_or_else(|| self.active_range());
        self.replace_input(r, s);
        self.marked = None;
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        s: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending.is_some() || self.popup_blocks_text_input() {
            return;
        }
        let r = range
            .map(|r| self.utf16_range_to_bytes(r))
            .or(self.marked.clone())
            .unwrap_or_else(|| self.active_range());
        let start = r.start;
        self.replace_input(r, s);
        self.marked = if s.is_empty() {
            None
        } else {
            Some(start..start + s.len())
        };
        if let Some(sel) = selected {
            let a = start + utf16_to_byte(s, sel.start);
            let b = start + utf16_to_byte(s, sel.end);
            if let Some(f) = &mut self.field {
                f.anchor = a;
                f.caret = b
            } else {
                self.anchor = a;
                self.caret = b
            }
        }
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.popup_blocks_text_input() {
            return None;
        }
        if self.field.is_some() {
            return Some(self.field_bounds);
        }
        let r = self.utf16_range_to_bytes(range);
        let c = self.layout()?.caret(r.start);
        let n = self.absolute_frame(self.editor.selected()?)?;
        Some(self.screen_rect(document::Rect::new(n.x + c.x, n.y + c.y, c.width, c.height)))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        if self.field.is_some() {
            return Some(self.to_utf16(self.active_range()).end);
        }
        let p = self.to_doc(point);
        let r = self.absolute_frame(self.editor.selected()?)?;
        let i = self.layout()?.hit_test(p.x - r.x, p.y - r.y);
        Some(byte_to_utf16(self.active_content(), i))
    }
}

impl Workspace {
    pub fn menu_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.key_down(
            &KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
            },
            window,
            cx,
        );
    }
}
