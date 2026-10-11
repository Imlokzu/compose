//! Editable document layouts, with original sample pixels stored as normal assets.
use super::Preset;
use crate::{Color, Document, NodeKind, Rect};
use std::sync::Arc;
use text::{RichText, TextStyle};

const INK: Color = Color::rgb(35, 42, 37);
const MUTED: Color = Color::rgb(103, 112, 104);
const ACCENT: Color = Color::rgb(56, 93, 70);
const LINE: Color = Color::rgb(222, 227, 218);

fn style(size: f32, weight: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        weight,
        color,
        line_height: 1.4,
        ..Default::default()
    }
}
fn text(doc: &mut Document, name: &str, content: &str, frame: Rect, style: TextStyle) {
    let mut rich = RichText::new(content);
    rich.style = style;
    let id = doc.insert(0, NodeKind::Text(rich), frame);
    doc.node_mut(id).unwrap().name = name.into();
}
fn shape(doc: &mut Document, name: &str, frame: Rect, color: Color) {
    let id = doc.insert(0, NodeKind::Shape, frame);
    let node = doc.node_mut(id).unwrap();
    node.name = name.into();
    node.fill = color;
}
fn photo(doc: &mut Document, name: &str, portrait: bool, frame: Rect, radius: f32) {
    let (key, bytes): (&str, &[u8]) = if portrait {
        (
            "compose-sample-portrait",
            include_bytes!("../../../../assets/presets/portrait.png"),
        )
    } else {
        (
            "compose-sample-architecture",
            include_bytes!("../../../../assets/presets/architecture.png"),
        )
    };
    doc.assets.insert(key.into(), Arc::new(bytes.to_vec()));
    let id = doc.insert(0, NodeKind::Image { asset: key.into() }, frame);
    let node = doc.node_mut(id).unwrap();
    node.name = name.into();
    node.radius = radius;
}
fn resume(doc: &mut Document) {
    let w = doc.pages[0].width - 144.;
    let mut y = 72.;
    let mut block =
        |name: &str, content: &str, size: f32, weight: f32, height: f32, color: Color| {
            text(
                doc,
                name,
                content,
                Rect::new(72., y, w, height),
                style(size, weight, color),
            );
            y += height + 12.;
        };
    block("Name", "Alex Morgan", 30., 700., 42., INK);
    block("Role", "Product designer", 12., 400., 20., ACCENT);
    block(
        "Contact",
        "alex@example.com · portfolio.example · Berlin, Germany",
        10.,
        400.,
        16.,
        MUTED,
    );
    block("Profile heading", "Profile", 13., 700., 20., INK);
    block(
        "Profile",
        "Thoughtful designer creating useful, accessible digital products. Experienced in research, clear interfaces and working with engineering teams.",
        10.5,
        400.,
        46.,
        INK,
    );
    block("Experience heading", "Experience", 13., 700., 20., INK);
    block(
        "Experience",
        "Product designer · Studio North\n2023 — Present\n• Led product design from discovery to launch.\n• Built a reusable, accessible design system.\n• Partnered with researchers and engineers.\n\nJunior designer · Common Ground\n2021 — 2023\n• Turned complex workflows into clear interfaces.",
        10.5,
        400.,
        150.,
        INK,
    );
    block("Education heading", "Education", 13., 700., 20., INK);
    block(
        "Education",
        "BA in Communication Design · Your university\n2017 — 2021",
        10.5,
        400.,
        32.,
        INK,
    );
    block("Skills heading", "Skills", 13., 700., 20., INK);
    block(
        "Skills",
        "Product strategy · User research · Interaction design\nPrototyping · Accessibility · Design systems",
        10.5,
        400.,
        32.,
        INK,
    );
    let node = doc.pages[0]
        .nodes
        .iter_mut()
        .find(|n| n.name == "Experience")
        .unwrap();
    let NodeKind::Text(rich) = &mut node.kind else {
        unreachable!()
    };
    for title in [
        "Product designer · Studio North",
        "Junior designer · Common Ground",
    ] {
        let start = rich.content.find(title).unwrap();
        rich.apply_style(start..start + title.len(), style(10.5, 700., INK));
    }
}
fn photo_resume(doc: &mut Document) {
    shape(doc, "Header accent", Rect::new(48., 50., 32., 3.), ACCENT);
    text(
        doc,
        "Name",
        "Alex Morgan",
        Rect::new(48., 69., 346., 54.),
        style(34., 700., INK),
    );
    text(
        doc,
        "Role",
        "Product designer",
        Rect::new(48., 128., 328., 26.),
        style(15., 400., ACCENT),
    );
    text(
        doc,
        "Introduction",
        "Useful products. Clear experiences.",
        Rect::new(48., 164., 329., 22.),
        style(11.5, 400., MUTED),
    );
    photo(
        doc,
        "Portrait · replace with your photo",
        true,
        Rect::new(423., 62., 124., 124.),
        12.,
    );
    shape(doc, "Header divider", Rect::new(48., 215., 499., 1.), LINE);
    shape(doc, "Column divider", Rect::new(210., 246., 1., 524.), LINE);
    text(
        doc,
        "Contact heading",
        "CONTACT",
        Rect::new(48., 247., 148., 18.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Contact",
        "alex@example.com\nportfolio.example\nBerlin, Germany",
        Rect::new(48., 275., 148., 72.),
        style(10.5, 400., INK),
    );
    text(
        doc,
        "Skills heading",
        "SKILLS",
        Rect::new(48., 391., 148., 18.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Skills",
        "Product strategy\nUser research\nInteraction design\nPrototyping\nAccessibility\nDesign systems",
        Rect::new(48., 419., 148., 105.),
        style(10.5, 400., INK),
    );
    text(
        doc,
        "Education heading",
        "EDUCATION",
        Rect::new(48., 588., 148., 18.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Education",
        "BA, Communication\nDesign\nYour university\n2017 — 2021",
        Rect::new(48., 616., 148., 80.),
        style(10.5, 400., INK),
    );
    text(
        doc,
        "Languages heading",
        "LANGUAGES",
        Rect::new(48., 727., 148., 18.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Languages",
        "English · German",
        Rect::new(48., 755., 148., 24.),
        style(10.5, 400., INK),
    );
    text(
        doc,
        "Profile heading",
        "PROFILE",
        Rect::new(235., 247., 312., 18.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Profile",
        "Thoughtful designer creating useful, accessible digital products. I turn research into clear interfaces and work with teams from first idea to launch.",
        Rect::new(235., 275., 312., 82.),
        style(11.5, 400., INK),
    );
    text(
        doc,
        "Experience heading",
        "EXPERIENCE",
        Rect::new(235., 387., 312., 18.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Current role",
        "Product designer",
        Rect::new(235., 420., 312., 26.),
        style(16., 700., INK),
    );
    text(
        doc,
        "Current employer",
        "Studio North · 2023 — Present",
        Rect::new(235., 451., 312., 20.),
        style(10.5, 400., MUTED),
    );
    text(
        doc,
        "Current work",
        "• Led product design from discovery to launch.\n• Built an accessible, reusable design system.\n• Worked with research and engineering.",
        Rect::new(235., 481., 312., 82.),
        style(11.5, 400., INK),
    );
    text(
        doc,
        "Previous role",
        "Junior designer",
        Rect::new(235., 591., 312., 26.),
        style(16., 700., INK),
    );
    text(
        doc,
        "Previous employer",
        "Common Ground · 2021 — 2023",
        Rect::new(235., 622., 312., 20.),
        style(10.5, 400., MUTED),
    );
    text(
        doc,
        "Previous work",
        "• Turned complex tasks into clear interfaces.\n• Prototyped and tested ideas with real users.\n• Helped grow a consistent design language.",
        Rect::new(235., 652., 312., 82.),
        style(11.5, 400., INK),
    );
}
fn photo_cover(doc: &mut Document) {
    let w = doc.pages[0].width - 88.;
    text(
        doc,
        "Category",
        "STUDIO JOURNAL  /  01",
        Rect::new(44., 50., w, 22.),
        style(11., 700., ACCENT),
    );
    let mut headline = style(46., 400., INK);
    headline.family = "Noto Serif".into();
    headline.line_height = 1.12;
    text(
        doc,
        "Title",
        "A quieter\nperspective.",
        Rect::new(44., 109., w, 117.),
        headline,
    );
    photo(
        doc,
        "Cover photo · replace with your image",
        false,
        Rect::new(44., 274., w, w / 1.5),
        8.,
    );
    text(
        doc,
        "Photo caption",
        "SPACE / LIGHT / MATERIAL",
        Rect::new(44., 636., w, 22.),
        style(10., 700., ACCENT),
    );
    text(
        doc,
        "Description",
        "A collection of places, ideas and details.\nMake this cover your own with a title and photograph.",
        Rect::new(44., 678., w, 55.),
        style(13., 400., INK),
    );
    shape(doc, "Footer divider", Rect::new(44., 772., w, 1.), LINE);
    text(
        doc,
        "Author",
        "Alex Morgan",
        Rect::new(44., 791., 300., 18.),
        style(10., 400., MUTED),
    );
    text(
        doc,
        "Edition",
        "AUTUMN 2026",
        Rect::new(430., 791., 121., 18.),
        style(10., 700., ACCENT),
    );
}
fn portfolio(doc: &mut Document) {
    let w = doc.pages[0].width - 88.;
    text(
        doc,
        "Portfolio label",
        "ALEX MORGAN  /  SPATIAL DESIGNER",
        Rect::new(44., 48., w, 22.),
        style(10.5, 700., ACCENT),
    );
    text(
        doc,
        "Title",
        "Selected work",
        Rect::new(44., 88., w, 60.),
        style(38., 700., INK),
    );
    text(
        doc,
        "Intro",
        "Spaces shaped by light, materials and everyday life.",
        Rect::new(44., 151., w, 22.),
        style(12., 400., MUTED),
    );
    photo(
        doc,
        "Project photo · replace with your work",
        false,
        Rect::new(44., 198., w, w / 1.5),
        6.,
    );
    text(
        doc,
        "Project number",
        "01  /  COASTAL HOUSE",
        Rect::new(44., 561., w, 22.),
        style(10.5, 700., ACCENT),
    );
    text(
        doc,
        "Project title",
        "Room to breathe",
        Rect::new(44., 593., w, 39.),
        style(26., 700., INK),
    );
    text(
        doc,
        "Project description",
        "A quiet place by the sea. Simple volumes, warm materials and framed views create a home connected to its landscape.",
        Rect::new(44., 648., 323., 66.),
        style(12., 400., INK),
    );
    text(
        doc,
        "Project details",
        "ROLE\nConcept & interior design\n\nYEAR\n2026",
        Rect::new(402., 648., 149., 88.),
        style(10.5, 400., MUTED),
    );
    shape(doc, "Footer divider", Rect::new(44., 773., w, 1.), LINE);
    text(
        doc,
        "Website",
        "portfolio.example",
        Rect::new(44., 791., w, 18.),
        style(10., 400., MUTED),
    );
}
pub(super) fn populate(preset: Preset, doc: &mut Document) {
    match preset {
        Preset::Resume => resume(doc),
        Preset::PhotoResume => photo_resume(doc),
        Preset::PhotoCover => photo_cover(doc),
        Preset::Portfolio => portfolio(doc),
        _ => unreachable!("Only designed presets use this module"),
    }
}
