//! Page output in document coordinates. No window, selection or camera state.
use anyhow::{Context, Result, ensure};
use document::{Color, Document, Node, NodeKind, Rect};
use parley::PositionedLayoutItem;
use renderer::TextEngine;
use std::{collections::HashMap, io::Write, path::Path};
use swash::{
    FontRef,
    scale::{Render, ScaleContext, Source, StrikeWith, image::Content},
    zeno::{Format, Vector},
};
use tiny_skia::{FillRule, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Transform};

pub const PAGE_BACKGROUND: Color = Color::rgb(255, 254, 251);
pub mod word;
const MAX_PIXELS: u64 = 32_000_000;

pub fn dimensions(width: f32, height: f32, dpi: u32) -> Result<(u32, u32)> {
    ensure!(
        (72..=600).contains(&dpi),
        "Resolution must be between 72 and 600 DPI"
    );
    ensure!(
        width.is_finite() && height.is_finite() && width > 0. && height > 0.,
        "Invalid page size"
    );
    let scale = dpi as f64 / 72.;
    let w = (width as f64 * scale).ceil();
    let h = (height as f64 * scale).ceil();
    ensure!(
        w <= MAX_PIXELS as f64 && h <= MAX_PIXELS as f64 && w * h <= MAX_PIXELS as f64,
        "This page exceeds the 32-megapixel export limit; choose a lower resolution"
    );
    Ok((w as u32, h as u32))
}

pub struct PageRenderer {
    text: TextEngine,
    glyphs: ScaleContext,
    images: HashMap<String, Pixmap>,
}
impl Default for PageRenderer {
    fn default() -> Self {
        Self {
            text: TextEngine::new(),
            glyphs: ScaleContext::new(),
            images: HashMap::new(),
        }
    }
}
impl PageRenderer {
    pub fn render(&mut self, doc: &Document, page: usize, dpi: u32) -> Result<Pixmap> {
        doc.validate().map_err(anyhow::Error::msg)?;
        ensure!(
            !doc.word_overflows(page),
            "A Word block extends past this page. Split it onto another page or export DOCX, where Word paginates its text"
        );
        let page = doc.pages.get(page).context("Page no longer exists")?;
        let (w, h) = dimensions(page.width, page.height, dpi)?;
        let mut canvas = Pixmap::new(w, h).context("Could not allocate the exported page")?;
        canvas.fill(tiny_skia::Color::from_rgba8(
            PAGE_BACKGROUND.r,
            PAGE_BACKGROUND.g,
            PAGE_BACKGROUND.b,
            255,
        ));
        self.images.clear();
        self.nodes(doc, &page.nodes, 0., 0., 1., dpi as f32 / 72., &mut canvas)?;
        Ok(canvas)
    }

