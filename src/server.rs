use crate::config::Config;
use crate::db::{connect_db, healthcheck};
use anyhow::Result;
use rmcp::model::{CallToolRequestParam, CallToolResult};
use surrealdb::{Surreal, engine::remote::ws::Client};

#[derive(Clone)]
pub struct PhotoMindServer {
    pub db: Surreal<Client>,
    pub cfg: Config,
}

impl PhotoMindServer {
    pub async fn new(cfg: Config) -> Result<Self> {
        tracing::info!(db_url = %cfg.db_url, ns = %cfg.db_namespace, db = %cfg.db_name, "connecting db");
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

    /// Get contact info for a family by last name
    pub async fn handle_get_contact(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        let query =
            "SELECT * FROM family WHERE string::lowercase(last_name) = string::lowercase($name);";

        let mut result = self
            .db
            .query(query)
            .bind(("name", last_name.clone()))
            .await?;
        let families: Vec<crate::photography::models::Family> = result.take(0)?;

        if families.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "found": false,
                "message": format!("No family found with last name: {}", last_name)
            })));
        }

        if families.len() == 1 {
            let family = &families[0];
            return Ok(CallToolResult::structured(serde_json::json!({
                "found": true,
                "family": family.last_name,
                "email": family.email,
            })));
        }

        // Multiple families with same last name - return all with disambiguation hint
        let results: Vec<_> = families
            .iter()
            .map(|f| {
                serde_json::json!({
                    "family": f.last_name,
                    "email": f.email,
                })
            })
            .collect();

        Ok(CallToolResult::structured(serde_json::json!({
            "found": true,
            "multiple": true,
            "count": families.len(),
            "families": results,
            "hint": "Multiple families found - use email to disambiguate"
        })))
    }
}
