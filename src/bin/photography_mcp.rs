use axum::{Router as AxumRouter, routing::get};
use photography_mind::{config::Config, router::Router, server::PhotoMindServer};
use rmcp::{
    ServiceExt,
    transport::stdio,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use std::net::SocketAddr;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging (respect RUST_LOG, default warn)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("warn".parse()?))
        .with_ansi(false)
        .init();

    let cfg = Config::load()?;
    let server = PhotoMindServer::new(cfg.clone()).await?;
    let router = Router(server.clone());

    if let Some(http_addr) = cfg.http_addr.clone() {
        let addr: SocketAddr = http_addr.parse()?;
        let session_mgr = std::sync::Arc::new(LocalSessionManager::default());
        let service = StreamableHttpService::new(
            move || Ok(router.clone()),
            session_mgr,
            StreamableHttpServerConfig::default(),
        );

        let app = AxumRouter::new()
            .route("/healthz", get(|| async { "ok" }))
            .nest_service("/mcp", service);

        tracing::info!(%addr, "starting HTTP MCP server");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async {
                let _ = signal::ctrl_c().await;
            })
            .await?;
    } else {
        // Default stdio transport
        let svc = router.serve(stdio()).await?;
        svc.waiting().await?;
    }

    Ok(())
}
