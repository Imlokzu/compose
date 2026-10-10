use crate::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: u8,
}
impl Hsv {
    pub fn from_color(c: Color) -> Self {
        let (r, g, b) = (c.r as f32 / 255., c.g as f32 / 255., c.b as f32 / 255.);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let hue = if d < 1e-6 {
            0.
        } else if max == r {
            60. * ((g - b) / d).rem_euclid(6.)
        } else if max == g {
            60. * ((b - r) / d + 2.)
        } else {
            60. * ((r - g) / d + 4.)
        };
        Self {
            hue,
            saturation: if max == 0. { 0. } else { d / max },
            value: max,
            alpha: c.a,
        }
    }
    pub fn color(self) -> Color {
        let h = self.hue.rem_euclid(360.) / 60.;
        let v = self.value.clamp(0., 1.);
        let c = v * self.saturation.clamp(0., 1.);
        let x = c * (1. - (h % 2. - 1.).abs());
        let m = v - c;
        let (r, g, b) = match h as u8 {
            0 => (c, x, 0.),
            1 => (x, c, 0.),
            2 => (0., c, x),
            3 => (0., x, c),
            4 => (x, 0., c),
            _ => (c, 0., x),
        };
        Color {
            r: ((r + m) * 255.).round() as u8,
            g: ((g + m) * 255.).round() as u8,
            b: ((b + m) * 255.).round() as u8,
            a: self.alpha,
        }
    }
}
pub fn from_hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 && s.len() != 8 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(if s.len() == 8 {
        Color {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    } else {
        Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    })
}
pub const PALETTE: [Color; 32] = [
    Color::rgb(23, 28, 32),
    Color::rgb(73, 82, 89),
    Color::rgb(130, 139, 145),
    Color::rgb(195, 202, 205),
    Color::rgb(227, 232, 231),
    Color::rgb(245, 246, 242),
    Color::rgb(255, 254, 251),
    Color::rgb(255, 255, 255),
    Color::rgb(120, 42, 55),
    Color::rgb(205, 84, 92),
    Color::rgb(230, 132, 113),
    Color::rgb(229, 162, 66),
    Color::rgb(223, 195, 74),
    Color::rgb(77, 125, 99),
    Color::rgb(76, 156, 149),
    Color::rgb(52, 132, 183),
    Color::rgb(235, 192, 197),
    Color::rgb(247, 219, 220),
    Color::rgb(244, 216, 193),
    Color::rgb(246, 226, 188),
    Color::rgb(246, 240, 193),
    Color::rgb(210, 226, 205),
    Color::rgb(203, 233, 229),
    Color::rgb(201, 225, 241),
    Color::rgb(40, 120, 237),
    Color::rgb(97, 92, 198),
    Color::rgb(152, 105, 191),
    Color::rgb(198, 105, 155),
    Color::rgb(39, 68, 82),
    Color::rgb(66, 89, 66),
    Color::rgb(135, 113, 88),
    Color::rgb(200, 171, 140),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hsv_roundtrip_palette_and_transparency() {
        for c in PALETTE {
            let mut c = c;
            c.a = 91;
            let r = Hsv::from_color(c).color();
            assert!((c.r as i16 - r.r as i16).abs() <= 1);
            assert!((c.g as i16 - r.g as i16).abs() <= 1);
            assert!((c.b as i16 - r.b as i16).abs() <= 1);
            assert_eq!(c.a, r.a);
        }
    }
    #[test]
    fn hex_accepts_rgba_and_rejects_bad_data() {
        assert_eq!(
            from_hex("#2878ED80"),
            Some(Color {
                r: 40,
                g: 120,
                b: 237,
                a: 128
            })
        );
        assert!(from_hex("#xyz123").is_none());
        assert!(from_hex("12345").is_none());
        assert_eq!(from_hex("FFFFFF"), Some(Color::rgb(255, 255, 255)));
    }
}
