//! Native image search: public web results plus an optional Commons catalog.
mod download;
mod web;
use anyhow::{Context, Result, bail, ensure};
use reqwest::{Url, blocking::Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Read, time::Duration};

const API: &str = "https://commons.wikimedia.org/w/api.php";
const USER_AGENT: &str = "Compose/0.6 (native document editor; image search)";
const MAX_JSON: u64 = 2 * 1024 * 1024;
pub const MAX_IMAGE: u64 = 64 * 1024 * 1024;
const MAX_THUMB: u64 = 2 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 32_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[default]
    Web,
    Commons,
}
impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Self::Web => "Bing Images",
            Self::Commons => "Wikimedia Commons",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchImage {
    pub provider: Provider,
    pub id: u64,
    pub title: String,
    pub original_url: String,
    pub thumbnail_url: String,
    pub source_url: String,
    pub author: String,
    pub license: String,
    pub license_url: String,
    pub credit: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}
impl SearchImage {
    pub fn source_site(&self) -> String {
        Url::parse(&self.source_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| "Source website".into())
    }
}
#[derive(Clone, Debug)]
pub struct SearchPage {
    pub images: Vec<SearchImage>,
    pub next: Option<u64>,
}
#[derive(Clone)]
pub struct SearchClient {
    client: Client,
    provider: Provider,
}
impl SearchClient {
    pub fn new() -> Result<Self> {
        Self::with_provider(Provider::Web)
    }
    pub fn with_provider(provider: Provider) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 4 {
                    return attempt.error("Too many image redirects");
                }
                if trusted_url(attempt.url().as_str(), false) {
                    attempt.follow()
                } else {
                    attempt.error("Search redirected to an unsupported website")
                }
            }))
            .build()?;
        Ok(Self { client, provider })
    }
    pub fn search(&self, query: &str, offset: u64) -> Result<SearchPage> {
        let url = provider_search_url(self.provider, query, offset)?;
        let data = self
            .read(&url, MAX_JSON, false)
            .context("Image search failed")?;
        match self.provider {
            Provider::Web => web::parse_results(&data, offset),
            Provider::Commons => parse_page(&data),
        }
    }
    pub fn thumbnail(&self, image: &SearchImage) -> Result<Vec<u8>> {
        download::image(&image.thumbnail_url, MAX_THUMB)
    }
    pub fn original(&self, image: &SearchImage) -> Result<Vec<u8>> {
        ensure!(image.bytes <= MAX_IMAGE, "This original exceeds 64 MB");
        ensure!(
            u64::from(image.width) * u64::from(image.height) <= MAX_PIXELS,
            "This original exceeds 32 megapixels"
        );
        download::image(&image.original_url, MAX_IMAGE)
    }
    fn read(&self, url: &str, limit: u64, media: bool) -> Result<Vec<u8>> {
        ensure!(trusted_url(url, media), "Unsupported image address");
        let response = self
            .client
            .get(url)
            .send()
            .context("Could not connect. Check your internet connection and try again")?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            bail!("Search is receiving too many requests. Wait a moment and retry")
        }
        let response = response
            .error_for_status()
            .context("Search could not serve this request")?;
        ensure!(
            trusted_url(response.url().as_str(), media),
            "Unexpected image host"
        );
        if let Some(n) = response.content_length() {
            ensure!(n <= limit, "Download exceeds the size limit")
        }
        if media {
            let mime = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("");
            ensure!(
                supported_mime(mime),
                "This download is not a supported raster image"
            );
        }
        let mut bytes = Vec::new();
        response
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .context("Download was interrupted. Try again")?;
        ensure!(
            bytes.len() as u64 <= limit,
            "Download exceeds the size limit"
        );
        Ok(bytes)
    }
}
pub fn search_url(query: &str, offset: u64) -> Result<String> {
    web::search_url(query, offset)
}
pub fn browser_search_url(provider: Provider, query: &str) -> Result<String> {
    let mut url = Url::parse(&provider_search_url(provider, query, 0)?)?;
    match provider {
        Provider::Web => {
            url.set_path("/images/search");
            url.set_query(None);
            url.query_pairs_mut().append_pair("q", query.trim());
        }
        Provider::Commons => {
            url.set_path("/w/index.php");
            url.set_query(None);
            url.query_pairs_mut().extend_pairs([
                ("search", query.trim()),
                ("title", "Special:MediaSearch"),
                ("type", "image"),
            ]);
        }
    }
    Ok(url.into())
}
pub fn provider_search_url(provider: Provider, query: &str, offset: u64) -> Result<String> {
    match provider {
        Provider::Web => web::search_url(query, offset),
        Provider::Commons => commons_search_url(query, offset),
    }
}
fn commons_search_url(query: &str, offset: u64) -> Result<String> {
    let q = query.trim();
    ensure!(!q.is_empty(), "Enter a search term");
    ensure!(
        q.chars().count() <= 200,
        "Keep the search under 200 characters"
    );
    ensure!(offset <= 10000, "Search result limit reached");
    let mut url = Url::parse(API)?;
    url.query_pairs_mut().extend_pairs([
        ("action", "query"),
        ("format", "json"),
        ("formatversion", "2"),
        ("generator", "search"),
        ("gsrnamespace", "6"),
        ("gsrlimit", "18"),
        ("gsrwhat", "text"),
        ("gsrsort", "relevance"),
        ("prop", "imageinfo"),
        ("iiprop", "url|size|mime|extmetadata"),
        ("iiurlwidth", "320"),
        ("iiextmetadatalanguage", "en"),
        (
            "iiextmetadatafilter",
            "Artist|Credit|LicenseShortName|LicenseUrl|AttributionRequired",
        ),
    ]);
    url.query_pairs_mut()
        .append_pair("gsrsearch", &format!("{q} filetype:bitmap"))
        .append_pair("gsroffset", &offset.to_string());
    Ok(url.into())
}
fn supported_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/webp" | "image/gif" | "image/bmp" | "image/x-ms-bmp"
    )
}
/// Fixed provider hosts for search responses. Public media uses pinned DNS checks.
pub fn trusted_url(url: &str, media: bool) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
    {
        return false;
    }
    matches!(
        url.host_str(),
        Some("upload.wikimedia.org" | "thumb.wikimedia.org")
    ) || (!media
        && matches!(
            url.host_str(),
            Some("commons.wikimedia.org" | "www.bing.com" | "bing.com")
        ))
}
pub fn safe_external_url(url: &str) -> bool {
    download::external_url(url)
}
fn value(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}
pub fn parse_page(bytes: &[u8]) -> Result<SearchPage> {
    let root: Value =
        serde_json::from_slice(bytes).context("Search returned an unreadable response")?;
    if let Some(error) = root.get("error") {
        bail!("Search unavailable: {}", plain_text(&value(error, "info")))
    }
    let mut pages = root
        .pointer("/query/pages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    pages.sort_by_key(|v| v.get("index").and_then(Value::as_u64).unwrap_or(u64::MAX));
    let mut images = vec![];
    for page in pages {
        let Some(info) = page.pointer("/imageinfo/0") else {
            continue;
        };
        let mime = value(info, "mime");
        if !supported_mime(&mime) {
            continue;
        }
        let width = info.get("width").and_then(Value::as_u64).unwrap_or(0);
        let height = info.get("height").and_then(Value::as_u64).unwrap_or(0);
        let bytes = info.get("size").and_then(Value::as_u64).unwrap_or(u64::MAX);
        if width == 0
            || height == 0
            || width > MAX_PIXELS
            || height > MAX_PIXELS
            || width * height > MAX_PIXELS
            || bytes > MAX_IMAGE
        {
            continue;
        }
        let original_url = value(info, "url");
        let thumbnail_url = value(info, "thumburl");
        let source_url = value(info, "descriptionurl");
        if !trusted_url(&original_url, true)
            || !trusted_url(&thumbnail_url, true)
            || !safe_external_url(&source_url)
        {
            continue;
        }
        let meta = |name: &str| {
            plain_text(
                info.pointer(&format!("/extmetadata/{name}/value"))
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
        };
        let id = page.get("pageid").and_then(Value::as_u64).unwrap_or(0);
        if id == 0 {
            continue;
        }
        let title = value(&page, "title");
        images.push(SearchImage {
            provider: Provider::Commons,
            id,
            title: plain_text(title.strip_prefix("File:").unwrap_or(&title))
                .chars()
                .take(250)
                .collect(),
            original_url,
            thumbnail_url,
            source_url,
            author: meta("Artist"),
            license: meta("LicenseShortName"),
            license_url: meta("LicenseUrl"),
            credit: meta("Credit"),
            width: width as u32,
            height: height as u32,
            bytes,
        });
    }
    let next = root
        .pointer("/continue/gsroffset")
        .and_then(Value::as_u64)
        .filter(|n| *n <= 10000);
    Ok(SearchPage { images, next })
}
/// Metadata is rendered as plain text; never interpret provider HTML in the UI.
pub fn plain_text(html: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => {
                tag = true;
                out.push(' ')
            }
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out = out
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(600)
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_is_encoded_not_interpolated() {
        let url = Url::parse(&commons_search_url("snow & ice #2", 18).unwrap()).unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(pairs["gsrsearch"], "snow & ice #2 filetype:bitmap");
        assert_eq!(pairs["gsroffset"], "18");
        assert!(search_url(" ", 0).is_err());
    }
    #[test]
    fn untrusted_addresses_are_rejected() {
        for s in [
            "http://upload.wikimedia.org/a.jpg",
            "https://localhost/a",
            "https://127.0.0.1/a",
            "https://upload.wikimedia.org.evil.test/a",
            "https://user@upload.wikimedia.org/a",
            "https://upload.wikimedia.org:444/a",
        ] {
            assert!(!trusted_url(s, true), "{s}")
        }
        assert!(trusted_url("https://thumb.wikimedia.org/a.jpg", true));
        assert!(!trusted_url("https://commons.wikimedia.org/a.jpg", true));
    }
    #[test]
    fn provider_markup_becomes_plain_text() {
        assert_eq!(
            plain_text("<a href='x'>A &amp; B</a><br />Photo"),
            "A & B Photo"
        );
    }
    #[test]
    fn empty_errors_and_corruption_are_distinct() {
        assert!(
            parse_page(br#"{"query":{"pages":[]}}"#)
                .unwrap()
                .images
                .is_empty()
        );
        assert!(parse_page(br#"{"error":{"info":"Try later"}}"#).is_err());
        assert!(parse_page(b"not json").is_err());
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    #[test]
    fn real_metadata_and_paging_are_preserved() {
        let page = parse_page(include_bytes!("../tests/fixtures/search.json")).unwrap();
        assert_eq!(page.next, Some(18));
        assert_eq!(page.images.len(), 1);
        let i = &page.images[0];
        assert_eq!(i.title, "Felis catus-cat on snow.jpg");
        assert_eq!(i.author, "Von.grzanka");
        assert_eq!(i.license, "CC BY-SA 3.0");
        assert_eq!(i.width, 3000);
        assert!(
            i.source_url
                .starts_with("https://commons.wikimedia.org/wiki/")
        );
    }
    #[test]
    fn oversized_and_untrusted_results_never_reach_ui() {
        let mut v: Value =
            serde_json::from_slice(include_bytes!("../tests/fixtures/search.json")).unwrap();
        v["query"]["pages"][0]["imageinfo"][0]["url"] =
            Value::String("https://127.0.0.1/secrets".into());
        assert!(
            parse_page(&serde_json::to_vec(&v).unwrap())
                .unwrap()
                .images
                .is_empty()
        );
        v["query"]["pages"][0]["imageinfo"][0]["url"] =
            Value::String("https://upload.wikimedia.org/image.jpg".into());
        v["query"]["pages"][0]["imageinfo"][0]["width"] = Value::from(100000);
        assert!(
            parse_page(&serde_json::to_vec(&v).unwrap())
                .unwrap()
                .images
                .is_empty()
        );
    }
}
