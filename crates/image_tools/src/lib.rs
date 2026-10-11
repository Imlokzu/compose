//! Portable cloud cutouts. No platform/GPU/model dependency; free previews only.
use anyhow::{Context, Result, ensure};
use reqwest::{
    blocking::{Client, multipart},
    redirect::Policy,
};
use std::{
    io::{Cursor, Read},
    time::Duration,
};

pub const ENDPOINT: &str = "https://www.bgclear.ai/api/v1/remove";
pub const MAX_UPLOAD: usize = 25 * 1024 * 1024;
const MAX_OUTPUT: u64 = 8 * 1024 * 1024;

pub fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 512 && key.bytes().all(|b| b.is_ascii_graphic())
}
pub fn remove(bytes: &[u8], key: &str) -> Result<Vec<u8>> {
    request(ENDPOINT, bytes, key)
}
/// Public relay address only. HTTPS is mandatory, except for local development.
pub fn validate_service_url(value: &str) -> Result<String> {
    let url = reqwest::Url::parse(value).context("Invalid Compose background service URL")?;
    let loopback = url
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1"));
    ensure!(
        url.scheme() == "https" || (url.scheme() == "http" && loopback),
        "Compose background service needs HTTPS (HTTP is allowed only on localhost)"
    );
    ensure!(
        url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "The service URL must not contain credentials, query parameters or fragments"
    );
    Ok(url.to_string())
}
pub fn configured_service_url() -> Result<Option<String>> {
    let value = std::env::var("COMPOSE_BACKGROUND_SERVICE_URL")
        .ok()
        .or_else(|| option_env!("COMPOSE_BACKGROUND_SERVICE_URL").map(str::to_string));
    value
        .filter(|s| !s.trim().is_empty())
        .map(|s| validate_service_url(s.trim()))
        .transpose()
}
pub fn remove_shared(bytes: &[u8], endpoint: &str) -> Result<Vec<u8>> {
    let endpoint = validate_service_url(endpoint)?;
    let (_, w, h) = validate_input(bytes)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(70))
        .redirect(Policy::none())
        .build()?;
    wait_for_service(&client, &endpoint)?;
    let response = client
        .post(endpoint)
        .header("Content-Type", image::guess_format(bytes)?.to_mime_type())
        .header("Accept", "image/png")
        .body(bytes.to_vec())
        .send()
        .context(
            "Could not reach Compose background removal. Check your internet connection and retry.",
        )?;
    let status = response.status();
    if !status.is_success() {
        let mut body = Vec::new();
        response.take(8192).read_to_end(&mut body)?;
        let parsed = serde_json::from_slice::<ServiceErrorBody>(&body).ok();
        let message =
            parsed
                .as_ref()
                .map(|e| e.error.code.message())
                .unwrap_or(match status.as_u16() {
                    413 => ErrorCode::ImageTooLarge.message(),
                    429 => ErrorCode::RateLimited.message(),
                    _ => ErrorCode::Unavailable.message(),
                });
        anyhow::bail!("{message}");
    }
    let mut output = Vec::new();
    response.take(MAX_OUTPUT + 1).read_to_end(&mut output)?;
    validate_output(&output, w, h)?;
    Ok(output)
}
fn wait_for_service(client: &Client, endpoint: &str) -> Result<()> {
    let mut health = reqwest::Url::parse(endpoint)?;
    let render = health
        .host_str()
        .is_some_and(|h| h.ends_with(".onrender.com"));
    health.set_path("/health");
    let budget = if render {
        Duration::from_secs(120)
    } else {
        Duration::from_secs(15)
    };
    let started = std::time::Instant::now();
    loop {
        let remaining = budget.saturating_sub(started.elapsed());
        ensure!(
            !remaining.is_zero(),
            "Background removal is still starting. Please try again shortly."
        );
        let result = client
            .get(health.clone())
            .timeout(remaining.min(Duration::from_secs(15)))
            .send();
        match result {
            Ok(response)=>{
                let status=response.status();let mut body=Vec::new();response.take(4097).read_to_end(&mut body)?;
                let ready=body.len()<=4096&&serde_json::from_slice::<serde_json::Value>(&body).is_ok_and(|v|v["status"]=="ok"&&v["service"]=="compose-background-service");
                if status.is_success()&&ready{return Ok(());}
                if !render{anyhow::bail!("Compose background removal is unavailable. Please try again later.");}
                if status.is_client_error()&&status.as_u16()!=429{anyhow::bail!("Compose background removal could not start. Please try again later.");}
            }
            Err(e) if render&&(e.is_timeout()||e.is_connect())=>{},
            Err(e)=>return Err(e).context("Could not reach Compose background removal. Check your internet connection and retry."),
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidImage,
    ImageTooLarge,
    RateLimited,
    Busy,
    Unavailable,
    Timeout,
    UploadNotAllowed,
}
impl ErrorCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidImage => "Try a supported image with a clear foreground subject.",
            Self::ImageTooLarge => {
                "This image is too large for background removal. Choose a smaller image."
            }
            Self::RateLimited => {
                "Compose's shared background-removal limit has been reached. Please try again later."
            }
            Self::Busy => "Compose background removal is busy. Please try again shortly.",
            Self::Unavailable => {
                "Compose background removal is temporarily unavailable. Your original image is unchanged."
            }
            Self::Timeout => "Background removal took too long. Please try again.",
            Self::UploadNotAllowed => "This upload is not supported by Compose background removal.",
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServiceErrorBody {
    pub error: ServiceError,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServiceError {
    pub code: ErrorCode,
    pub request_id: String,
}
pub fn validate_input(bytes: &[u8]) -> Result<(image::ImageFormat, u32, u32)> {
    ensure!(
        !bytes.is_empty() && bytes.len() <= MAX_UPLOAD,
        "Cloud background removal accepts images up to 25 MB"
    );
    let format = image::guess_format(bytes).context("Unsupported image format")?;
    let (w, h) = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .into_dimensions()?;
    ensure!(
        w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 32_000_000,
        "Image exceeds 32 megapixels"
    );
    Ok((format, w, h))
}
pub fn validate_output(output: &[u8], w: u32, h: u32) -> Result<()> {
    ensure!(
        output.len() as u64 <= MAX_OUTPUT,
        "Cutout response exceeds 8 MB"
    );
    ensure!(
        image::guess_format(output)? == image::ImageFormat::Png,
        "Background removal did not return a PNG"
    );
    let (ow, oh) = image::ImageReader::new(Cursor::new(output))
        .with_guessed_format()?
        .into_dimensions()?;
    ensure!(
        ow > 0 && oh > 0 && ow <= 800 && oh <= 800,
        "Service returned an unexpected preview size"
    );
    ensure!(
        (ow as f64 / oh as f64 - w as f64 / h as f64).abs() < 0.02,
        "Service changed the image proportions"
    );
    let pixels = image::load_from_memory(output)?.into_rgba8();
    ensure!(
        pixels.pixels().any(|p| p.0[3] < 250) && pixels.pixels().any(|p| p.0[3] > 5),
        "No useful transparent foreground was returned. Try a clearer subject."
    );
    Ok(())
}
fn request(endpoint: &str, bytes: &[u8], key: &str) -> Result<Vec<u8>> {
    ensure!(
        valid_key(key),
        "Add a valid bgclear API key in Background removal settings"
    );
    let (format, w, h) = validate_input(bytes)?;
    let mime = format.to_mime_type();
    let form = multipart::Form::new()
        .part(
            "image_file",
            multipart::Part::bytes(bytes.to_vec())
                .file_name(format!("image.{}", format.extensions_str()[0]))
                .mime_str(mime)?,
        )
        .text("size", "preview")
        .text("format", "png")
        .text("crop", "false")
        .text("bg_color", "transparent");
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(55))
        .redirect(Policy::none())
        .build()?;
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .header("Accept", "image/png")
        .multipart(form)
        .send()
        .context("Could not reach bgclear. Check your internet connection and retry.")?;
    let status = response.status();
    match status.as_u16() {
        200 => {}
        401 => anyhow::bail!(
            "The bgclear API key was rejected. Update it in Background removal settings."
        ),
        402 | 403 => anyhow::bail!(
            "bgclear rejected this account or its free preview quota. Check your API dashboard."
        ),
        429 => anyhow::bail!("bgclear rate limit reached. Wait and try again."),
        400 | 413 | 422 => anyhow::bail!(
            "bgclear could not process this image. Try a smaller image with a clear foreground subject."
        ),
        500..=599 => anyhow::bail!(
            "bgclear is temporarily unavailable. Your original image is unchanged; retry later."
        ),
        _ => anyhow::bail!("Background removal returned HTTP {}", status.as_u16()),
    }
    let mut output = Vec::new();
    response.take(MAX_OUTPUT + 1).read_to_end(&mut output)?;
    validate_output(&output, w, h)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    fn png() -> Vec<u8> {
        let mut p = image::RgbaImage::from_pixel(10, 10, image::Rgba([30, 90, 60, 255]));
        p.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
        let mut out = Cursor::new(Vec::new());
        p.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }
    fn server(status: u16, body: Vec<u8>) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            use std::io::Write;
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut request = Vec::new();
            let mut part = [0; 4096];
            loop {
                let n = socket.read(&mut part).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&part[..n]);
                if let Some(end) = request.windows(4).position(|p| p == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let size = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if request.len() >= end + 4 + size {
                        break;
                    }
                }
            }
            write!(socket,"HTTP/1.1 {status} Result\r\nContent-Length: {}\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n",body.len()).unwrap();
            socket.write_all(&body).unwrap();
            String::from_utf8_lossy(&request).into_owned()
        });
        (format!("http://{address}/remove"), thread)
    }
    #[test]
    fn bounded_preview_request_preserves_alpha_and_never_asks_for_paid_size() {
        let input = png();
        let (url, server) = server(200, input.clone());
        assert_eq!(request(&url, &input, "test_key").unwrap(), input);
        let req = server.join().unwrap();
        assert!(
            req.to_lowercase()
                .contains("authorization: bearer test_key")
        );
        assert!(req.contains("name=\"size\"\r\n\r\npreview"));
        assert!(!req.contains("\r\nfull\r\n"));
        assert!(req.contains("name=\"crop\"\r\n\r\nfalse"));
    }
    #[test]
    fn service_failures_have_actionable_messages_and_no_secret() {
        for code in [401, 403, 429, 422, 503] {
            let (url, server) = server(code, Vec::new());
            let error = request(&url, &png(), "private_test_key")
                .unwrap_err()
                .to_string();
            assert!(!error.contains("private_test_key"));
            assert!(!error.is_empty());
            server.join().unwrap();
        }
    }
    #[test]
    fn invalid_key_and_nontransparent_response_rejected() {
        assert!(!valid_key("hello\nsecret"));
        let mut opaque = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 0, 255]))
            .write_to(&mut opaque, image::ImageFormat::Png)
            .unwrap();
        let (url, server) = server(200, opaque.into_inner());
        assert!(request(&url, &png(), "key").is_err());
        server.join().unwrap();
    }
}
