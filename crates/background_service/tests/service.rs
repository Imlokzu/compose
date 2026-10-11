use compose_background_service::{Processor, config::Config, router_with_processor};
use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
fn png() -> Vec<u8> {
    let mut image = image::RgbaImage::from_pixel(20, 20, image::Rgba([80, 120, 90, 255]));
    image.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
    let mut data = Cursor::new(Vec::new());
    image.write_to(&mut data, image::ImageFormat::Png).unwrap();
    data.into_inner()
}
async fn start(minute: u32, processor: Arc<Processor>) -> (String, tokio::task::JoinHandle<()>) {
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        key: "server_only_test_secret".into(),
        concurrency: 1,
        minute,
        per_ip_day: 100,
        global_day: 500,
        trusted_proxies: vec![],
        render_proxy: false,
    };
    let listener = tokio::net::TcpListener::bind(config.bind).await.unwrap();
    let url = format!(
        "http://{}/v1/background/remove",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            router_with_processor(config, processor)
                .into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    (url, server)
}
#[tokio::test]
async fn public_client_roundtrip_needs_no_key_and_retains_transparency() {
    let output = png();
    let response = output.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let (url, server) = start(
        3,
        Arc::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(response.clone())
        }),
    )
    .await;
    let result = tokio::task::spawn_blocking({
        let output = output.clone();
        move || image_tools::remove_shared(&output, &url)
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result, output);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    server.abort();
}
#[tokio::test]
async fn desktop_never_uploads_to_a_service_with_invalid_health() {
    let uploads = Arc::new(AtomicUsize::new(0));
    let counter = uploads.clone();
    let app = axum::Router::new()
        .route(
            "/health",
            axum::routing::get(|| async { "unrelated service" }),
        )
        .route(
            "/v1/background/remove",
            axum::routing::post(move || async move {
                counter.fetch_add(1, Ordering::SeqCst);
                "unexpected upload"
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/v1/background/remove",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let result = tokio::task::spawn_blocking(move || image_tools::remove_shared(&png(), &url))
        .await
        .unwrap();
    assert!(result.is_err());
    assert_eq!(uploads.load(Ordering::SeqCst), 0);
    server.abort();
}
#[tokio::test]
async fn validates_images_origin_and_public_error_responses() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let (url, server) = start(
        10,
        Arc::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(png())
        }),
    )
    .await;
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "image/png")
        .body("not an image")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    let body = response.text().await.unwrap();
    assert!(body.contains("invalid_image") && !body.contains("server_only_test_secret"));
    let response = client
        .post(&url)
        .header("Content-Type", "image/png")
        .header("Origin", "https://unrelated.example")
        .body(png())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    let response = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body(png())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 415);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let base = url.strip_suffix("/v1/background/remove").unwrap();
    let health = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(health.status(), 200);
    assert_eq!(health.headers()["cache-control"], "no-store");
    server.abort();
}
#[tokio::test]
async fn rate_limit_cannot_be_bypassed_by_spoofed_forwarded_headers() {
    let (url, server) = start(1, Arc::new(|_| Ok(png()))).await;
    let c = reqwest::Client::new();
    for i in 0..2 {
        let response = c
            .post(&url)
            .header("Content-Type", "image/png")
            .header("X-Forwarded-For", format!("203.0.113.{}", i + 1))
            .body(png())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), if i == 0 { 200 } else { 429 });
        if i == 1 {
            assert!(response.headers().contains_key("retry-after"));
        }
    }
    server.abort();
}
#[tokio::test]
async fn rejected_upstream_never_leaks_server_secret() {
    let (url, server) = start(
        3,
        Arc::new(|_| anyhow::bail!("provider returned server_only_test_secret")),
    )
    .await;
    let response = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "image/png")
        .body(png())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 502);
    let text = response.text().await.unwrap();
    assert!(text.contains("unavailable") && !text.contains("server_only_test_secret"));
    server.abort();
}

#[tokio::test]
async fn concurrency_is_bounded_before_another_image_is_processed() {
    let entered = Arc::new(AtomicUsize::new(0));
    let marker = entered.clone();
    let (url, server) = start(
        10,
        Arc::new(move |_| {
            marker.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(200));
            Ok(png())
        }),
    )
    .await;
    let first = tokio::spawn({
        let url = url.clone();
        async move {
            reqwest::Client::new()
                .post(url)
                .header("Content-Type", "image/png")
                .body(png())
                .send()
                .await
                .unwrap()
        }
    });
    for _ in 0..100 {
        if entered.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    assert_eq!(entered.load(Ordering::SeqCst), 1);
    let second = reqwest::Client::new()
        .post(url)
        .header("Content-Type", "image/png")
        .body(png())
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), 503);
    assert!(second.text().await.unwrap().contains("busy"));
    assert_eq!(first.await.unwrap().status(), 200);
    assert_eq!(entered.load(Ordering::SeqCst), 1);
    server.abort();
}
