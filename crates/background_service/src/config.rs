use anyhow::{Context, Result, ensure};
use std::net::{IpAddr, SocketAddr};
pub struct Config {
    pub bind: SocketAddr,
    pub key: String,
    pub concurrency: usize,
    pub minute: u32,
    pub per_ip_day: u32,
    pub global_day: u32,
    pub trusted_proxies: Vec<IpAddr>,
    pub render_proxy: bool,
}
impl Config {
    pub fn from_env() -> Result<Self> {
        let key = if let Ok(path) = std::env::var("COMPOSE_BGCLEAR_API_KEY_FILE") {
            std::fs::read_to_string(path)
                .context("Could not read the configured provider-key file")?
                .trim()
                .to_string()
        } else {
            std::env::var("COMPOSE_BGCLEAR_API_KEY")
                .context("Set COMPOSE_BGCLEAR_API_KEY in the server's secret settings")?
        };
        ensure!(
            image_tools::valid_key(&key),
            "Invalid COMPOSE_BGCLEAR_API_KEY format"
        );
        ensure!(
            key.starts_with("bgc_live_") || key.starts_with("bgc_test_"),
            "COMPOSE_BGCLEAR_API_KEY must be a bgclear key from your provider dashboard"
        );
        let bind = std::env::var("COMPOSE_SERVICE_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8787".into())
            .parse()
            .context("Invalid COMPOSE_SERVICE_BIND")?;
        let trusted_proxies = std::env::var("COMPOSE_TRUSTED_PROXY_IPS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse::<IpAddr>())
            .collect::<Result<Vec<_>, _>>()
            .context("Invalid COMPOSE_TRUSTED_PROXY_IPS")?;
        Ok(Self {
            bind,
            key,
            concurrency: number("COMPOSE_MAX_CONCURRENT", 2, 1, 16)? as usize,
            minute: number("COMPOSE_PER_IP_MINUTE", 3, 1, 120)?,
            per_ip_day: number("COMPOSE_PER_IP_DAY", 30, 1, 10000)?,
            global_day: number("COMPOSE_GLOBAL_DAY", 500, 1, 100000)?,
            trusted_proxies,
            render_proxy: match std::env::var("COMPOSE_PLATFORM_PROXY")
                .unwrap_or_default()
                .as_str()
            {
                "" => false,
                "render" => {
                    ensure!(
                        std::env::var("RENDER").as_deref() == Ok("true"),
                        "Render proxy mode requires Render's runtime environment"
                    );
                    true
                }
                _ => anyhow::bail!("Unsupported COMPOSE_PLATFORM_PROXY"),
            },
        })
    }
}
fn number(name: &str, default: u32, min: u32, max: u32) -> Result<u32> {
    let value = std::env::var(name)
        .ok()
        .map(|v| v.parse::<u32>())
        .transpose()
        .with_context(|| format!("Invalid {name}"))?
        .unwrap_or(default);
    ensure!(
        (min..=max).contains(&value),
        "{name} must be between {min} and {max}"
    );
    Ok(value)
}
