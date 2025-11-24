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

        Ok(Self {
            db_url,
            db_namespace,
            db_name,
            db_user,
            db_pass,
            http_addr,
        })
    }
}
