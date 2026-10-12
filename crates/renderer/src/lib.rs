//! Parley lays out in document points. GPUI paints the resulting glyph positions.
use document::Rect;
#[cfg(feature = "gpui")]
use gpui::{App, Pixels, Point, Window, point, px};
#[cfg(feature = "gpui")]
use parley::PositionedLayoutItem;
use parley::{
    Affinity, Alignment, AlignmentOptions, Cursor, FontContext, Layout, LayoutContext, Selection,
    StyleProperty,
};
#[cfg(feature = "gpui")]
use skrifa::{FontRef, MetadataProvider, string::StringId};
#[cfg(feature = "gpui")]
use std::borrow::Cow;
use std::{ops::Range, sync::Arc};
use text::{Color, RichText, TextAlign, TextStyle};
mod fonts;
use fonts::FONTS;
pub use fonts::{FONT_CHOICES, FontChoice, normalize_style};
pub struct TextEngine {
    fonts: FontContext,
    context: LayoutContext<Color>,
}
impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}
impl TextEngine {
    pub fn new() -> Self {
        let mut fonts = FontContext::new();
        for bytes in FONTS {
            fonts
                .collection
                .register_fonts(parley::fontique::Blob::new(Arc::new(bytes.to_vec())), None);
        }
        #[cfg(target_os = "android")]
        {
            use parley::fontique::{Blob, GenericFamily, Script};
            static SYSTEM_FONTS: std::sync::OnceLock<Vec<(&'static str, Arc<Vec<u8>>)>> =
                std::sync::OnceLock::new();
            let sources = SYSTEM_FONTS.get_or_init(|| {
                let mut sources = vec![(
                    "emoji",
                    Arc::new(include_bytes!("../../../assets/fonts/NotoColorEmoji.ttf").to_vec()),
                )];
                sources.extend(
                    [
                        ("Arab", "/system/fonts/NotoNaskhArabic-Regular.ttf"),
                        ("Hani", "/system/fonts/NotoSansCJK-Regular.ttc"),
                        ("Deva", "/system/fonts/NotoSansDevanagari-VF.ttf"),
                    ]
                    .into_iter()
                    .filter_map(|(script, path)| {
                        std::fs::read(path)
                            .ok()
                            .map(|bytes| (script, Arc::new(bytes)))
                    })
                    .collect::<Vec<_>>(),
                );
                sources
            });
            for (script, bytes) in sources {
                let families = fonts
                    .collection
                    .register_fonts(Blob::new(bytes.clone()), None)
                    .into_iter()
                    .map(|(family, _)| family)
                    .collect::<Vec<_>>();
                if *script == "emoji" {
                    fonts
                        .collection
                        .append_generic_families(GenericFamily::Emoji, families.into_iter());
                } else if let Ok(tag) = <[u8; 4]>::try_from(script.as_bytes()) {
                    fonts
                        .collection
                        .append_fallbacks(Script::from_bytes(tag), families.into_iter());
                }
            }
        }
        Self {
            fonts,
            context: LayoutContext::new(),
        }
    }
    #[cfg(feature = "gpui")]
    pub fn register_gpui_fonts(&self, cx: &mut App) -> anyhow::Result<()> {
        cx.text_system()
            .add_fonts(FONTS.iter().map(|b| Cow::Borrowed(*b)).collect())
    }
    /// Native regression check: the painter must resolve the exact static faces
    /// used by Parley, never a silent fallback with unrelated glyph indices.
    #[cfg(feature = "gpui")]
    pub fn verify_native_faces(&mut self, window: &mut Window) -> anyhow::Result<usize> {
        use anyhow::ensure;
        let probe = "HAMBURGEVONS0123456789";
        let mut faces = std::collections::HashSet::new();
        let mut checked = 0;
        for choice in FONT_CHOICES {
            for (weight, _) in choice.weights {
                for italic in [false, true]
                    .into_iter()
                    .filter(|italic| !italic || choice.italic)
                {
                    let mut text = RichText::new(probe);
                    text.style.family = choice.family.into();
                    text.style.weight = *weight;
                    text.style.italic = italic;
                    text.style.size = 32.;
                    let layout = self.layout(&text, 1200.);
                    let mut parley_glyphs = vec![];
                    let mut descriptor = None;
                    for line in layout.layout.lines() {
                        for item in line.items() {
                            if let PositionedLayoutItem::GlyphRun(gr) = item {
                                let run = gr.run();
                                let face = FontRef::from_index(
                                    run.font().data.as_ref(),
                                    run.font().index,
                                )?;
                                descriptor = Some(paint_font(
                                    &face,
                                    run.font_attrs().weight.value(),
                                    run.font_attrs().style == parley::FontStyle::Italic,
                                ));
                                parley_glyphs.extend(gr.glyphs().map(|g| g.id));
                            }
                        }
                    }
                    let descriptor = descriptor
                        .ok_or_else(|| anyhow::anyhow!("No glyphs for {}", choice.family))?;
                    let font_id = window.text_system().resolve_font(&descriptor);
                    let resolved = window
                        .text_system()
                        .get_font_for_id(font_id)
                        .ok_or_else(|| anyhow::anyhow!("Unregistered native font"))?;
                    ensure!(
                        resolved.family.as_ref() == choice.family
                            && resolved.weight.0 == *weight
                            && (resolved.style == gpui::FontStyle::Italic) == italic,
                        "{} {weight} italic {italic} silently resolved as {resolved:?} (requested {descriptor:?})",
                        choice.family
                    );
                    ensure!(
                        faces.insert(font_id),
                        "Distinct face {} {weight} italic {italic} reused another native face",
                        choice.family
                    );
                    let line = window.text_system().shape_line(
                        probe.into(),
                        px(32.),
                        &[gpui::TextRun {
                            len: probe.len(),
                            font: descriptor,
                            color: gpui::black(),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }],
                        None,
                    );
                    let native_glyphs: Vec<_> = line
                        .runs
                        .iter()
                        .flat_map(|r| r.glyphs.iter().map(|g| g.id.0))
                        .collect();
                    ensure!(
                        parley_glyphs == native_glyphs,
                        "Glyph indices differ for {} {weight} italic {italic}",
                        choice.family
                    );
                    checked += 1;
                }
            }
        }
        Ok(checked)
    }
    pub fn layout(&mut self, text: &RichText, width: f32) -> TextLayout {
        let mut builder = self
            .context
            .ranged_builder(&mut self.fonts, &text.content, 1.0, false);
        fn properties(s: &TextStyle) -> Vec<StyleProperty<'_, Color>> {
            #[cfg(target_os = "android")]
            let family = parley::FontFamily::Source(std::borrow::Cow::Owned(format!(
                "\"{}\", emoji",
                s.family
            )));
            #[cfg(not(target_os = "android"))]
            let family = s.family.as_str().into();
            vec![
                StyleProperty::FontFamily(family),
                StyleProperty::FontSize(s.size),
                StyleProperty::FontWeight(parley::FontWeight::new(s.weight)),
                StyleProperty::FontStyle(if s.italic {
                    parley::FontStyle::Italic
                } else {
                    parley::FontStyle::Normal
                }),
                StyleProperty::LineHeight(parley::LineHeight::FontSizeRelative(s.line_height)),
                StyleProperty::LetterSpacing(s.letter_spacing),
                StyleProperty::Brush(s.color),
                StyleProperty::Underline(s.underline),
            ]
        }
        for p in properties(&text.style) {
            builder.push_default(p)
        }
        for span in &text.spans {
            for p in properties(&span.style) {
                builder.push(p, span.range.clone())
            }
        }
        let mut layout = builder.build(&text.content);
        layout.break_all_lines(Some(width.max(1.)));
        layout.align(
            match text.align {
                TextAlign::Left => Alignment::Start,
                TextAlign::Center => Alignment::Center,
                TextAlign::Right => Alignment::End,
                TextAlign::Justify => Alignment::Justify,
            },
            AlignmentOptions::default(),
        );
        let height = layout
            .height()
            .max(text.style.size * text.style.line_height);
        TextLayout {
            layout: Arc::new(layout),
            height,
            len: text.content.len(),
        }
    }
}
#[derive(Clone)]
pub struct TextLayout {
    layout: Arc<Layout<Color>>,
    pub height: f32,
    len: usize,
}
fn rect(r: parley::BoundingBox) -> Rect {
    Rect::new(
        r.x0 as f32,
        r.y0 as f32,
        r.width() as f32,
        r.height() as f32,
    )
}
#[cfg(feature = "gpui")]
fn paint_font(face: &FontRef<'_>, weight: f32, italic: bool) -> gpui::Font {
    // Extended weights may have a legacy family such as "Lato Light" while
    // CoreText/fontique register the typographic family "Lato". Resolving the
    // legacy name silently substitutes a system font, invalidating glyph IDs.
    let family = face
        .localized_strings(StringId::TYPOGRAPHIC_FAMILY_NAME)
        .english_or_first()
        .or_else(|| {
            face.localized_strings(StringId::FAMILY_NAME)
                .english_or_first()
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Lato".into());
    let mut font = gpui::font(family);
    font.weight = gpui::FontWeight(weight);
    font.style = if italic {
        gpui::FontStyle::Italic
    } else {
        gpui::FontStyle::Normal
    };
    font
}
impl TextLayout {
    /// Positioned glyphs and their actual fonts, shared by native paint and export.
    pub fn glyph_layout(&self) -> &Layout<Color> {
        &self.layout
    }
    fn cursor(&self, index: usize) -> Cursor {
        Cursor::from_byte_index(&self.layout, index.min(self.len), Affinity::Downstream)
    }
    pub fn hit_test(&self, x: f32, y: f32) -> usize {
        Cursor::from_point(&self.layout, x, y).index().min(self.len)
    }
    pub fn caret(&self, index: usize) -> Rect {
        let mut r = rect(self.cursor(index).geometry(&self.layout, 1.));
        if r.height < 1. {
            r.height = self.height;
        }
        r
    }
    pub fn selection(&self, range: Range<usize>) -> Vec<Rect> {
        if range.is_empty() {
            return vec![];
        }
        Selection::new(self.cursor(range.start), self.cursor(range.end))
            .geometry(&self.layout)
            .into_iter()
            .map(|(r, _)| rect(r))
            .collect()
    }
    pub fn move_vertical(&self, index: usize, down: bool) -> usize {
        let s = Selection::new(self.cursor(index), self.cursor(index));
        if down {
            s.next_line(&self.layout, false)
        } else {
            s.previous_line(&self.layout, false)
        }
        .focus()
        .index()
    }
    pub fn move_horizontal(&self, index: usize, right: bool, word: bool) -> usize {
        let c = self.cursor(index);
        match (right, word) {
            (true, true) => c.next_visual_word(&self.layout),
            (false, true) => c.previous_visual_word(&self.layout),
            (true, false) => c.next_visual(&self.layout),
            (false, false) => c.previous_visual(&self.layout),
        }
        .index()
    }
    pub fn line_boundary(&self, index: usize, end: bool) -> usize {
        let s = Selection::new(self.cursor(index), self.cursor(index));
        if end {
            s.line_end(&self.layout, false)
        } else {
            s.line_start(&self.layout, false)
        }
        .focus()
        .index()
    }
    pub fn word_at(&self, x: f32, y: f32) -> Range<usize> {
        Selection::word_from_point(&self.layout, x, y).text_range()
    }
    #[cfg(feature = "gpui")]
    pub fn paint(
        &self,
        origin: Point<Pixels>,
        zoom: f32,
        opacity: f32,
        window: &mut Window,
        _cx: &mut App,
    ) {
        for line in self.layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(gr) = item else {
                    continue;
                };
                let run = gr.run();
                let attrs = run.font_attrs();
                let face = FontRef::from_index(run.font().data.as_ref(), run.font().index);
                let font = face
                    .map(|f| {
                        paint_font(
                            &f,
                            attrs.weight.value(),
                            attrs.style == parley::FontStyle::Italic,
                        )
                    })
                    .unwrap_or_else(|_| gpui::font("Lato"));
                let emoji = font.family.to_ascii_lowercase().contains("emoji");
                let font_id = window.text_system().resolve_font(&font);
                let c = gr.style().brush;
                let mut color: gpui::Hsla = gpui::rgba(
                    (c.r as u32) << 24 | (c.g as u32) << 16 | (c.b as u32) << 8 | c.a as u32,
                )
                .into();
                color.a *= opacity;
                for g in gr.positioned_glyphs() {
                    let pos = point(origin.x + px(g.x * zoom), origin.y + px(g.y * zoom));
                    let result = if emoji {
                        window.paint_emoji(
                            pos,
                            font_id,
                            gpui::GlyphId(g.id),
                            px(run.font_size() * zoom),
                        )
                    } else {
                        window.paint_glyph(
                            pos,
                            font_id,
                            gpui::GlyphId(g.id),
                            px(run.font_size() * zoom),
                            color,
                        )
                    };
                    if let Err(err) = result {
                        eprintln!("Glyph paint failed: {err}");
                    }
                }
                if gr.style().underline.is_some() {
                    window.paint_quad(gpui::fill(
                        gpui::Bounds::new(
                            point(
                                origin.x + px(gr.offset() * zoom),
                                origin.y + px((gr.baseline() + run.font_size() * 0.1) * zoom),
                            ),
                            gpui::size(
                                px(gr.advance() * zoom),
                                px((run.font_size() * 0.055 * zoom).max(1.)),
                            ),
                        ),
                        color,
                    ));
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_sizes_reflow_and_selection() {
        let mut e = TextEngine::new();
        let mut t = RichText::new("Small and BIG on the same line. Then a long wrapped paragraph.");
        let mut s = t.style.clone();
        s.size = 40.;
        t.apply_style(10..13, s);
        let l = e.layout(&t, 250.);
        assert!(l.height > 50.);
        let c = l.caret(10);
        assert!(c.height >= 40.);
        assert!(!l.selection(0..13).is_empty());
        assert_eq!(l.hit_test(c.x, c.y + c.height / 2.), 10);
        assert!(l.move_vertical(0, true) > 0);
    }
    #[test]
    fn unicode_carets_are_boundaries() {
        let mut e = TextEngine::new();
        let t = RichText::new("Hello 👩‍💻 café\nمرحبا بالعالم");
        let l = e.layout(&t, 180.);
        for y in (0..100).step_by(3) {
            for x in (0..180).step_by(3) {
                assert!(t.content.is_char_boundary(l.hit_test(x as f32, y as f32)));
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "gpui")]
mod bundled_font_tests {
    use super::*;
    #[test]
    fn every_bundled_weight_uses_the_same_canonical_family_for_painting() {
        let mut engine = TextEngine::new();
        for choice in FONT_CHOICES {
            for (weight, _) in choice.weights {
                for italic in [false, true]
                    .into_iter()
                    .filter(|italic| !italic || choice.italic)
                {
                    let mut t = RichText::new("HAMBURGEVONS0123456789");
                    t.style.family = choice.family.into();
                    t.style.weight = *weight;
                    t.style.italic = italic;
                    let layout = engine.layout(&t, 800.);
                    for line in layout.layout.lines() {
                        for item in line.items() {
                            if let PositionedLayoutItem::GlyphRun(gr) = item {
                                let run = gr.run();
                                let face =
                                    FontRef::from_index(run.font().data.as_ref(), run.font().index)
                                        .unwrap();
                                let font = paint_font(
                                    &face,
                                    run.font_attrs().weight.value(),
                                    run.font_attrs().style == parley::FontStyle::Italic,
                                );
                                assert_eq!(
                                    font.family.as_ref(),
                                    choice.family,
                                    "wrong paint family for {} weight {weight}, italic {italic}",
                                    choice.family
                                );
                                assert_eq!(font.weight.0, *weight);
                                assert_eq!(font.style == gpui::FontStyle::Italic, italic);
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn every_bundled_family_is_resolved_by_parley() {
        let mut engine = TextEngine::new();
        for choice in FONT_CHOICES {
            for (weight, _) in choice.weights {
                let mut t = RichText::new("Compose 0123");
                t.style.family = choice.family.into();
                t.style.weight = *weight;
                let layout = engine.layout(&t, 600.);
                let mut glyphs = 0;
                for line in layout.layout.lines() {
                    for item in line.items() {
                        if let PositionedLayoutItem::GlyphRun(gr) = item {
                            let font = gr.run().font();
                            let f = FontRef::from_index(font.data.as_ref(), font.index).unwrap();
                            let name = f
                                .localized_strings(StringId::FAMILY_NAME)
                                .english_or_first()
                                .unwrap()
                                .to_string();
                            assert!(
                                name.starts_with(choice.family),
                                "{} resolved as {name}",
                                choice.family
                            );
                            glyphs += gr.glyphs().count();
                        }
                    }
                }
                assert!(glyphs >= 10);
            }
        }
    }
}
