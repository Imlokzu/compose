pub mod palette;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}
impl Default for Color {
    fn default() -> Self {
        Self::rgb(35, 38, 36)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub family: String,
    pub size: f32,
    pub weight: f32,
    pub italic: bool,
    pub underline: bool,
    pub color: Color,
    pub line_height: f32,
    pub letter_spacing: f32,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            family: "Lato".into(),
            size: 16.,
            weight: 400.,
            italic: false,
            underline: false,
            color: Color::rgb(35, 38, 36),
            line_height: 1.4,
            letter_spacing: 0.,
        }
    }
}
impl TextStyle {
    pub fn valid(&self) -> bool {
        !self.family.is_empty()
            && self.family.len() < 256
            && self.size.is_finite()
            && (1.0..=1000.).contains(&self.size)
            && self.weight.is_finite()
            && (1.0..=1000.).contains(&self.weight)
            && self.line_height.is_finite()
            && (0.5..=10.).contains(&self.line_height)
            && self.letter_spacing.is_finite()
            && self.letter_spacing.abs() <= 100.
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub range: Range<usize>,
    pub style: TextStyle,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RichText {
    pub content: String,
    pub style: TextStyle,
    pub spans: Vec<Span>,
    pub align: TextAlign,
}
impl RichText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::default(),
            spans: vec![],
            align: TextAlign::Left,
        }
    }
    pub fn style_at(&self, index: usize) -> TextStyle {
        self.spans
            .iter()
            .find(|s| s.range.contains(&index))
            .map(|s| s.style.clone())
            .unwrap_or_else(|| self.style.clone())
    }
    pub fn range(&self, r: Range<usize>) -> Range<usize> {
        let a = floor_boundary(&self.content, r.start);
        let b = floor_boundary(&self.content, r.end);
        a.min(b)..a.max(b)
    }
    fn segments(&self, r: Range<usize>, offset: usize) -> Vec<Span> {
        if r.is_empty() {
            return vec![];
        }
        let mut bounds = vec![r.start, r.end];
        for s in &self.spans {
            for i in [s.range.start, s.range.end] {
                if i > r.start && i < r.end {
                    bounds.push(i)
                }
            }
        }
        bounds.sort_unstable();
        bounds.dedup();
        bounds
            .windows(2)
            .map(|w| Span {
                range: w[0] - r.start + offset..w[1] - r.start + offset,
                style: self.style_at(w[0]),
            })
            .collect()
    }
    fn normalize(&mut self) {
        let mut out: Vec<Span> = vec![];
        for s in self.spans.drain(..) {
            if s.range.is_empty() {
                continue;
            }
            if let Some(last) = out.last_mut()
                && last.range.end == s.range.start
                && last.style == s.style
            {
                last.range.end = s.range.end;
                continue;
            }
            out.push(s)
        }
        self.spans = out;
    }
    pub fn replace(&mut self, range: Range<usize>, replacement: &str) {
        let r = self.range(range);
        let inherited = self.style_at(
            if (r.is_empty() || r.start == self.content.len()) && r.start > 0 {
                previous_grapheme(&self.content, r.start)
            } else {
                r.start
            },
        );
        let mut spans = self.segments(0..r.start, 0);
        if !replacement.is_empty() {
            spans.push(Span {
                range: r.start..r.start + replacement.len(),
                style: inherited,
            });
        }
        spans.extend(self.segments(r.end..self.content.len(), r.start + replacement.len()));
        self.content.replace_range(r, replacement);
        self.spans = spans;
        self.normalize();
    }
    /// Copy a passage with byte ranges rebased to its new content.
    pub fn slice(&self, range: Range<usize>) -> Self {
        let r = self.range(range);
        Self {
            content: self.content[r.clone()].to_string(),
            style: self.style_at(r.start),
            spans: self.segments(r, 0),
            align: self.align,
        }
    }
    /// Paste formatted text while preserving the styles on both sides of the replacement.
    pub fn replace_rich(&mut self, range: Range<usize>, replacement: &Self) {
        let r = self.range(range);
        let mut spans = self.segments(0..r.start, 0);
        spans.extend(replacement.segments(0..replacement.content.len(), r.start));
        spans.extend(self.segments(
            r.end..self.content.len(),
            r.start + replacement.content.len(),
        ));
        self.content.replace_range(r, &replacement.content);
        self.spans = spans;
        self.normalize();
    }
    pub fn apply_style(&mut self, range: Range<usize>, style: TextStyle) {
        let r = self.range(range);
        if r.is_empty() {
            if self.content.is_empty() {
                self.style = style;
            }
            return;
        }
        let mut spans = self.segments(0..r.start, 0);
        spans.push(Span {
            range: r.clone(),
            style,
        });
        spans.extend(self.segments(r.end..self.content.len(), r.end));
        self.spans = spans;
        self.normalize();
    }
    pub fn validate(&self) -> bool {
        let mut end = 0;
        self.style.valid()
            && self.spans.iter().all(|s| {
                let ok = s.range.start >= end
                    && s.range.start < s.range.end
                    && s.range.end <= self.content.len()
                    && self.content.is_char_boundary(s.range.start)
                    && self.content.is_char_boundary(s.range.end)
                    && s.style.valid();
                end = s.range.end;
                ok
            })
    }
}
pub fn floor_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1
    }
    i
}
pub fn previous_grapheme(s: &str, i: usize) -> usize {
    s.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|&j| j < i)
        .last()
        .unwrap_or(0)
}
pub fn next_grapheme(s: &str, i: usize) -> usize {
    s.grapheme_indices(true)
        .map(|(i, _)| i)
        .find(|&j| j > i)
        .unwrap_or(s.len())
}
pub fn utf16_to_byte(s: &str, i: usize) -> usize {
    let mut units = 0;
    for (b, c) in s.char_indices() {
        if units + c.len_utf16() > i {
            return b;
        }
        units += c.len_utf16();
    }
    s.len()
}
pub fn byte_to_utf16(s: &str, i: usize) -> usize {
    s[..floor_boundary(s, i)].encode_utf16().count()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatted_unicode_passage_pastes_without_changing_neighbors() {
        let mut source = RichText::new("aBIG 👩‍💻z");
        let mut bold = source.style.clone();
        bold.size = 32.;
        bold.weight = 700.;
        source.apply_style(1..4, bold.clone());
        let clip = source.slice(1..16);
        assert_eq!(clip.content, "BIG 👩‍💻");
        assert!(clip.validate());
        let mut target = RichText::new("left right");
        target.replace_rich(4..5, &clip);
        assert_eq!(target.content, "leftBIG 👩‍💻right");
        assert_eq!(target.style_at(4), bold);
        assert_eq!(target.style_at(0).size, 16.);
        assert_eq!(target.style_at(8).weight, 400.);
        assert_eq!(target.style_at(19).size, 16.);
        assert!(target.validate());
    }
    #[test]
    fn unicode_edit_retains_styles() {
        let mut t = RichText::new("hello 👩‍💻 world");
        let mut bold = t.style.clone();
        bold.size = 32.;
        bold.weight = 700.;
        t.apply_style(6..17, bold.clone());
        t.replace(0..5, "Hi");
        assert_eq!(t.content, "Hi 👩‍💻 world");
        assert_eq!(t.style_at(3), bold);
        assert!(t.validate());
        t.replace(3..14, "");
        assert_eq!(t.content, "Hi  world");
        assert!(t.validate());
    }
    #[test]
    fn overlap_and_insertion() {
        let mut t = RichText::new("abcdef");
        let mut b = t.style.clone();
        b.weight = 700.;
        t.apply_style(1..5, b);
        let mut s = t.style.clone();
        s.size = 40.;
        t.apply_style(3..6, s);
        t.replace(2..4, "X");
        assert_eq!(t.content, "abXef");
        assert_eq!(t.style_at(2).weight, 700.);
        assert_eq!(t.style_at(3).size, 40.);
        assert!(t.validate());
    }
    #[test]
    fn insertion_inherits_preceding_style() {
        let mut t = RichText::new("bold plain");
        let mut bold = t.style.clone();
        bold.weight = 700.;
        t.apply_style(0..4, bold);
        t.replace(4..4, "!");
        assert_eq!(t.content, "bold! plain");
        assert_eq!(t.style_at(4).weight, 700.);
        assert_eq!(t.style_at(5).weight, 400.);
    }
    #[test]
    fn graphemes_and_utf16() {
        let s = "a👩‍💻e\u{301}";
        assert_eq!(next_grapheme(s, 1), 12);
        assert_eq!(previous_grapheme(s, 12), 1);
        for (b, _) in s.char_indices() {
            assert_eq!(utf16_to_byte(s, byte_to_utf16(s, b)), b)
        }
    }
}
