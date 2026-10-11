//! Local contract fixture only; never a production background-removal backend.
use compose_background_service::{config::Config, router_with_processor};
use std::{io::Cursor, sync::Arc};
#[tokio::main]
async fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("Path to a transparent PNG fixture");
    let bind = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:18787".into())
        .parse()
        .unwrap();
    let bytes = std::fs::read(output).unwrap();
    let config = Config {
        bind,
        key: "fixture-only-not-a-provider-key".into(),
        concurrency: 2,
        minute: 120,
        per_ip_day: 1000,
        global_day: 1000,
        trusted_proxies: vec![],
        render_proxy: false,
    };
    let app = router_with_processor(
        config,
        Arc::new(move |input| {
            let (_, w, h) = image_tools::validate_input(input)?;
            let source = image::load_from_memory(&bytes)?.into_rgba8();
            let resized = image::imageops::resize(
                &source,
                w.min(800),
                h.min(800),
                image::imageops::FilterType::Triangle,
            );
            let mut output = Cursor::new(Vec::new());
            resized.write_to(&mut output, image::ImageFormat::Png)?;
            Ok(output.into_inner())
        }),
    );
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    println!("FIXTURE_READY {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
