pub mod config;
mod limits;
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{ConnectInfo, DefaultBodyLimit, Request, State, rejection::BytesRejection},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use config::Config;
use image_tools::{ErrorCode, ServiceError, ServiceErrorBody};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
pub type Processor = dyn Fn(&[u8]) -> anyhow::Result<Vec<u8>> + Send + Sync;
#[derive(Clone)]
struct ServiceState {
    config: Arc<Config>,
    limits: Arc<Mutex<limits::Limits>>,
    slots: Arc<Semaphore>,
    processor: Arc<Processor>,
}
#[derive(Clone)]
struct RequestId(String);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn failure(code: ErrorCode, status: StatusCode, id: &str) -> Response {
    let mut response = (
        status,
        Json(ServiceErrorBody {
            error: ServiceError {
                code,
                request_id: id.into(),
            },
        }),
    )
        .into_response();
    if matches!(code, ErrorCode::RateLimited | ErrorCode::Busy) {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "60".parse().unwrap());
    }
    response
}
pub fn router(config: Config) -> Router {
    let key = config.key.clone();
    router_with_processor(
        config,
        Arc::new(move |bytes| image_tools::remove(bytes, &key)),
    )
}
/// Dependency injection keeps integration tests off real user/provider credentials.
pub fn router_with_processor(config: Config, processor: Arc<Processor>) -> Router {
    let state = ServiceState {
        limits: Arc::new(Mutex::new(limits::Limits::new(
            config.minute,
            config.per_ip_day,
            config.global_day,
        ))),
        slots: Arc::new(Semaphore::new(config.concurrency)),
        config: Arc::new(config),
        processor,
    };
    let images = Router::new()
        .route("/v1/background/remove", post(remove))
        .route_layer(middleware::from_fn_with_state(state.clone(), guard));
    Router::new()
        .route(
            "/health",
            get(|| async {
                Json(serde_json::json!({"status":"ok","service":"compose-background-service"}))
            }),
        )
        .route(
            "/ready",
            get(|| async { Json(serde_json::json!({"status":"ready","output":"preview"})) }),
        )
        .merge(images)
        .layer(DefaultBodyLimit::max(image_tools::MAX_UPLOAD))
        .layer(middleware::from_fn(observe))
        .with_state(state)
}
async fn observe(mut request: Request, next: Next) -> Response {
    let id = format!(
        "bg-{}-{}",
        seconds(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    );
    let is_image = request.uri().path() == "/v1/background/remove";
    request.extensions_mut().insert(RequestId(id.clone()));
    let start = Instant::now();
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("x-request-id", id.parse().unwrap());
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("x-content-type-options", "nosniff".parse().unwrap());
    if is_image {
        eprintln!(
            "{}",
            serde_json::json!({"event":"background_request","request_id":id,"status":response.status().as_u16(),"elapsed_ms":start.elapsed().as_millis()})
        );
    }
    response
}
fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpAddr], render_proxy: bool) -> IpAddr {
    let private = match peer {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_unique_local() || ip.is_loopback(),
    };
    // Render's public edge replaces XFF's first entry with the real client IP.
    if render_proxy && private {
        return headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(peer);
    }
    if trusted.contains(&peer) {
        if let Some(ip) = headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
        {
            return ip;
        }
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next_back())
            .and_then(|v| v.trim().parse().ok())
        {
            return ip;
        }
    }
    peer
}
async fn guard(State(state): State<ServiceState>, mut request: Request, next: Next) -> Response {
    let id = request.extensions().get::<RequestId>().unwrap().0.clone();
    if request.headers().contains_key(header::ORIGIN) {
        return failure(ErrorCode::UploadNotAllowed, StatusCode::FORBIDDEN, &id);
    }
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|s| s > image_tools::MAX_UPLOAD as u64)
    {
        return failure(ErrorCode::ImageTooLarge, StatusCode::PAYLOAD_TOO_LARGE, &id);
    }
    if !request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|s| s.to_str().ok())
        .is_some_and(|s| s.starts_with("image/"))
    {
        return failure(
            ErrorCode::UploadNotAllowed,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            &id,
        );
    }
    let Some(peer) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|p| p.0.ip())
    else {
        return failure(ErrorCode::Unavailable, StatusCode::SERVICE_UNAVAILABLE, &id);
    };
    let ip = client_ip(
        peer,
        request.headers(),
        &state.config.trusted_proxies,
        state.config.render_proxy,
    );
    if !state.limits.lock().unwrap().admit(ip, seconds()) {
        return failure(ErrorCode::RateLimited, StatusCode::TOO_MANY_REQUESTS, &id);
    }
    let Ok(permit) = state.slots.clone().try_acquire_owned() else {
        return failure(ErrorCode::Busy, StatusCode::SERVICE_UNAVAILABLE, &id);
    };
    request.extensions_mut().insert(Arc::new(permit));
    match tokio::time::timeout(Duration::from_secs(65), next.run(request)).await {
        Ok(response) => response,
        Err(_) => failure(ErrorCode::Timeout, StatusCode::GATEWAY_TIMEOUT, &id),
    }
}
async fn remove(
    State(state): State<ServiceState>,
    Extension(id): Extension<RequestId>,
    Extension(permit): Extension<Arc<OwnedSemaphorePermit>>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let body = match body {
        Ok(body) => body,
        Err(e) => {
            return failure(
                if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    ErrorCode::ImageTooLarge
                } else {
                    ErrorCode::InvalidImage
                },
                e.status(),
                &id.0,
            );
        }
    };
    // The body and shared permit stay in the worker even if the client disconnects.
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let (_, w, h) = image_tools::validate_input(&body).map_err(|_| ErrorCode::InvalidImage)?;
        let pixels = (state.processor)(&body).map_err(|_| ErrorCode::Unavailable)?;
        image_tools::validate_output(&pixels, w, h).map_err(|_| ErrorCode::Unavailable)?;
        Ok::<_, ErrorCode>(pixels)
    })
    .await;
    match result {
        Ok(Ok(pixels)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "image/png")],
            Body::from(pixels),
        )
            .into_response(),
        Ok(Err(ErrorCode::InvalidImage)) => failure(
            ErrorCode::InvalidImage,
            StatusCode::UNPROCESSABLE_ENTITY,
            &id.0,
        ),
        _ => failure(ErrorCode::Unavailable, StatusCode::BAD_GATEWAY, &id.0),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicitly_trusted_proxies_control_rate_limit_identity() {
        let peer = "127.0.0.1".parse().unwrap();
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "203.0.113.25".parse().unwrap());
        assert_eq!(client_ip(peer, &h, &[], false), peer);
        assert_eq!(
            client_ip(peer, &h, &[peer], false),
            "203.0.113.25".parse::<IpAddr>().unwrap()
        );
        h.insert("cf-connecting-ip", "198.51.100.24".parse().unwrap());
        assert_eq!(
            client_ip(peer, &h, &[peer], false),
            "198.51.100.24".parse::<IpAddr>().unwrap()
        );
    }
    #[test]
    fn render_proxy_uses_the_platform_client_ip_without_trusting_public_peers() {
        let proxy = "10.0.0.2".parse().unwrap();
        let public = "198.51.100.10".parse().unwrap();
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "203.0.113.25, 10.0.0.1".parse().unwrap());
        h.insert("cf-connecting-ip", "203.0.113.99".parse().unwrap());
        assert_eq!(
            client_ip(proxy, &h, &[], true),
            "203.0.113.25".parse::<IpAddr>().unwrap()
        );
        assert_eq!(client_ip(public, &h, &[], true), public);
        assert_eq!(client_ip(proxy, &h, &[], false), proxy);
    }
}
