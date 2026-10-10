use crate::{MAX_PIXELS, SearchImage, SearchPage, plain_text};
use anyhow::{Context, Result, bail, ensure};
use reqwest::Url;
use scraper::{Html, Selector};
use serde_json::Value;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

pub fn search_url(query: &str, offset: u64) -> Result<String> {
    let q = query.trim();
    ensure!(!q.is_empty(), "Enter a search term");
    ensure!(
        q.chars().count() <= 200,
        "Keep the search under 200 characters"
    );
    ensure!(offset <= 980, "Search result limit reached");
    let mut url = Url::parse("https://www.bing.com/images/async")?;
    url.query_pairs_mut()
        .extend_pairs([
            ("q", q),
            ("mmasync", "1"),
            ("count", "35"),
            ("adlt", "strict"),
        ])
        .append_pair("first", &(offset + 1).to_string());
    Ok(url.into())
}

/// Parse structured image metadata from the public Bing image results page.
/// Provider markup is data only; it is never rendered or executed by Compose.
pub fn parse_results(bytes: &[u8], offset: u64) -> Result<SearchPage> {
    let html = std::str::from_utf8(bytes).context("Image search returned unreadable text")?;
    let document = Html::parse_fragment(html);
    let cards = Selector::parse("a.iusc[m]").expect("Static selector");
    let mut images = vec![];
    let mut seen = HashSet::new();
    let mut candidates = 0;
    for card in document.select(&cards) {
        candidates += 1;
        let Ok(meta) = serde_json::from_str::<Value>(card.attr("m").unwrap_or_default()) else {
            continue;
        };
        let get = |key| meta.get(key).and_then(Value::as_str).unwrap_or_default();
        let Some(original_url) = crate::download::media_url(get("murl")) else {
            continue;
        };
        let Some(thumbnail_url) = crate::download::media_url(get("turl")) else {
            continue;
        };
        let source_url = get("purl");
        if !crate::safe_external_url(source_url) || !seen.insert(original_url.clone()) {
            continue;
        }
        let ext = Url::parse(&original_url)?
            .path()
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "svg" | "avif" | "heic" | "tif" | "tiff") {
            continue;
        }
        let width = meta
            .get("ow")
            .or_else(|| meta.get("w"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let height = meta
            .get("oh")
            .or_else(|| meta.get("h"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if width > MAX_PIXELS || height > MAX_PIXELS || width.saturating_mul(height) > MAX_PIXELS {
            continue;
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        original_url.hash(&mut hash);
        let source_host = Url::parse(source_url)?
            .host_str()
            .unwrap_or_default()
            .to_string();
        let title: String = plain_text(get("t"))
            .chars()
            .filter(|c| !('\u{e000}'..='\u{f8ff}').contains(c))
            .take(250)
            .collect();
        images.push(SearchImage {
            id: hash.finish().max(1),
            provider: crate::Provider::Web,
            title: if title.is_empty() {
                source_host.clone()
            } else {
                title
            },
            original_url,
            thumbnail_url,
            source_url: source_url.into(),
            author: String::new(),
            license: String::new(),
            license_url: String::new(),
            credit: format!("Source website: {source_host}"),
            width: width as u32,
            height: height as u32,
            bytes: 0,
        });
    }
    if candidates == 0 && !html.contains("dgControl") && !html.trim().is_empty() {
        bail!(
            "Web image search is temporarily unavailable. Try again or open the search in your browser"
        );
    }
    Ok(SearchPage {
        images,
        next: (candidates > 0 && offset < 980).then_some(offset + 35),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn web_query_preserves_all_words_and_paging() {
        let u = Url::parse(&search_url("pixel art cat & logo", 35).unwrap()).unwrap();
        let q: std::collections::HashMap<_, _> = u.query_pairs().collect();
        assert_eq!(q["q"], "pixel art cat & logo");
        assert_eq!(q["first"], "36");
    }
    #[test]
    fn html_entities_metadata_and_duplicates_are_handled() {
        let data = include_bytes!("../tests/fixtures/web.html");
        let p = parse_results(data, 0).unwrap();
        assert_eq!(p.images.len(), 1);
        assert_eq!(p.images[0].title, "Pixel art cat & friends");
        assert_eq!(p.images[0].provider, crate::Provider::Web);
        assert!(p.images[0].license.is_empty());
        assert_eq!(p.next, Some(35));
        assert!(parse_results(b"<html>captcha challenge</html>", 0).is_err());
        assert!(
            parse_results(b"<ul class='dgControl_list'></ul>", 0)
                .unwrap()
                .images
                .is_empty()
        );
    }
}