    #[allow(clippy::too_many_arguments)]
    fn nodes(
        &mut self,
        doc: &Document,
        nodes: &[Node],
        x: f32,
        y: f32,
        opacity: f32,
        scale: f32,
        canvas: &mut Pixmap,
    ) -> Result<()> {
        for node in nodes {
            if !node.visible {
                continue;
            }
            let r = Rect::new(
                x + node.frame.x,
                y + node.frame.y,
                node.frame.width,
                node.frame.height,
            );
            let opacity = opacity * node.opacity;
            if opacity <= 0. {
                continue;
            }
            let path = rounded_rect(r, node.radius);
            if node.appearance.shadow && !matches!(node.kind, NodeKind::Text(_)) {
                shadow(canvas, r, node, scale, opacity)?;
            }
            match &node.kind {
                NodeKind::Shape | NodeKind::Frame => {
                    canvas.fill_path(
                        &path,
                        &paint(node.fill, opacity),
                        FillRule::Winding,
                        Transform::from_scale(scale, scale),
                        None,
                    );
                }
                NodeKind::Image { asset } => {
                    if !self.images.contains_key(asset) {
                        let bytes = doc.assets.get(asset).context("Missing embedded image")?;
                        let reader =
                            image::ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
                                .with_guessed_format()?;
                        let (w, h) = reader.into_dimensions()?;
                        ensure!(
                            u64::from(w) * u64::from(h) <= MAX_PIXELS,
                            "Embedded image exceeds 32 megapixels"
                        );
                        let image = image::load_from_memory(bytes)?.into_rgba8();
                        self.images
                            .insert(asset.clone(), rgba_pixmap(w, h, image.as_raw())?);
                    }
                    let image = &self.images[asset];
                    let picture = layout::image_bounds(
                        r,
                        image.width() as f32,
                        image.height() as f32,
                        node.image_sizing,
                    );
                    let mask = clip(canvas, picture, node.radius, scale)?;
                    let transform = Transform::from_row(
                        picture.width * scale / image.width() as f32,
                        0.,
                        0.,
                        picture.height * scale / image.height() as f32,
                        picture.x * scale,
                        picture.y * scale,
                    );
                    canvas.draw_pixmap(
                        0,
                        0,
                        image.as_ref(),
                        &PixmapPaint {
                            opacity,
                            quality: tiny_skia::FilterQuality::Bicubic,
                            ..Default::default()
                        },
                        transform,
                        Some(&mask),
                    );
                }
                NodeKind::Text(text) => {
                    let mask = clip(canvas, r, 0., scale)?;
                    let layout = self.text.layout(text, r.width);
                    for line in layout.glyph_layout().lines() {
                        for item in line.items() {
                            let PositionedLayoutItem::GlyphRun(gr) = item else {
                                continue;
                            };
                            let run = gr.run();
                            let font = FontRef::from_index(
                                run.font().data.as_ref(),
                                run.font().index as usize,
                            )
                            .context("Could not read the document font")?;
                            let mut scaler = self
                                .glyphs
                                .builder(font)
                                .size(run.font_size() * scale)
                                .hint(false)
                                .normalized_coords(run.normalized_coords())
                                .build();
                            let color = gr.style().brush;
                            for glyph in gr.positioned_glyphs() {
                                let gx = (r.x + glyph.x) * scale;
                                let gy = (r.y + glyph.y) * scale;
                                let Some(image) = Render::new(&[
                                    Source::ColorOutline(0),
                                    Source::ColorBitmap(StrikeWith::BestFit),
                                    Source::Outline,
                                ])
                                .format(Format::Alpha)
                                .offset(Vector::new(gx - gx.floor(), gy - gy.floor()))
                                .render(&mut scaler, glyph.id as u16) else {
                                    continue;
                                };
                                let p = image.placement;
                                if p.width == 0 || p.height == 0 {
                                    continue;
                                }
                                let rgba = match image.content {
                                    Content::Mask => image
                                        .data
                                        .iter()
                                        .flat_map(|alpha| {
                                            [
                                                color.r,
                                                color.g,
                                                color.b,
                                                ((*alpha as u16 * color.a as u16 + 127) / 255)
                                                    as u8,
                                            ]
                                        })
                                        .collect(),
                                    Content::Color => image.data,
                                    Content::SubpixelMask => {
                                        anyhow::bail!("Unexpected subpixel glyph format")
                                    }
                                };
                                let glyph = rgba_pixmap(p.width, p.height, &rgba)?;
                                canvas.draw_pixmap(
                                    gx.floor() as i32 + p.left,
                                    gy.floor() as i32 - p.top,
                                    glyph.as_ref(),
                                    &PixmapPaint {
                                        opacity,
                                        ..Default::default()
                                    },
                                    Transform::identity(),
                                    Some(&mask),
                                );
                            }
                            if gr.style().underline.is_some() {
                                let underline = rounded_rect(
                                    Rect::new(
                                        r.x + gr.offset(),
                                        r.y + gr.baseline() + run.font_size() * 0.1,
                                        gr.advance(),
                                        run.font_size() * 0.055,
                                    ),
                                    0.,
                                );
                                canvas.fill_path(
                                    &underline,
                                    &paint(color, opacity),
                                    FillRule::Winding,
                                    Transform::from_scale(scale, scale),
                                    Some(&mask),
                                );
                            }
                        }
                    }
                }
            }
            if node.appearance.stroke_width > 0. && !matches!(node.kind, NodeKind::Text(_)) {
                let mask = clip(canvas, r, node.radius, scale)?;
                canvas.stroke_path(
                    &path,
                    &paint(node.appearance.stroke, opacity),
                    &tiny_skia::Stroke {
                        width: node.appearance.stroke_width * 2.,
                        ..Default::default()
                    },
                    Transform::from_scale(scale, scale),
                    Some(&mask),
                );
            }
            self.nodes(doc, &node.children, r.x, r.y, opacity, scale, canvas)?;
        }
        Ok(())
    }
}

