use photography_mind::{config::Config, router::Router, server::PhotoMindServer};
use rmcp::{transport::stdio, ServiceExt};
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
    let router = Router(server);

    // Default stdio transport; HTTP transport can be added later when a public URL is assigned.
    let svc = router.serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}
