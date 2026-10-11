use crate::RichText;
use std::ops::Range;
pub fn normalize(value: &str) -> Result<String, &'static str> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 2048
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err("Use a web address or mailto: email link without spaces");
    }
    let candidate = if value.contains(':') {
        value.to_string()
    } else if value.contains('@') {
        format!("mailto:{value}")
    } else {
        format!("https://{value}")
    };
    let url = url::Url::parse(&candidate)
        .map_err(|_| "Use a valid https:// web address or mailto: email link")?;
    let okay = match url.scheme() {
        "https" | "http" => {
            url.host_str().is_some() && url.username().is_empty() && url.password().is_none()
        }
        "mailto" => url.path().contains('@') && !url.path().starts_with('@'),
        _ => false,
    };
    if okay {
        Ok(url.to_string())
    } else {
        Err("Links support http, https and mailto addresses")
    }
}
pub fn valid(value: &str) -> bool {
    normalize(value).is_ok_and(|normalized| normalized == value)
}
impl RichText {
    pub fn link_range(&self, index: usize) -> Option<(Range<usize>, String)> {
        let index = crate::floor_boundary(&self.content, index);
        let link = self.style_at(index).link?;
        let mut boundaries = vec![0, self.content.len()];
        for span in &self.spans {
            boundaries.extend([span.range.start, span.range.end]);
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        let mut run = None::<Range<usize>>;
        for b in boundaries.windows(2) {
            if self.style_at(b[0]).link.as_deref() == Some(&link) {
                if let Some(r) = &mut run {
                    r.end = b[1];
                } else {
                    run = Some(b[0]..b[1]);
                }
            } else if let Some(r) = run.take()
                && r.contains(&index)
            {
                return Some((r, link));
            }
        }
        run.filter(|r| r.contains(&index)).map(|r| (r, link))
    }
    pub fn apply_link(&mut self, range: Range<usize>, url: Option<String>) {
        let range = self.range(range);
        if range.is_empty() {
            return;
        }
        let spans = self.segments(range.clone(), range.start);
        for span in spans {
            let mut style = span.style;
            style.link = url.clone();
            if url.is_some() {
                style.underline = true;
                style.color = crate::Color::rgb(40, 120, 237);
            }
            self.apply_style(span.range, style);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_survive_style_edits_text_edits_and_clipboard() {
        let mut t = RichText::new("Read this page please");
        let mut bold = t.style.clone();
        bold.weight = 700.;
        t.apply_style(5..9, bold);
        let url = normalize("example.com/docs").unwrap();
        t.apply_link(5..14, Some(url.clone()));
        assert_eq!(t.style_at(5).weight, 700.);
        assert_eq!(t.link_range(10), Some((5..14, url.clone())));
        let copied = t.slice(5..14);
        assert_eq!(copied.style_at(0).link, Some(url.clone()));
        t.replace(0..0, "Now ");
        assert_eq!(t.link_range(14), Some((9..18, url.clone())));
        t.apply_link(9..18, None);
        assert!(t.style_at(10).link.is_none());
        assert!(t.validate());
    }
    #[test]
    fn safe_schemes_and_legacy_styles() {
        assert_eq!(normalize("example.com").unwrap(), "https://example.com/");
        assert_eq!(
            normalize("hello@example.com").unwrap(),
            "mailto:hello@example.com"
        );
        for url in [
            "javascript:alert(1)",
            "file:///private/key",
            "data:text/html,hi",
            "https://user:pass@example.com",
            "https://example.com\nhi",
        ] {
            assert!(normalize(url).is_err());
        }
        let mut value = serde_json::to_value(crate::TextStyle::default()).unwrap();
        value.as_object_mut().unwrap().remove("link");
        let style: crate::TextStyle = serde_json::from_value(value).unwrap();
        assert!(style.link.is_none());
    }
}
