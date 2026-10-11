use crate::Editor;
use document::{Appearance, Color, ImageSizing, Node, NodeKind, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    Title,
    Callout,
    Quote,
    Stat,
    Card,
    Divider,
}
impl Component {
    pub const ALL: [Self; 6] = [
        Self::Title,
        Self::Callout,
        Self::Quote,
        Self::Stat,
        Self::Card,
        Self::Divider,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Title => "Title & introduction",
            Self::Callout => "Callout",
            Self::Quote => "Pull quote",
            Self::Stat => "Key number",
            Self::Card => "Feature card",
            Self::Divider => "Section divider",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Title => "A confident opening",
            Self::Callout => "Give an idea some space",
            Self::Quote => "Words worth highlighting",
            Self::Stat => "Make a number stand out",
            Self::Card => "A clear, compact feature",
            Self::Divider => "A pause between sections",
        }
    }
}
struct Builder {
    next: u64,
}
impl Builder {
    fn node(&mut self, name: &str, kind: NodeKind, r: Rect, color: Color) -> Node {
        let id = self.next;
        self.next += 1;
        Node {
            id,
            name: name.into(),
            frame: r,
            opacity: 1.,
            fill: color,
            radius: 0.,
            shape: document::ShapeKind::Rectangle,
            line_flipped: false,
            appearance: Appearance::default(),
            image_sizing: ImageSizing::Scale,
            image_source: None,
            visible: true,
            locked: false,
            kind,
            is_group: false,
            children: vec![],
        }
    }
    fn text(&mut self, name: &str, content: &str, r: Rect, size: f32, color: Color) -> Node {
        let mut t = text::RichText::new(content);
        t.style.size = size;
        t.style.color = color;
        t.style.line_height = 1.35;
        self.node(name, NodeKind::Text(t), r, Color::rgb(255, 255, 255))
    }
    fn shape(&mut self, name: &str, r: Rect, color: Color) -> Node {
        self.node(name, NodeKind::Shape, r, color)
    }
}
impl Editor {
    /// Insert a reusable layout as ordinary nested nodes in one undo transaction.
    pub fn insert_component(&mut self, component: Component, x: f32, y: f32) -> u64 {
        self.checkpoint();
        let mut b = Builder {
            next: self.document.next_id,
        };
        let ink = Color::rgb(35, 38, 36);
        let muted = Color::rgb(101, 111, 108);
        let blue = Color::rgb(40, 120, 237);
        let clear = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };
        let (w, h) = match component {
            Component::Title => (480., 152.),
            Component::Callout => (460., 122.),
            Component::Quote => (460., 132.),
            Component::Stat => (216., 152.),
            Component::Card => (256., 202.),
            Component::Divider => (480., 34.),
        };
        let mut root = b.node(
            component.name(),
            NodeKind::Frame,
            Rect::new(x, y, w, h),
            clear,
        );
        match component {
            Component::Title => {
                let mut eyebrow = b.text(
                    "Section label",
                    "A NEW PERSPECTIVE",
                    Rect::new(0., 0., 480., 18.),
                    10.,
                    muted,
                );
                if let NodeKind::Text(t) = &mut eyebrow.kind {
                    t.style.letter_spacing = 2.;
                    t.style.weight = 700.;
                }
                let mut title = b.text(
                    "Heading",
                    "Make room for ideas.",
                    Rect::new(0., 30., 480., 60.),
                    38.,
                    ink,
                );
                if let NodeKind::Text(t) = &mut title.kind {
                    t.style.family = "Noto Serif".into();
                }
                root.children=vec![eyebrow,title,b.text("Introduction","A thoughtful beginning for your next chapter.\nWrite something that deserves to be read.",Rect::new(0.,96.,460.,52.),15.,muted)];
            }
            Component::Callout => {
                root.fill = Color::rgb(229, 239, 252);
                root.radius = 12.;
                let mut line = b.shape("Accent", Rect::new(0., 20., 3., 82.), blue);
                line.radius = 1.5;
                let mut heading = b.text(
                    "Heading",
                    "Worth remembering",
                    Rect::new(24., 20., 410., 26.),
                    17.,
                    ink,
                );
                if let NodeKind::Text(t) = &mut heading.kind {
                    t.style.weight = 700.;
                }
                root.children=vec![line,heading,b.text("Body","Give an important idea the attention it deserves.\nEvery word and color here is yours to change.",Rect::new(24.,54.,410.,52.),14.,muted)];
            }
            Component::Quote => {
                root.children.push(b.shape(
                    "Rule",
                    Rect::new(0., 4., 2., 120.),
                    Color::rgb(114, 130, 101),
                ));
                let mut quote = b.text(
                    "Quote",
                    "“The details are not the details.\nThey make the design.”",
                    Rect::new(24., 0., 432., 88.),
                    25.,
                    ink,
                );
                if let NodeKind::Text(t) = &mut quote.kind {
                    t.style.family = "Noto Serif".into();
                    t.style.italic = true;
                }
                root.children.extend([
                    quote,
                    b.text(
                        "Attribution",
                        "— Charles Eames",
                        Rect::new(24., 99., 400., 22.),
                        12.,
                        muted,
                    ),
                ]);
            }
            Component::Stat => {
                root.fill = Color::rgb(233, 237, 225);
                root.radius = 16.;
                let mut number = b.text("Number", "86%", Rect::new(20., 16., 176., 70.), 48., ink);
                if let NodeKind::Text(t) = &mut number.kind {
                    t.style.weight = 700.;
                }
                root.children = vec![
                    number,
                    b.text(
                        "Metric",
                        "A number that matters",
                        Rect::new(20., 91., 180., 24.),
                        14.,
                        ink,
                    ),
                    b.text(
                        "Caption",
                        "Add your context here",
                        Rect::new(20., 119., 180., 18.),
                        11.,
                        muted,
                    ),
                ];
            }
            Component::Card => {
                root.fill = Color::rgb(255, 254, 251);
                root.radius = 16.;
                root.appearance.stroke_width = 1.;
                root.appearance.stroke = Color::rgb(218, 224, 216);
                root.appearance.shadow = true;
                let mut badge = b.shape(
                    "Badge",
                    Rect::new(20., 20., 34., 34.),
                    Color::rgb(224, 235, 251),
                );
                badge.radius = 10.;
                let mut title = b.text(
                    "Heading",
                    "Thoughtfully made",
                    Rect::new(20., 76., 216., 29.),
                    18.,
                    ink,
                );
                if let NodeKind::Text(t) = &mut title.kind {
                    t.style.weight = 700.;
                }
                root.children = vec![
                    badge,
                    b.text(
                        "Badge label",
                        "01",
                        Rect::new(28., 27., 25., 20.),
                        12.,
                        blue,
                    ),
                    title,
                    b.text(
                        "Description",
                        "A simple detail. A useful idea.\nDescribe what makes it work.",
                        Rect::new(20., 115., 216., 64.),
                        14.,
                        muted,
                    ),
                ];
            }
            Component::Divider => {
                root.children = vec![
                    b.shape(
                        "Rule",
                        Rect::new(0., 0., 480., 1.),
                        Color::rgb(214, 220, 211),
                    ),
                    b.text(
                        "Section label",
                        "THE NEXT CHAPTER",
                        Rect::new(0., 13., 400., 18.),
                        10.,
                        muted,
                    ),
                ];
            }
        }
        let id = root.id;
        self.document.next_id = b.next;
        self.document.pages[self.page].nodes.push(root);
        self.select_only(Some(id));
        id
    }
    pub fn is_locked(&self, id: u64) -> bool {
        let mut current = Some(id);
        while let Some(id) = current {
            if self.document.node(id).is_some_and(|n| n.locked) {
                return true;
            }
            current = self.parent_of(id);
        }
        false
    }
    pub fn toggle_visibility(&mut self, id: u64) {
        if self.document.node(id).is_some() {
            self.checkpoint();
            let n = self.document.node_mut(id).unwrap();
            n.visible = !n.visible;
        }
    }
    pub fn toggle_lock(&mut self, id: u64) {
        if self.document.node(id).is_some() {
            self.checkpoint();
            let n = self.document.node_mut(id).unwrap();
            n.locked = !n.locked;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_components_are_valid_editable_and_undoable() {
        let mut e = Editor::default();
        for c in Component::ALL {
            let before = e.document.clone();
            let id = e.insert_component(c, 40., 60.);
            assert!(e.document.validate().is_ok());
            let n = e.document.node(id).unwrap();
            assert!(!n.children.is_empty());
            let child = n
                .children
                .iter()
                .find(|n| matches!(n.kind, NodeKind::Text(_)))
                .unwrap()
                .id;
            if let NodeKind::Text(t) = &mut e.document.node_mut(child).unwrap().kind {
                t.replace(0..0, "Edited ");
            }
            assert!(e.undo());
            assert_eq!(e.document, before);
            assert!(e.redo());
            assert!(e.document.validate().is_ok());
        }
    }
    #[test]
    fn locked_frame_prevents_child_hits() {
        let mut e = Editor::default();
        let root = e.insert_component(Component::Callout, 40., 60.);
        let child = e.document.node(root).unwrap().children[1].id;
        assert!(!e.is_locked(child));
        e.toggle_lock(root);
        assert!(e.is_locked(child));
        assert_eq!(e.hit_test(document::Point { x: 70., y: 90. }), None);
        e.undo();
        assert!(!e.is_locked(child));
        e.toggle_visibility(root);
        assert_eq!(e.hit_test(document::Point { x: 70., y: 90. }), None);
    }
}
