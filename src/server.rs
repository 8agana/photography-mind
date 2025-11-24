use crate::config::Config;
use crate::db::{connect_db, healthcheck};
use anyhow::Result;
use rmcp::model::{CallToolRequestParam, CallToolResult};
use surrealdb::{engine::remote::ws::Client, Surreal};

#[derive(Clone)]
pub struct PhotoMindServer {
    pub db: Surreal<Client>,
    pub cfg: Config,
}

impl PhotoMindServer {
    pub async fn new(cfg: Config) -> Result<Self> {
        let db = connect_db(&cfg).await?;
        Ok(Self { db, cfg })
    }

    /// Lightweight health tool: returns DB connectivity + config surface.
    pub async fn handle_health(&self, _req: CallToolRequestParam) -> Result<CallToolResult> {
        let db_ok = healthcheck(&self.db).await.unwrap_or(false);
        let body = serde_json::json!({
            "db": db_ok,
            "namespace": self.cfg.db_namespace,
            "database": self.cfg.db_name,
        });
        Ok(CallToolResult::structured(body))
    }

    /// Simple status tool: counts key tables (best effort, errors become 0).
    pub async fn handle_status(&self, _req: CallToolRequestParam) -> Result<CallToolResult> {
        let tables = [
            ("family", "family"),
            ("skater", "skater"),
            ("competition", "competition"),
            ("event", "event"),
            ("family_competition", "family_competition"),
        ];

        let mut counts = serde_json::Map::new();
        for (key, table) in tables {
            let query = format!("SELECT count() FROM {} GROUP ALL;", table);
            let count = self
                .db
                .query(query)
                .await
                .ok()
                .and_then(|mut res| res.take::<Option<i64>>(0).ok())
                .flatten()
                .unwrap_or(0);
            counts.insert(key.to_string(), serde_json::json!(count));
        }

        Ok(CallToolResult::structured(serde_json::Value::Object(
            counts,
        )))
    }
}
