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
        #[derive(serde::Deserialize)]
        struct CountResult {
            count: i64,
        }

        let tables = [
            ("family", "family"),
            ("skater", "skater"),
            ("competition", "competition"),
            ("event", "event"),
            ("family_competition", "family_competition"),
            ("shoot", "shoot"),
            ("family_shoot", "family_shoot"),
        ];

        let mut counts = serde_json::Map::new();
        for (key, table) in tables {
            let query = format!("SELECT count() FROM {} GROUP ALL;", table);
            let count = self
                .db
                .query(query)
                .await
                .ok()
                .and_then(|mut res| res.take::<Vec<CountResult>>(0).ok())
                .and_then(|vec| vec.into_iter().next())
                .map(|r| r.count)
                .unwrap_or(0);
            counts.insert(key.to_string(), serde_json::json!(count));
        }

        Ok(CallToolResult::structured(serde_json::Value::Object(
            counts,
        )))
    }

    /// Find skaters by partial name match (first or last name)
    pub async fn handle_find_skater(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let search_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;

        let query = r#"
            SELECT * FROM skater
            WHERE string::lowercase(first_name ?? '') CONTAINS string::lowercase($search)
            OR string::lowercase(last_name ?? '') CONTAINS string::lowercase($search)
            ORDER BY last_name, first_name
            LIMIT 50;
        "#;

        let mut result = self
            .db
            .query(query)
            .bind(("search", search_name.clone()))
            .await?;

        #[derive(serde::Deserialize, serde::Serialize)]
        struct Skater {
            id: surrealdb::sql::Thing,
            first_name: String,
            last_name: String,
        }

        let skaters: Vec<Skater> = result.take(0)?;

        if skaters.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "found": false,
                "message": format!("No skaters found matching: {}", search_name)
            })));
        }

        let results: Vec<_> = skaters
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.to_string(),
                    "name": format!("{} {}", s.first_name, s.last_name),
                    "first_name": s.first_name,
                    "last_name": s.last_name,
                })
            })
            .collect();

        Ok(CallToolResult::structured(serde_json::json!({
            "found": true,
            "count": skaters.len(),
            "skaters": results,
        })))
    }

    /// Get complete family record including all family members
    pub async fn handle_get_family(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        // Use ID-based lookup like CLI does (family:lastname_lowercase)
        let family_id = format!("family:{}", last_name.to_lowercase().replace(' ', "_"));

        let family_query = "SELECT * FROM type::thing($family_id);";

        let mut family_result = self
            .db
            .query(family_query)
            .bind(("family_id", family_id.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct FamilyRecord {
            id: surrealdb::sql::Thing,
            name: Option<String>,
            last_name: Option<String>,
            delivery_email: Option<String>,
        }

        let families: Vec<FamilyRecord> = family_result.take(0)?;

        if families.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "found": false,
                "message": format!("No family found with last name: {} (ID: {})", last_name, family_id)
            })));
        }

        let family = &families[0];
        let display_name = family
            .last_name
            .clone()
            .or_else(|| family.name.clone())
            .unwrap_or_else(|| last_name.clone());

        // Get all skaters belonging to this family
        let skaters_query = r#"
            SELECT in.id as id, in.first_name as first_name, in.last_name as last_name
            FROM belongs_to
            WHERE out = $family_id
        "#;

        let mut skaters_result = self
            .db
            .query(skaters_query)
            .bind(("family_id", family.id.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct SkaterInfo {
            id: surrealdb::sql::Thing,
            first_name: String,
            last_name: String,
        }

        let skaters: Vec<SkaterInfo> = skaters_result.take(0).unwrap_or_default();

        let skater_list: Vec<_> = skaters
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.to_string(),
                    "name": format!("{} {}", s.first_name, s.last_name),
                    "first_name": s.first_name,
                    "last_name": s.last_name,
                })
            })
            .collect();

        Ok(CallToolResult::structured(serde_json::json!({
            "found": true,
            "family": {
                "id": family.id.to_string(),
                "name": display_name,
                "email": family.delivery_email,
            },
            "skaters": skater_list,
            "skater_count": skaters.len(),
        })))
    }

    /// Mark gallery as sent for a family at a competition
    pub async fn handle_mark_gallery_sent(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        let competition_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("competition_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: competition_name"))?;

        // Use ID-based lookup for family (family:lastname_lowercase)
        let family_id_str = format!("family:{}", last_name.to_lowercase().replace(' ', "_"));
        let family_query = "SELECT VALUE id FROM type::thing($family_id);";
        let mut family_result = self
            .db
            .query(family_query)
            .bind(("family_id", family_id_str.clone()))
            .await?;
        let family_ids: Vec<surrealdb::sql::Thing> = family_result.take(0)?;

        if family_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No family found with last name: {} (ID: {})", last_name, family_id_str)
            })));
        }

        // Find competition
        let comp_query = "SELECT VALUE id FROM competition WHERE string::lowercase(name ?? '') CONTAINS string::lowercase($comp);";
        let mut comp_result = self
            .db
            .query(comp_query)
            .bind(("comp", competition_name.clone()))
            .await?;
        let comp_ids: Vec<surrealdb::sql::Thing> = comp_result.take(0)?;

        if comp_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No competition found matching: {}", competition_name)
            })));
        }

        // Check if edge exists first
        let check_query = r#"
            SELECT id FROM family_competition
            WHERE in = $family_id AND out = $comp_id
            LIMIT 1
        "#;
        let mut check_result = self
            .db
            .query(check_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("comp_id", comp_ids[0].clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct EdgeCheck {
            #[allow(dead_code)]
            id: surrealdb::sql::Thing,
        }
        let edges: Vec<EdgeCheck> = check_result.take(0)?;

        if edges.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No family_competition edge exists for {} at {}. Family may not be linked to this competition.", last_name, competition_name),
                "family_id": family_ids[0].to_string(),
                "competition_id": comp_ids[0].to_string(),
            })));
        }

        // Update family_competition edge
        let update_query = r#"
            UPDATE family_competition
            SET gallery_status = 'sent', sent_date = time::now()
            WHERE in = $family_id AND out = $comp_id
        "#;

        self.db
            .query(update_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("comp_id", comp_ids[0].clone()))
            .await?;

        Ok(CallToolResult::structured(serde_json::json!({
            "success": true,
            "message": format!("Marked gallery as sent for {} at {}", last_name, competition_name),
            "family_id": family_ids[0].to_string(),
            "competition_id": comp_ids[0].to_string(),
        })))
    }

    /// List all families with pending galleries for a competition
    pub async fn handle_list_pending_galleries(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let competition_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("competition_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: competition_name"))?;

        let query = r#"
            SELECT in.last_name as family, in.delivery_email as email, gallery_status
            FROM family_competition
            WHERE string::lowercase(out.name ?? '') CONTAINS string::lowercase($comp)
            AND gallery_status IN ['pending', 'culling', 'processing']
            ORDER BY in.last_name
        "#;

        let mut result = self
            .db
            .query(query)
            .bind(("comp", competition_name.clone()))
            .await?;

        let families: Vec<crate::photography::models::PendingFamily> =
            result.take(0).unwrap_or_default();

        Ok(CallToolResult::structured(serde_json::json!({
            "competition": competition_name,
            "pending_count": families.len(),
            "families": families,
        })))
    }

    /// Get status overview for a competition
    pub async fn handle_competition_status(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let competition_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("competition_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: competition_name"))?;

        // Get counts by gallery_status
        let status_query = r#"
            SELECT gallery_status ?? 'unknown' as gallery_status, count() as count
            FROM family_competition
            WHERE string::lowercase(out.name ?? '') CONTAINS string::lowercase($comp)
            GROUP BY gallery_status
        "#;

        let mut status_result = self
            .db
            .query(status_query)
            .bind(("comp", competition_name.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct StatusCount {
            gallery_status: Option<String>,
            count: i64,
        }

        let status_counts: Vec<StatusCount> = status_result.take(0).unwrap_or_default();

        let mut counts = serde_json::Map::new();
        let mut total = 0i64;

        for sc in status_counts {
            let status = sc.gallery_status.unwrap_or_else(|| "unknown".to_string());
            counts.insert(status, serde_json::json!(sc.count));
            total += sc.count;
        }

        Ok(CallToolResult::structured(serde_json::json!({
            "competition": competition_name,
            "total_families": total,
            "status_breakdown": counts,
        })))
    }

    /// Create a new shoot
    pub async fn handle_create_shoot(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;

        let shoot_type = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_type"))?;

        let location = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("location"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let notes = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("notes"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Optional date parameter (format: YYYY-MM-DD or YYYYMMDD)
        let shoot_date = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("date"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Build query based on whether shoot_date is provided
        let create_query = if shoot_date.is_some() {
            r#"
                CREATE shoot CONTENT {
                    name: $name,
                    shoot_type: $shoot_type,
                    shoot_date: type::datetime($shoot_date),
                    location: $location,
                    notes: $notes
                }
            "#
        } else {
            r#"
                CREATE shoot CONTENT {
                    name: $name,
                    shoot_type: $shoot_type,
                    shoot_date: time::now(),
                    location: $location,
                    notes: $notes
                }
            "#
        };

        let mut result = self
            .db
            .query(create_query)
            .bind(("name", name.clone()))
            .bind(("shoot_type", shoot_type.clone()))
            .bind(("shoot_date", shoot_date.unwrap_or_default()))
            .bind(("location", location))
            .bind(("notes", notes))
            .await?;

        let shoots: Vec<crate::photography::models::Shoot> = result.take(0)?;

        if let Some(shoot) = shoots.first() {
            Ok(CallToolResult::structured(serde_json::json!({
                "success": true,
                "shoot_id": shoot.id.to_string(),
                "name": shoot.name,
                "shoot_type": shoot.shoot_type,
            })))
        } else {
            Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": "Failed to create shoot"
            })))
        }
    }

    /// Mark shoot gallery as sent for a family
    pub async fn handle_mark_shoot_sent(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        let shoot_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_name"))?;

        // Use ID-based lookup for family (family:lastname_lowercase)
        let family_id_str = format!("family:{}", last_name.to_lowercase().replace(' ', "_"));
        let family_query = "SELECT VALUE id FROM type::thing($family_id);";
        let mut family_result = self
            .db
            .query(family_query)
            .bind(("family_id", family_id_str.clone()))
            .await?;
        let family_ids: Vec<surrealdb::sql::Thing> = family_result.take(0)?;

        if family_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No family found with last name: {} (ID: {})", last_name, family_id_str)
            })));
        }

        // Find shoot
        let shoot_query = "SELECT VALUE id FROM shoot WHERE string::lowercase(name ?? '') CONTAINS string::lowercase($shoot);";
        let mut shoot_result = self
            .db
            .query(shoot_query)
            .bind(("shoot", shoot_name.clone()))
            .await?;
        let shoot_ids: Vec<surrealdb::sql::Thing> = shoot_result.take(0)?;

        if shoot_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No shoot found matching: {}", shoot_name)
            })));
        }

        // Check if edge exists first
        let check_query = r#"
            SELECT id FROM family_shoot
            WHERE in = $family_id AND out = $shoot_id
            LIMIT 1
        "#;
        let mut check_result = self
            .db
            .query(check_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("shoot_id", shoot_ids[0].clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct EdgeCheck {
            #[allow(dead_code)]
            id: surrealdb::sql::Thing,
        }
        let edges: Vec<EdgeCheck> = check_result.take(0)?;

        if edges.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No family_shoot edge exists for {} at {}. Family may not be linked to this shoot.", last_name, shoot_name),
                "family_id": family_ids[0].to_string(),
                "shoot_id": shoot_ids[0].to_string(),
            })));
        }

        // Update family_shoot edge
        let update_query = r#"
            UPDATE family_shoot
            SET gallery_status = 'sent', sent_date = time::now()
            WHERE in = $family_id AND out = $shoot_id
        "#;

        self.db
            .query(update_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("shoot_id", shoot_ids[0].clone()))
            .await?;

        Ok(CallToolResult::structured(serde_json::json!({
            "success": true,
            "message": format!("Marked shoot gallery as sent for {} at {}", last_name, shoot_name),
            "family_id": family_ids[0].to_string(),
            "shoot_id": shoot_ids[0].to_string(),
        })))
    }

    /// List all shoots
    pub async fn handle_list_shoots(&self, _req: CallToolRequestParam) -> Result<CallToolResult> {
        let query = "SELECT * FROM shoot ORDER BY shoot_date DESC, name;";

        let mut result = self.db.query(query).await?;
        let shoots: Vec<crate::photography::models::Shoot> = result.take(0)?;

        let shoot_list: Vec<_> = shoots
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.to_string(),
                    "name": s.name,
                    "shoot_type": s.shoot_type,
                    "shoot_date": s.shoot_date,
                    "location": s.location,
                })
            })
            .collect();

        Ok(CallToolResult::structured(serde_json::json!({
            "count": shoots.len(),
            "shoots": shoot_list,
        })))
    }

    /// List all families with pending galleries for a shoot
    pub async fn handle_list_pending_shoot_galleries(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let shoot_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_name"))?;

        let query = r#"
            SELECT in.last_name as family, in.delivery_email as email, gallery_status
            FROM family_shoot
            WHERE string::lowercase(out.name ?? '') CONTAINS string::lowercase($shoot)
            AND gallery_status IN ['pending', 'culling', 'processing']
            ORDER BY in.last_name
        "#;

        let mut result = self
            .db
            .query(query)
            .bind(("shoot", shoot_name.clone()))
            .await?;

        let families: Vec<crate::photography::models::PendingFamily> =
            result.take(0).unwrap_or_default();

        Ok(CallToolResult::structured(serde_json::json!({
            "shoot": shoot_name,
            "pending_count": families.len(),
            "families": families,
        })))
    }

    /// Get status overview for a shoot
    pub async fn handle_shoot_status(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let shoot_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_name"))?;

        // Get counts by gallery_status
        let status_query = r#"
            SELECT gallery_status, count() as count
            FROM family_shoot
            WHERE string::lowercase(out.name ?? '') CONTAINS string::lowercase($shoot)
            GROUP BY gallery_status
        "#;

        let mut status_result = self
            .db
            .query(status_query)
            .bind(("shoot", shoot_name.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct StatusCount {
            gallery_status: String,
            count: i64,
        }

        let status_counts: Vec<StatusCount> = status_result.take(0)?;

        let mut counts = serde_json::Map::new();
        let mut total = 0i64;

        for sc in status_counts {
            counts.insert(sc.gallery_status, serde_json::json!(sc.count));
            total += sc.count;
        }

        // Also get total revenue if any purchases
        let revenue_query = r#"
            SELECT math::sum(purchase_amount) as total_revenue
            FROM family_shoot
            WHERE string::lowercase(out.name ?? '') CONTAINS string::lowercase($shoot)
            AND purchase_amount IS NOT NONE
        "#;

        let mut revenue_result = self
            .db
            .query(revenue_query)
            .bind(("shoot", shoot_name.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct Revenue {
            total_revenue: Option<f64>,
        }

        let revenue: Vec<Revenue> = revenue_result.take(0).unwrap_or_default();
        let total_revenue = revenue.first().and_then(|r| r.total_revenue).unwrap_or(0.0);

        Ok(CallToolResult::structured(serde_json::json!({
            "shoot": shoot_name,
            "total_families": total,
            "status_breakdown": counts,
            "total_revenue": total_revenue,
        })))
    }

    /// Get details about a specific shoot
    pub async fn handle_get_shoot(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let shoot_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_name"))?;

        let query = r#"
            SELECT * FROM shoot
            WHERE string::lowercase(name ?? '') CONTAINS string::lowercase($shoot)
            LIMIT 1
        "#;

        let mut result = self
            .db
            .query(query)
            .bind(("shoot", shoot_name.clone()))
            .await?;

        let shoots: Vec<crate::photography::models::Shoot> = result.take(0)?;

        if shoots.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "found": false,
                "message": format!("No shoot found matching: {}", shoot_name)
            })));
        }

        let shoot = &shoots[0];

        // Get linked families count
        let family_query = r#"
            SELECT count() as count FROM family_shoot WHERE out = $shoot_id GROUP ALL
        "#;

        let mut family_result = self
            .db
            .query(family_query)
            .bind(("shoot_id", shoot.id.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct Count {
            count: i64,
        }

        let counts: Vec<Count> = family_result.take(0).unwrap_or_default();
        let family_count = counts.first().map(|c| c.count).unwrap_or(0);

        Ok(CallToolResult::structured(serde_json::json!({
            "found": true,
            "shoot": {
                "id": shoot.id.to_string(),
                "name": shoot.name,
                "shoot_type": shoot.shoot_type,
                "shoot_date": shoot.shoot_date,
                "location": shoot.location,
                "notes": shoot.notes,
            },
            "family_count": family_count,
        })))
    }

    /// List all families (with optional search)
    pub async fn handle_list_families(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let search = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("search"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let query = if search.is_some() {
            r#"
                SELECT id, name, last_name, delivery_email FROM family
                WHERE string::lowercase(last_name ?? '') CONTAINS string::lowercase($search)
                   OR string::lowercase(name ?? '') CONTAINS string::lowercase($search)
                ORDER BY last_name
                LIMIT 50
            "#
        } else {
            r#"
                SELECT id, name, last_name, delivery_email FROM family
                ORDER BY last_name
                LIMIT 100
            "#
        };

        let mut result = self
            .db
            .query(query)
            .bind(("search", search.clone().unwrap_or_default()))
            .await?;

        #[derive(serde::Deserialize)]
        struct FamilyRow {
            id: surrealdb::sql::Thing,
            name: Option<String>,
            last_name: Option<String>,
            delivery_email: Option<String>,
        }

        let families: Vec<FamilyRow> = result.take(0)?;

        let family_list: Vec<_> = families
            .iter()
            .map(|f| {
                let display_name = f
                    .last_name
                    .clone()
                    .or_else(|| f.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                serde_json::json!({
                    "id": f.id.to_string(),
                    "name": display_name,
                    "email": f.delivery_email,
                })
            })
            .collect();

        Ok(CallToolResult::structured(serde_json::json!({
            "count": families.len(),
            "search": search,
            "families": family_list,
        })))
    }

    /// Create a new family
    pub async fn handle_create_family(&self, req: CallToolRequestParam) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        let email = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("delivery_email"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: delivery_email"))?;

        let notes = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("notes"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Generate ID and name from last name (matching CLI format)
        let family_id = last_name.to_lowercase().replace(' ', "_");
        let family_name = format!("Family {}", last_name);

        // Match CLI format: includes name, first_name, last_name for compatibility
        let create_query = r#"
            INSERT INTO family (id, name, first_name, last_name, delivery_email, notes, created_at)
            VALUES (type::thing('family', $family_id), $name, 'Family', $last_name, $email, $notes, time::now())
            ON DUPLICATE KEY UPDATE delivery_email = $email, notes = $notes
        "#;

        let result = self
            .db
            .query(create_query)
            .bind(("family_id", family_id.clone()))
            .bind(("name", family_name.clone()))
            .bind(("last_name", last_name.clone()))
            .bind(("email", email.clone()))
            .bind(("notes", notes))
            .await?;

        // Check query result
        result.check()?;

        Ok(CallToolResult::structured(serde_json::json!({
            "success": true,
            "family_id": format!("family:{}", family_id),
            "name": family_name,
            "last_name": last_name,
            "email": email,
        })))
    }

    /// Link a family to a shoot (creates family_shoot edge)
    pub async fn handle_link_family_shoot(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        let shoot_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_name"))?;

        // Use ID-based lookup for family (family:lastname_lowercase)
        let family_id_str = format!("family:{}", last_name.to_lowercase().replace(' ', "_"));
        let family_query = "SELECT VALUE id FROM type::thing($family_id);";
        let mut family_result = self
            .db
            .query(family_query)
            .bind(("family_id", family_id_str.clone()))
            .await?;
        let family_ids: Vec<surrealdb::sql::Thing> = family_result.take(0)?;

        if family_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No family found with last name: {} (ID: {})", last_name, family_id_str)
            })));
        }

        // Find shoot
        let shoot_query = "SELECT VALUE id FROM shoot WHERE string::lowercase(name ?? '') CONTAINS string::lowercase($shoot);";
        let mut shoot_result = self
            .db
            .query(shoot_query)
            .bind(("shoot", shoot_name.clone()))
            .await?;
        let shoot_ids: Vec<surrealdb::sql::Thing> = shoot_result.take(0)?;

        if shoot_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No shoot found matching: {}", shoot_name)
            })));
        }

        // Check if edge already exists
        let check_query = r#"
            SELECT id FROM family_shoot
            WHERE in = $family_id AND out = $shoot_id
            LIMIT 1
        "#;
        let mut check_result = self
            .db
            .query(check_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("shoot_id", shoot_ids[0].clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct EdgeCheck {
            id: surrealdb::sql::Thing,
        }
        let existing: Vec<EdgeCheck> = check_result.take(0)?;

        if !existing.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("{} is already linked to shoot {}", last_name, shoot_name),
                "family_id": family_ids[0].to_string(),
                "shoot_id": shoot_ids[0].to_string(),
                "existing_edge_id": existing[0].id.to_string(),
            })));
        }

        // Create family_shoot edge using RELATE
        let relate_query = r#"
            RELATE $family_id->family_shoot->$shoot_id
            SET gallery_status = 'pending', created_at = time::now()
        "#;

        self.db
            .query(relate_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("shoot_id", shoot_ids[0].clone()))
            .await?;

        Ok(CallToolResult::structured(serde_json::json!({
            "success": true,
            "message": format!("Linked {} to shoot {}", last_name, shoot_name),
            "family_id": family_ids[0].to_string(),
            "shoot_id": shoot_ids[0].to_string(),
        })))
    }

    /// Record a purchase for a family at a shoot
    pub async fn handle_record_purchase(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let last_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: last_name"))?;

        let shoot_name = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("shoot_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: shoot_name"))?;

        let amount = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("amount"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: amount"))?;

        // Use ID-based lookup for family (family:lastname_lowercase)
        let family_id_str = format!("family:{}", last_name.to_lowercase().replace(' ', "_"));
        let family_query = "SELECT VALUE id FROM type::thing($family_id);";
        let mut family_result = self
            .db
            .query(family_query)
            .bind(("family_id", family_id_str.clone()))
            .await?;
        let family_ids: Vec<surrealdb::sql::Thing> = family_result.take(0)?;

        if family_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No family found with last name: {} (ID: {})", last_name, family_id_str)
            })));
        }

        // Find shoot
        let shoot_query = "SELECT VALUE id FROM shoot WHERE string::lowercase(name ?? '') CONTAINS string::lowercase($shoot);";
        let mut shoot_result = self
            .db
            .query(shoot_query)
            .bind(("shoot", shoot_name.clone()))
            .await?;
        let shoot_ids: Vec<surrealdb::sql::Thing> = shoot_result.take(0)?;

        if shoot_ids.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "success": false,
                "message": format!("No shoot found matching: {}", shoot_name)
            })));
        }

        // Update family_shoot edge with purchase info
        let update_query = r#"
            UPDATE family_shoot
            SET gallery_status = 'purchased', purchase_amount = $amount, purchase_date = time::now()
            WHERE in = $family_id AND out = $shoot_id
        "#;

        self.db
            .query(update_query)
            .bind(("family_id", family_ids[0].clone()))
            .bind(("shoot_id", shoot_ids[0].clone()))
            .bind(("amount", amount))
            .await?;

        Ok(CallToolResult::structured(serde_json::json!({
            "success": true,
            "message": format!("Recorded ${:.2} purchase for {} at {}", amount, last_name, shoot_name),
        })))
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

        // Use ID-based lookup like CLI does (family:lastname_lowercase)
        let family_id = format!("family:{}", last_name.to_lowercase().replace(' ', "_"));

        let query = "SELECT * FROM type::thing($family_id);";

        let mut result = self
            .db
            .query(query)
            .bind(("family_id", family_id.clone()))
            .await?;

        #[derive(serde::Deserialize)]
        struct FamilyRecord {
            id: surrealdb::sql::Thing,
            name: Option<String>,
            last_name: Option<String>,
            delivery_email: Option<String>,
        }

        let families: Vec<FamilyRecord> = result.take(0)?;

        if families.is_empty() {
            return Ok(CallToolResult::structured(serde_json::json!({
                "found": false,
                "message": format!("No family found with last name: {} (ID: {})", last_name, family_id)
            })));
        }

        let family = &families[0];
        let display_name = family
            .last_name
            .clone()
            .or_else(|| family.name.clone())
            .unwrap_or_else(|| last_name.clone());

        Ok(CallToolResult::structured(serde_json::json!({
            "found": true,
            "family_id": family.id.to_string(),
            "family": display_name,
            "email": family.delivery_email,
        })))
    }

    /// Sync ShootProof galleries - match gallery names to family records
    pub async fn handle_sync_shootproof_galleries(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let json_path = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("json_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: json_path"))?;

        let dry_run = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("dry_run"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Read the JSON file
        let content = tokio::fs::read_to_string(&json_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", json_path, e))?;

        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let galleries = data["galleries"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Expected 'galleries' array in JSON"))?;

        let mut matched = Vec::new();
        let mut unmatched = Vec::new();
        let mut updated = 0;

        for gallery in galleries {
            let sp_id = gallery["id"].as_i64().unwrap_or(0);
            let name = gallery["name"].as_str().unwrap_or("").to_string();
            let url = gallery["url"].as_str().unwrap_or("").to_string();

            // Extract last name from gallery name (e.g., "Addie Knox" -> "knox", "Clements" -> "clements")
            let last_name = name
                .split_whitespace()
                .last()
                .unwrap_or(&name)
                .to_lowercase();
            let family_id_str = format!("family:{}", last_name.replace(' ', "_"));

            // Check if family exists
            let family_query =
                "SELECT id, name, shootproof_gallery_id FROM type::thing($family_id);";
            let mut result = self
                .db
                .query(family_query)
                .bind(("family_id", family_id_str.clone()))
                .await?;

            #[derive(serde::Deserialize)]
            struct FamilyCheck {
                id: surrealdb::sql::Thing,
                _name: Option<String>,
                shootproof_gallery_id: Option<i64>,
            }

            let families: Vec<FamilyCheck> = result.take(0).unwrap_or_default();

            if !families.is_empty() {
                let family = &families[0];
                matched.push(serde_json::json!({
                    "gallery_name": name,
                    "gallery_id": sp_id,
                    "family_id": family.id.to_string(),
                    "family_name": family._name,
                    "existing_sp_id": family.shootproof_gallery_id,
                    "url": url,
                }));

                if !dry_run && family.shootproof_gallery_id.is_none() {
                    // Update family with ShootProof gallery ID
                    let update_query = "UPDATE type::thing($family_id) SET shootproof_gallery_id = $sp_id, shootproof_url = $url;";
                    self.db
                        .query(update_query)
                        .bind(("family_id", family_id_str))
                        .bind(("sp_id", sp_id))
                        .bind(("url", url))
                        .await?;
                    updated += 1;
                }
            } else {
                unmatched.push(serde_json::json!({
                    "gallery_name": name,
                    "gallery_id": sp_id,
                    "attempted_family_id": family_id_str,
                }));
            }
        }

        Ok(CallToolResult::structured(serde_json::json!({
            "dry_run": dry_run,
            "total_galleries": galleries.len(),
            "matched": matched.len(),
            "unmatched": unmatched.len(),
            "updated": updated,
            "matched_details": matched,
            "unmatched_details": unmatched,
        })))
    }

    /// Sync ShootProof orders - update emails and record purchases
    pub async fn handle_sync_shootproof_orders(
        &self,
        req: CallToolRequestParam,
    ) -> Result<CallToolResult> {
        let json_path = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("json_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: json_path"))?;

        let dry_run = req
            .arguments
            .as_ref()
            .and_then(|args| args.get("dry_run"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Read the JSON file
        let content = tokio::fs::read_to_string(&json_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", json_path, e))?;

        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let orders = data["orders"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Expected 'orders' array in JSON"))?;

        let mut emails_updated = 0;
        let mut matched_orders = Vec::new();
        let mut unmatched_orders = Vec::new();

        for order in orders {
            let customer_email = order["customer_email"].as_str().unwrap_or("").to_string();
            let customer_name = order["customer_name"].as_str().unwrap_or("").to_string();
            let event_name = order["event_name"].as_str().unwrap_or("").to_string();
            let grand_total = order["grand_total"].as_f64().unwrap_or(0.0);
            let event_id = order["event_id"].as_i64().unwrap_or(0);

            // Extract last name from event name (gallery name = family name usually)
            let last_name = event_name
                .split_whitespace()
                .last()
                .unwrap_or(&event_name)
                .to_lowercase();
            let family_id_str = format!("family:{}", last_name.replace(' ', "_"));

            // Check if family exists
            let family_query = "SELECT id, name, delivery_email FROM type::thing($family_id);";
            let mut result = self
                .db
                .query(family_query)
                .bind(("family_id", family_id_str.clone()))
                .await?;

            #[derive(serde::Deserialize)]
            struct FamilyCheck {
                id: surrealdb::sql::Thing,
                _name: Option<String>,
                delivery_email: Option<String>,
            }

            let families: Vec<FamilyCheck> = result.take(0).unwrap_or_default();

            if !families.is_empty() {
                let family = &families[0];
                let needs_email = family.delivery_email.is_none() && !customer_email.is_empty();
                let customer_email_clone = customer_email.clone();

                matched_orders.push(serde_json::json!({
                    "event_name": event_name,
                    "event_id": event_id,
                    "customer_name": customer_name,
                    "customer_email": customer_email,
                    "amount": grand_total,
                    "family_id": family.id.to_string(),
                    "existing_email": family.delivery_email,
                    "will_update_email": needs_email,
                }));

                if !dry_run && needs_email {
                    // Update family with email from order
                    let update_query =
                        "UPDATE type::thing($family_id) SET delivery_email = $email;";
                    self.db
                        .query(update_query)
                        .bind(("family_id", family_id_str))
                        .bind(("email", customer_email_clone))
                        .await?;
                    emails_updated += 1;
                }
            } else {
                unmatched_orders.push(serde_json::json!({
                    "event_name": event_name,
                    "customer_name": customer_name,
                    "customer_email": customer_email,
                    "amount": grand_total,
                    "attempted_family_id": family_id_str,
                }));
            }
        }

        Ok(CallToolResult::structured(serde_json::json!({
            "dry_run": dry_run,
            "total_orders": orders.len(),
            "matched": matched_orders.len(),
            "unmatched": unmatched_orders.len(),
            "emails_updated": emails_updated,
            "matched_details": matched_orders,
            "unmatched_details": unmatched_orders,
        })))
    }
}
