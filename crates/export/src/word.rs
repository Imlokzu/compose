//! Editable DOCX output for the restricted Word document mode.
use anyhow::{Context, Result, ensure};
use document::{Document, NodeKind, WORD_BLOCK_GAP, WORD_MARGIN};
use docx_rs::{
    AlignmentType, Docx, Hyperlink, HyperlinkType, LineSpacing, LineSpacingType, PageMargin,
    PageSize, Paragraph, Pic, Run, RunFonts, SectionProperty, SectionType,
};
use std::{io::Cursor, path::Path};
use text::{RichText, TextAlign, TextStyle};

fn twips(points: f32) -> u32 {
    (points * 20.).round() as u32
}
fn margin() -> PageMargin {
    PageMargin::new()
        .top(twips(WORD_MARGIN) as i32)
        .bottom(twips(WORD_MARGIN) as i32)
        .left(twips(WORD_MARGIN) as i32)
        .right(twips(WORD_MARGIN) as i32)
        .header(0)
        .footer(0)
}
fn run_style(style: &TextStyle) -> Run {
    let mut family = style.family.clone();
    // Word's font-family selector uses the actual static family for extended weights.
    if family == "Lato" {
        if style.weight < 350. {
            family = "Lato Light".into();
        } else if style.weight >= 850. {
            family = "Lato Black".into();
        }
    }
    let mut run = Run::new()
        .fonts(
            RunFonts::new()
                .ascii(&family)
                .hi_ansi(&family)
                .east_asia(&family)
                .cs(&family),
        )
        .size((style.size * 2.).round() as usize)
        .color(format!(
            "{:02X}{:02X}{:02X}",
            style.color.r, style.color.g, style.color.b
        ))
        .character_spacing((style.letter_spacing * 20.).round() as i32);
    if style.weight >= 600. && style.weight < 850. {
        run = run.bold();
    }
    if style.italic {
        run = run.italic();
    }
    if style.underline {
        run = run.underline("single");
    }
    run
}
fn paragraphs(text: &RichText) -> Vec<Paragraph> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut last_height = text.style.size * text.style.line_height;
    for line in text.content.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let end = start + line.len();
        let style = text.style_at(start);
        let line_height = text
            .spans
            .iter()
            .filter(|s| s.range.start < end && s.range.end > start)
            .map(|s| s.style.size * s.style.line_height)
            .fold(style.size * style.line_height, f32::max);
        last_height = line_height;
        let mut para = Paragraph::new()
            .align(match text.align {
                TextAlign::Left => AlignmentType::Left,
                TextAlign::Center => AlignmentType::Center,
                TextAlign::Right => AlignmentType::Right,
                TextAlign::Justify => AlignmentType::Both,
            })
            .line_spacing(
                LineSpacing::new()
                    .before(0)
                    .after(0)
                    .line_rule(LineSpacingType::Exact)
                    .line(twips(line_height) as i32),
            );
        if line.is_empty() {
            para = para.add_run(run_style(&style).add_text(""));
        } else {
            let mut boundaries = vec![start, end];
            for span in &text.spans {
                for pos in [span.range.start, span.range.end] {
                    if pos > start && pos < end {
                        boundaries.push(pos);
                    }
                }
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            for bounds in boundaries.windows(2) {
                let style = text.style_at(bounds[0]);
                let content = &text.content[bounds[0]..bounds[1]];
                let mut run = run_style(&style);
                for (index, part) in content.split('\t').enumerate() {
                    if index > 0 {
                        run = run.add_tab();
                    }
                    if !part.is_empty() {
                        run = run.add_text(part);
                    }
                }
                para = if let Some(url) = &style.link {
                    para.add_hyperlink(Hyperlink::new(url, HyperlinkType::External).add_run(run))
                } else {
                    para.add_run(run)
                };
            }
        }
        output.push(para);
        start += text.content[start..]
            .find('\n')
            .map(|offset| offset + 1)
            .unwrap_or(text.content.len() - start);
    }
    if let Some(last) = output.last_mut() {
        *last = last.clone().line_spacing(
            LineSpacing::new()
                .before(0)
                .after(twips(WORD_BLOCK_GAP))
                .line_rule(LineSpacingType::Exact)
                .line(twips(last_height) as i32),
        );
    }
    output
}
fn image(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    ensure!(bytes.len() <= 64 * 1024 * 1024, "Image exceeds 64 MB");
    let reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let (w, h) = reader.into_dimensions()?;
    ensure!(
        w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 32_000_000,
        "Image exceeds 32 megapixels"
    );
    let decoded = image::load_from_memory(bytes)?;
    let mut png = Cursor::new(Vec::new());
    decoded.write_to(&mut png, image::ImageFormat::Png)?;
    Ok((png.into_inner(), w, h))
}
pub fn write_docx(path: &Path, document: &Document) -> Result<()> {
    document.validate().map_err(anyhow::Error::msg)?;
    document.validate_word().map_err(anyhow::Error::msg)?;
    let mut doc = Docx::new()
        .default_size(32)
        .default_fonts(RunFonts::new().ascii("Lato").hi_ansi("Lato"))
        .page_margin(margin());
    for (index, page) in document.pages.iter().enumerate() {
        let mut output = Vec::new();
        for node in &page.nodes {
            if !node.visible {
                continue;
            }
            match &node.kind {
                NodeKind::Text(text) => output.extend(paragraphs(text)),
                NodeKind::Image { asset } => {
                    let (png, w, h) = image(document.assets.get(asset).context("Missing image")?)?;
                    let max_w = page.width - 2. * WORD_MARGIN;
                    let max_h = page.height - 2. * WORD_MARGIN - WORD_BLOCK_GAP;
                    let factor = (max_w / node.frame.width)
                        .min(max_h / node.frame.height)
                        .min(1.);
                    let picture = Pic::new_with_dimensions(png, w, h).size(
                        (node.frame.width * factor * 12700.).round() as u32,
                        (node.frame.height * factor * 12700.).round() as u32,
                    );
                    output.push(
                        Paragraph::new()
                            .add_run(Run::new().add_image(picture))
                            .line_spacing(
                                LineSpacing::new().before(0).after(twips(WORD_BLOCK_GAP)),
                            ),
                    );
                }
                _ => anyhow::bail!("Canvas objects cannot be exported to Word"),
            }
        }
        if output.is_empty() {
            output.push(Paragraph::new().add_run(Run::new().add_text("")));
        }
        if index + 1 < document.pages.len() {
            let mut section = SectionProperty::new()
                .page_size(
                    PageSize::new()
                        .width(twips(page.width))
                        .height(twips(page.height)),
                )
                .page_margin(margin());
            section.section_type = Some(SectionType::NextPage);
            let last = output.last_mut().unwrap();
            *last = last.clone().section_property(section);
        }
        for paragraph in output {
            doc = doc.add_paragraph(paragraph);
        }
    }
    let last = document.pages.last().unwrap();
    doc = doc.page_size(twips(last.width), twips(last.height));
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).context("Could not create the Word file")?;
    doc.build()
        .pack(temp.as_file_mut())
        .context("Could not write DOCX")?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .context("Could not replace the exported Word file")?;
    Ok(())
}
