//! Local, editable starting documents. UI state and previews live in the app.
use crate::{Color, Document, DocumentMode, NodeKind, Rect};
use text::{RichText, TextStyle};
mod designed;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    Blank,
    Resume,
    PhotoResume,
    PhotoCover,
    Portfolio,
    Report,
    Letter,
    Poster,
}
impl Preset {
    pub const ALL: [Self; 8] = [
        Self::Blank,
        Self::Resume,
        Self::PhotoResume,
        Self::PhotoCover,
        Self::Report,
        Self::Letter,
        Self::Portfolio,
        Self::Poster,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Blank => "Blank page",
            Self::Resume => "Résumé",
            Self::PhotoResume => "Photo résumé",
            Self::PhotoCover => "Photo cover",
            Self::Portfolio => "Portfolio",
            Self::Report => "Report",
            Self::Letter => "Letter",
            Self::Poster => "Poster",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Blank => "A fresh A4 page",
            Self::Resume => "Experience, education & skills",
            Self::PhotoResume => "Portrait & two-column résumé",
            Self::PhotoCover => "A large photo & editorial title",
            Self::Portfolio => "Selected work & a project photo",
            Self::Report => "Headings & body text",
            Self::Letter => "A clean personal letter",
            Self::Poster => "Color & canvas layout",
        }
    }
    pub fn supports(self, mode: DocumentMode) -> bool {
        !matches!(
            self,
            Self::Poster | Self::PhotoResume | Self::PhotoCover | Self::Portfolio
        ) || mode == DocumentMode::Compose
    }
    pub fn has_photo(self) -> bool {
        matches!(self, Self::PhotoResume | Self::PhotoCover | Self::Portfolio)
    }
    pub fn document_name(self) -> &'static str {
        match self {
            Self::Blank => "Untitled",
            Self::Resume => "My résumé",
            Self::PhotoResume => "My photo résumé",
            Self::PhotoCover => "Photo cover",
            Self::Portfolio => "My portfolio",
            Self::Report => "Project report",
            Self::Letter => "Personal letter",
            Self::Poster => "Studio poster",
        }
    }
    pub fn create(self, mode: DocumentMode) -> Result<Document, String> {
        if !self.supports(mode) {
            return Err(format!(
                "{} uses canvas placement or effects. Choose Compose to keep its design.",
                self.name()
            ));
        }
        let mut doc = Document {
            mode,
            name: self.document_name().into(),
            ..Default::default()
        };
        let width = doc.pages[0].width - 144.;
        let mut y = 72.;
        let mut block = |name: &str, content: &str, size: f32, weight: f32, height: f32| {
            let mut rich = RichText::new(content);
            rich.style = TextStyle {
                size,
                weight,
                line_height: 1.4,
                ..Default::default()
            };
            let id = doc.insert(0, NodeKind::Text(rich), Rect::new(72., y, width, height));
            doc.node_mut(id).unwrap().name = name.into();
            y += height + 12.;
        };
        match self {
            Self::Resume | Self::PhotoResume | Self::PhotoCover | Self::Portfolio => {
                designed::populate(self, &mut doc)
            }
            Self::Blank => {}
            Self::Report => {
                block("Title", "Project report", 32., 700., 45.);
                block(
                    "Byline",
                    "Your name · Course or team · Date",
                    12.,
                    400.,
                    22.,
                );
                block("Overview heading", "Overview", 20., 700., 28.);
                block(
                    "Overview",
                    "Introduce your topic and the question you want to answer. Explain why it matters and what this report covers.",
                    14.,
                    400.,
                    62.,
                );
                block("Findings heading", "Key findings", 20., 700., 28.);
                block(
                    "Findings",
                    "Describe your most useful finding here. Support it with a source, an example or a result.\n\nAdd a second observation and connect it to your main question.",
                    14.,
                    400.,
                    116.,
                );
                block("Next steps heading", "Next steps", 20., 700., 28.);
                block(
                    "Next steps",
                    "Summarize what you learned and what should happen next. Add references or a final recommendation below.",
                    14.,
                    400.,
                    62.,
                );
            }
            Self::Letter => {
                block("Sender", "Your name", 26., 700., 38.);
                block("Contact", "Address · Email · Phone", 12., 400., 22.);
                block("Date", "Date", 13., 400., 22.);
                block("Greeting", "Dear recipient,", 16., 400., 26.);
                block(
                    "Letter",
                    "Start with the reason for your letter. Keep your introduction clear and personal.\n\nUse this paragraph to add the details, context or request you want to share.\n\nClose with your next step and thank the reader for their time.",
                    14.,
                    400.,
                    174.,
                );
                block("Signature", "Kind regards,\nYour name", 14., 400., 48.);
            }
            Self::Poster => {
                let id = doc.insert(0, NodeKind::Shape, Rect::new(36., 36., 523.28, 769.89));
                let panel = doc.node_mut(id).unwrap();
                panel.name = "Sage background".into();
                panel.fill = Color::rgb(221, 230, 209);
                panel.radius = 18.;
                let mut add = |name: &str, content: &str, rect: Rect, size: f32, weight: f32| {
                    let mut rich = RichText::new(content);
                    rich.style = TextStyle {
                        size,
                        weight,
                        line_height: 1.05,
                        color: Color::rgb(45, 64, 47),
                        ..Default::default()
                    };
                    let id = doc.insert(0, NodeKind::Text(rich), rect);
                    doc.node_mut(id).unwrap().name = name.into();
                };
                add(
                    "Category",
                    "IDEAS / PEOPLE / MAKING",
                    Rect::new(72., 84., 430., 24.),
                    12.,
                    700.,
                );
                add(
                    "Headline",
                    "Make\nsomething\nmatter.",
                    Rect::new(68., 162., 460., 258.),
                    72.,
                    850.,
                );
                add(
                    "Description",
                    "A space for good ideas.\nAn evening of creating together.",
                    Rect::new(72., 465., 430., 90.),
                    23.,
                    400.,
                );
                add(
                    "Details",
                    "SATURDAY  /  18:00\nYour venue · Your city",
                    Rect::new(72., 706., 430., 54.),
                    14.,
                    700.,
                );
                let id = doc.insert(0, NodeKind::Shape, Rect::new(72., 626., 451., 2.));
                let line = doc.node_mut(id).unwrap();
                line.name = "Divider".into();
                line.fill = Color::rgb(90, 116, 80);
            }
        }
        doc.validate()?;
        Ok(doc)
    }
}