fn paint(color: Color, opacity: f32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        color.r,
        color.g,
        color.b,
        (color.a as f32 * opacity).round() as u8,
    );
    paint
}
fn rgba_pixmap(w: u32, h: u32, bytes: &[u8]) -> Result<Pixmap> {
    let mut pixmap = Pixmap::new(w, h).context("Could not allocate image pixels")?;
    ensure!(
        pixmap.data().len() == bytes.len(),
        "Invalid decoded image length"
    );
    for (out, src) in pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(bytes.as_chunks::<4>().0)
    {
        let a = src[3] as u16;
        out.copy_from_slice(&[
            ((src[0] as u16 * a + 127) / 255) as u8,
            ((src[1] as u16 * a + 127) / 255) as u8,
            ((src[2] as u16 * a + 127) / 255) as u8,
            src[3],
        ]);
    }
    Ok(pixmap)
}
fn clip(canvas: &Pixmap, rect: Rect, radius: f32, scale: f32) -> Result<Mask> {
    let mut mask =
        Mask::new(canvas.width(), canvas.height()).context("Could not allocate page clipping")?;
    mask.fill_path(
        &rounded_rect(rect, radius),
        FillRule::Winding,
        true,
        Transform::from_scale(scale, scale),
    );
    Ok(mask)
}
fn rounded_rect(r: Rect, radius: f32) -> tiny_skia::Path {
    let d = radius.min(r.width.min(r.height) / 2.);
    let (x, y, right, bottom) = (r.x, r.y, r.x + r.width, r.y + r.height);
    let mut p = PathBuilder::new();
    if d <= 0. {
        p.move_to(x, y);
        p.line_to(right, y);
        p.line_to(right, bottom);
        p.line_to(x, bottom);
        p.close();
    } else {
        let k = d * 0.552_284_8;
        p.move_to(x + d, y);
        p.line_to(right - d, y);
        p.cubic_to(right - d + k, y, right, y + d - k, right, y + d);
        p.line_to(right, bottom - d);
        p.cubic_to(
            right,
            bottom - d + k,
            right - d + k,
            bottom,
            right - d,
            bottom,
        );
        p.line_to(x + d, bottom);
        p.cubic_to(x + d - k, bottom, x, bottom - d + k, x, bottom - d);
        p.line_to(x, y + d);
        p.cubic_to(x, y + d - k, x + d - k, y, x + d, y);
        p.close();
    }
    p.finish().expect("Validated rectangle")
}
fn shadow(canvas: &mut Pixmap, r: Rect, node: &Node, scale: f32, opacity: f32) -> Result<()> {
    let sigma = node.appearance.shadow_blur * scale / 2.;
    let pad = (sigma * 3.).ceil() + 2.;
    let x = (r.x * scale - pad)
        .floor()
        .max(0.)
        .min(canvas.width() as f32);
    let y = ((r.y + node.appearance.shadow_y) * scale - pad)
        .floor()
        .max(0.)
        .min(canvas.height() as f32);
    let right = ((r.x + r.width) * scale + pad)
        .ceil()
        .max(0.)
        .min(canvas.width() as f32);
    let bottom = ((r.y + r.height + node.appearance.shadow_y) * scale + pad)
        .ceil()
        .max(0.)
        .min(canvas.height() as f32);
    let (w, h) = ((right - x) as u32, (bottom - y) as u32);
    if w == 0 || h == 0 {
        return Ok(());
    }
    let local = Rect::new(
        r.x * scale - x,
        (r.y + node.appearance.shadow_y) * scale - y,
        r.width * scale,
        r.height * scale,
    );
    let mut mask = Mask::new(w, h).context("Could not allocate shadow")?;
    mask.fill_path(
        &rounded_rect(local, node.radius * scale),
        FillRule::Winding,
        true,
        Transform::identity(),
    );
    let alpha = image::GrayImage::from_raw(w, h, mask.data().to_vec()).context("Invalid shadow")?;
    let alpha = if sigma > 0. {
        image::imageops::blur(&alpha, sigma)
    } else {
        alpha
    };
    let rgba: Vec<_> = alpha
        .as_raw()
        .iter()
        .flat_map(|a| [0, 0, 0, (*a as f32 * 0.16 * opacity).round() as u8])
        .collect();
    let shadow = rgba_pixmap(w, h, &rgba)?;
    canvas.draw_pixmap(
        x as i32,
        y as i32,
        shadow.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    Ok(())
}

/// Atomic destination replacement; a failed render cannot truncate an existing export.
pub fn write_png(path: &Path, doc: &Document, page: usize, dpi: u32) -> Result<(u32, u32)> {
    let pixmap = PageRenderer::default().render(doc, page, dpi)?;
    save_png(path, &pixmap, dpi)?;
    Ok((pixmap.width(), pixmap.height()))
}
pub fn save_png(path: &Path, pixmap: &Pixmap, dpi: u32) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file =
        tempfile::NamedTempFile::new_in(parent).context("Could not create the PNG file")?;
    {
        let mut encoder = png::Encoder::new(file.as_file_mut(), pixmap.width(), pixmap.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let ppm = (dpi as f64 / 0.0254).round() as u32;
        encoder.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
        // The page is opaque, so composited premultiplied channels equal RGBA.
        encoder.write_header()?.write_image_data(pixmap.data())?;
    }
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|e| e.error)
        .context("Could not replace the exported PNG")?;
    Ok(())
}
