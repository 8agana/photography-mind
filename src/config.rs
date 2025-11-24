use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_url: String,
    pub db_namespace: String,
    pub db_name: String,
    pub db_user: String,
    pub db_pass: String,
    pub http_addr: Option<String>,
    pub bearer_token: Option<String>,
    pub allow_token_in_url: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let db_url = env::var("PHOTO_DB_URL").unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
        let db_namespace = env::var("PHOTO_DB_NS").unwrap_or_else(|_| "photography".to_string());
        let db_name = env::var("PHOTO_DB_NAME").unwrap_or_else(|_| "ops".to_string());
        let db_user = env::var("PHOTO_DB_USER").unwrap_or_else(|_| "root".to_string());
        let db_pass = env::var("PHOTO_DB_PASS").unwrap_or_else(|_| "root".to_string());

        // Enable HTTP transport only when explicitly set (e.g., "0.0.0.0:8788")
        let http_addr = env::var("PHOTO_HTTP_ADDR").ok();

        let bearer_token = env::var("PHOTO_BEARER_TOKEN").ok().or_else(|| {
            let home = env::var("HOME").ok()?;
            std::fs::read_to_string(format!("{home}/.surr_token"))
                .ok()
                .map(|s| s.trim().to_string())
        });
        let allow_token_in_url = env::var("PHOTO_ALLOW_TOKEN_IN_URL")
            .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(true);

        Ok(Self {
            db_url,
            db_namespace,
            db_name,
            db_user,
            db_pass,
            http_addr,
            bearer_token,
            allow_token_in_url,
        })
    }
}
