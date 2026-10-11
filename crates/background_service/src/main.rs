use anyhow::Result;
#[tokio::main]
async fn main() -> Result<()> {
    let config = compose_background_service::config::Config::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    eprintln!(
        "{}",
        serde_json::json!({"event":"service_started","bind":listener.local_addr()?.to_string(),"output":"preview"})
    );
    axum::serve(
        listener,
        compose_background_service::router(config)
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}
async fn shutdown() {
    let ctrl = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {_=ctrl=>{},_=terminate=>{}}
}
