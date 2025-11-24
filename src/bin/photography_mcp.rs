use axum::{
    Json, Router as AxumRouter,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware,
    response::IntoResponse,
    response::Response,
    routing::get,
};
use photography_mind::{config::Config, router::Router, server::PhotoMindServer};
use rmcp::{
    ServiceExt,
    transport::stdio,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde_json::json;
use std::net::SocketAddr;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AuthState {
    token: Option<String>,
    allow_query: bool,
}

async fn auth_layer(
    State(state): State<AuthState>,
    req: Request<Body>,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    // Allow open healthz
    if req.uri().path().starts_with("/healthz") {
        return Ok(next.run(req).await);
    }

    // If no token configured, allow all
    let Some(expected) = state.token else {
        return Ok(next.run(req).await);
    };

    let headers: &axum::http::HeaderMap = req.headers();
    let header_ok = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|v| v == format!("Bearer {expected}"))
        .unwrap_or(false);

    let mut query_ok = false;
    if !header_ok
        && state.allow_query
        && let Some(q) = req.uri().query()
    {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=')
                && (k == "access_token" || k == "token")
                && v == expected
            {
                query_ok = true;
                break;
            }
        }
    }

    if header_ok || query_ok {
        Ok(next.run(req).await)
    } else {
        let body = json!({
            "error": "invalid_token",
            "error_description": "Unauthorized"
        });
        Ok((StatusCode::UNAUTHORIZED, Json(body)).into_response())
    }
}

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

    tracing::info!(http_addr=?cfg.http_addr, "config loaded");

    if let Some(http_addr) = cfg.http_addr.clone() {
        let addr: SocketAddr = http_addr.parse()?;
        let session_mgr = std::sync::Arc::new(LocalSessionManager::default());
        let service = StreamableHttpService::new(
            move || Ok(router.clone()),
            session_mgr,
            StreamableHttpServerConfig::default(),
        );
        let auth_state = AuthState {
            token: cfg.bearer_token.clone(),
            allow_query: cfg.allow_token_in_url,
        };

        let app = AxumRouter::new()
            .route("/healthz", get(|| async { "ok" }))
            .nest_service("/mcp", service)
            .layer(middleware::from_fn_with_state(auth_state, auth_layer));

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
