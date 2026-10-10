use anyhow::{Context, Result, bail, ensure};
use reqwest::{Url, blocking::Client};
use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs},
    time::Duration,
};

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 192 && b == 0)
        || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113))
}
fn public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return public_v4(v4);
    }
    let s = ip.segments();
    // Global unicast only; reject special/documentation/transition address blocks.
    (s[0] & 0xe000) == 0x2000
        && s[0] != 0x2002
        && !(s[0] == 0x2001 && s[1] < 0x0200)
        && !(s[0] == 0x2001 && s[1] == 0x0db8)
}
pub(crate) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => public_v4(v),
        IpAddr::V6(v) => public_v6(v),
    }
}
pub fn external_url(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.port_or_known_default(), Some(80 | 443))
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host
        .trim_matches(['[', ']'])
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if let Ok(ip) = host.parse() {
        return public_ip(ip);
    }
    host.contains('.')
        && ![
            "localhost",
            "local",
            "internal",
            "lan",
            "home",
            "onion",
            "test",
            "invalid",
        ]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}
pub fn media_url(raw: &str) -> Option<String> {
    if !external_url(raw) {
        return None;
    }
    let mut u = Url::parse(raw).ok()?;
    if u.scheme() == "http" {
        u.set_scheme("https").ok()?;
    }
    (u.port_or_known_default() == Some(443)).then(|| u.into())
}

/// Follow bounded public HTTPS redirects, pinning DNS for each connection.
/// Search metadata cannot make the native app fetch a private-network resource.
pub fn image(raw: &str, limit: u64) -> Result<Vec<u8>> {
    let mut url = Url::parse(&media_url(raw).context("Unsupported image address")?)?;
    for _ in 0..=4 {
        let host = url.host_str().context("Missing image host")?;
        let dns_host = host.trim_matches(['[', ']']);
        let addresses: Vec<_> = (dns_host, 443)
            .to_socket_addrs()
            .context("Could not resolve the image website")?
            .collect();
        ensure!(
            !addresses.is_empty() && addresses.iter().all(|a| public_ip(a.ip())),
            "Image website does not resolve to a public address"
        );
        let client = Client::builder()
            .user_agent(super::USER_AGENT)
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve_to_addrs(host, &addresses)
            .build()?;
        let response = client
            .get(url.clone())
            .send()
            .context("Could not download from this website")?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .context("Image redirect has no destination")?
                .to_str()?;
            let next = url.join(location)?;
            url = Url::parse(
                &media_url(next.as_str()).context("Image redirected to an unsupported address")?,
            )?;
            continue;
        }
        let response = response
            .error_for_status()
            .context("Website did not allow downloading this image")?;
        if let Some(length) = response.content_length() {
            ensure!(length <= limit, "Image exceeds the download limit");
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        ensure!(
            super::supported_mime(&mime)
                || matches!(
                    mime.as_str(),
                    "application/octet-stream" | "binary/octet-stream"
                ),
            "Website did not return a supported raster image"
        );
        let mut bytes = vec![];
        response.take(limit + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= limit,
            "Image exceeds the download limit"
        );
        return Ok(bytes);
    }
    bail!("Too many image redirects")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_result_addresses_and_private_dns_are_rejected() {
        for url in [
            "file:///secret",
            "https://127.0.0.1/image",
            "https://[::1]/image",
            "https://10.0.0.1/image",
            "https://host.local/image",
            "https://user:secret@example.com/image",
            "https://example.com:8888/image",
            "https://localhost./image",
        ] {
            assert!(!external_url(url), "{url}");
        }
        for ip in [
            "192.168.1.1",
            "100.64.1.1",
            "169.254.169.254",
            "198.18.0.1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
            "fc00::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()));
        }
        assert!(external_url("https://images.example.org/a.png"));
        assert_eq!(
            media_url("http://images.example.org/a.png").unwrap(),
            "https://images.example.org/a.png"
        );
        assert!(public_ip("8.8.8.8".parse().unwrap()));
    }
}
