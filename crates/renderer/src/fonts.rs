pub struct FontChoice {
    pub family: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub weights: &'static [(f32, &'static str)],
    pub italic: bool,
}
pub const FONT_CHOICES: &[FontChoice] = &[
    FontChoice {
        family: "Lato",
        label: "Lato",
        category: "Humanist sans",
        weights: &[
            (300., "Light"),
            (400., "Regular"),
            (700., "Bold"),
            (900., "Black"),
        ],
        italic: true,
    },
    FontChoice {
        family: "Noto Sans",
        label: "Noto Sans",
        category: "Neutral sans",
        weights: &[(400., "Regular"), (700., "Bold")],
        italic: true,
    },
    FontChoice {
        family: "Noto Serif",
        label: "Noto Serif",
        category: "Editorial serif",
        weights: &[(400., "Regular"), (700., "Bold")],
        italic: true,
    },
    FontChoice {
        family: "Noto Sans Mono",
        label: "Noto Sans Mono",
        category: "Monospace",
        weights: &[(400., "Regular"), (700., "Bold")],
        italic: false,
    },
    FontChoice {
        family: "Anton",
        label: "Anton",
        category: "Condensed display",
        weights: &[(400., "Regular")],
        italic: false,
    },
    FontChoice {
        family: "Bebas Neue",
        label: "Bebas Neue",
        category: "Poster display",
        weights: &[(400., "Regular")],
        italic: false,
    },
];
pub const FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/Lato-Light.ttf"),
    include_bytes!("../../../assets/fonts/Lato-LightItalic.ttf"),
    include_bytes!("../../../assets/fonts/Lato-Regular.ttf"),
    include_bytes!("../../../assets/fonts/Lato-Italic.ttf"),
    include_bytes!("../../../assets/fonts/Lato-Bold.ttf"),
    include_bytes!("../../../assets/fonts/Lato-BoldItalic.ttf"),
    include_bytes!("../../../assets/fonts/Lato-Black.ttf"),
    include_bytes!("../../../assets/fonts/Lato-BlackItalic.ttf"),
    include_bytes!("../../../assets/fonts/NotoSerif-Regular.ttf"),
    include_bytes!("../../../assets/fonts/NotoSerif-Italic.ttf"),
    include_bytes!("../../../assets/fonts/NotoSerif-Bold.ttf"),
    include_bytes!("../../../assets/fonts/NotoSerif-BoldItalic.ttf"),
    include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf"),
    include_bytes!("../../../assets/fonts/NotoSans-Italic.ttf"),
    include_bytes!("../../../assets/fonts/NotoSans-Bold.ttf"),
    include_bytes!("../../../assets/fonts/NotoSans-BoldItalic.ttf"),
    include_bytes!("../../../assets/fonts/NotoSansMono-Regular.ttf"),
    include_bytes!("../../../assets/fonts/NotoSansMono-Bold.ttf"),
    include_bytes!("../../../assets/fonts/Anton-Regular.ttf"),
    include_bytes!("../../../assets/fonts/BebasNeue-Regular.ttf"),
];

/// Keep UI formatting on the same static face in the shaper and glyph painter.
pub fn normalize_style(style: &mut text::TextStyle) {
    if let Some(face) = FONT_CHOICES.iter().find(|f| f.family == style.family) {
        style.weight = face
            .weights
            .iter()
            .min_by(|a, b| {
                (a.0 - style.weight)
                    .abs()
                    .total_cmp(&(b.0 - style.weight).abs())
            })
            .unwrap()
            .0;
        if !face.italic {
            style.italic = false;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_presets_do_not_request_missing_faces() {
        let mut s = text::TextStyle {
            family: "Anton".into(),
            weight: 700.,
            italic: true,
            ..Default::default()
        };
        normalize_style(&mut s);
        assert_eq!(s.weight, 400.);
        assert!(!s.italic);
    }
}
