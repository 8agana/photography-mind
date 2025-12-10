use crate::server::PhotoMindServer;
use rmcp::{
    ErrorData as McpError,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Implementation, InitializeRequestParam,
        InitializeResult, ListToolsResult, PaginatedRequestParam, ProtocolVersion,
        ServerCapabilities, ServerInfo, Tool, ToolsCapability,
    },
    service::RequestContext,
};

#[derive(Clone)]
pub struct Router(pub PhotoMindServer);

impl ServerHandler for Router {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "photography-mind".to_string(),
                title: Some("Photography Mind".to_string()),
                version: "0.1.0".to_string(),
                website_url: Some("https://github.com/8agana/photography-mind".to_string()),
                icons: None,
            },
            ..Default::default()
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> std::result::Result<InitializeResult, McpError> {
        let mut info = self.get_info();
        info.protocol_version = request.protocol_version.clone();
        Ok(info)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> std::result::Result<ListToolsResult, McpError> {
        // Helper to create schema Arc from JSON
        fn schema(
            json: serde_json::Value,
        ) -> std::sync::Arc<serde_json::Map<String, serde_json::Value>> {
            std::sync::Arc::new(json.as_object().cloned().unwrap_or_default())
        }

        // Empty schema for tools with no parameters
        let empty_schema = schema(serde_json::json!({ "type": "object" }));

        // Schema for last_name parameter
        let last_name_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "string",
                    "description": "Family last name to search for"
                }
            },
            "required": ["last_name"]
        }));

        // Schema for name parameter (skater search)
        let name_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name to search for (first or last)"
                }
            },
            "required": ["name"]
        }));

        // Schema for competition_name parameter
        let competition_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "competition_name": {
                    "type": "string",
                    "description": "Competition name to query"
                }
            },
            "required": ["competition_name"]
        }));

        // Schema for shoot_name parameter
        let shoot_name_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "shoot_name": {
                    "type": "string",
                    "description": "Shoot name to query"
                }
            },
            "required": ["shoot_name"]
        }));

        // Schema for mark_gallery_sent (last_name + competition_name)
        let mark_gallery_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "string",
                    "description": "Family last name"
                },
                "competition_name": {
                    "type": "string",
                    "description": "Competition name"
                }
            },
            "required": ["last_name", "competition_name"]
        }));

        // Schema for mark_shoot_sent (last_name + shoot_name)
        let mark_shoot_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "string",
                    "description": "Family last name"
                },
                "shoot_name": {
                    "type": "string",
                    "description": "Shoot name"
                }
            },
            "required": ["last_name", "shoot_name"]
        }));

        // Schema for create_shoot
        let create_shoot_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Shoot name"
                },
                "shoot_type": {
                    "type": "string",
                    "description": "Type of shoot (portrait, commercial, event, etc.)"
                },
                "date": {
                    "type": "string",
                    "description": "Shoot date (YYYY-MM-DD format, optional)"
                },
                "location": {
                    "type": "string",
                    "description": "Shoot location (optional)"
                }
            },
            "required": ["name", "shoot_type"]
        }));

        // Schema for create_family
        let create_family_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "string",
                    "description": "Family last name"
                },
                "delivery_email": {
                    "type": "string",
                    "description": "Email for gallery delivery"
                },
                "phone": {
                    "type": "string",
                    "description": "Phone number (optional)"
                }
            },
            "required": ["last_name", "delivery_email"]
        }));

        // Schema for link_family_shoot
        let link_family_shoot_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "string",
                    "description": "Family last name"
                },
                "shoot_name": {
                    "type": "string",
                    "description": "Shoot name to link"
                }
            },
            "required": ["last_name", "shoot_name"]
        }));

        // Schema for record_purchase
        let record_purchase_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "last_name": {
                    "type": "string",
                    "description": "Family last name"
                },
                "amount": {
                    "type": "number",
                    "description": "Purchase amount in dollars"
                },
                "shoot_name": {
                    "type": "string",
                    "description": "Shoot name"
                }
            },
            "required": ["last_name", "amount", "shoot_name"]
        }));

        // Schema for list_families (optional search)
        let list_families_schema = schema(serde_json::json!({
            "type": "object",
            "properties": {
                "search": {
                    "type": "string",
                    "description": "Optional search term to filter families"
                }
            }
        }));

        let tools = vec![
            Tool {
                name: "health".into(),
                title: Some("Health".into()),
                description: Some("Check SurrealDB connectivity and config surface".into()),
                input_schema: empty_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "status".into(),
                title: Some("Status".into()),
                description: Some("Counts key photography tables".into()),
                input_schema: empty_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "get_contact".into(),
                title: Some("Get Contact".into()),
                description: Some("Get email and phone for a family by last name".into()),
                input_schema: last_name_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "find_skater".into(),
                title: Some("Find Skater".into()),
                description: Some(
                    "Search for skaters by partial name match (first or last)".into(),
                ),
                input_schema: name_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "get_family".into(),
                title: Some("Get Family".into()),
                description: Some("Get complete family record including all family members".into()),
                input_schema: last_name_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "mark_gallery_sent".into(),
                title: Some("Mark Gallery Sent".into()),
                description: Some("Mark gallery as sent for a family at a competition".into()),
                input_schema: mark_gallery_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "list_pending_galleries".into(),
                title: Some("List Pending Galleries".into()),
                description: Some(
                    "List all families with pending galleries for a competition".into(),
                ),
                input_schema: competition_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "competition_status".into(),
                title: Some("Competition Status".into()),
                description: Some("Get status overview and counts for a competition".into()),
                input_schema: competition_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "create_shoot".into(),
                title: Some("Create Shoot".into()),
                description: Some("Create a new shoot (portrait, commercial, event, etc.)".into()),
                input_schema: create_shoot_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "mark_shoot_sent".into(),
                title: Some("Mark Shoot Sent".into()),
                description: Some("Mark shoot gallery as sent for a family".into()),
                input_schema: mark_shoot_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "list_shoots".into(),
                title: Some("List Shoots".into()),
                description: Some("List all shoots".into()),
                input_schema: empty_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "create_family".into(),
                title: Some("Create Family".into()),
                description: Some("Create a new family/client in the database".into()),
                input_schema: create_family_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "link_family_shoot".into(),
                title: Some("Link Family to Shoot".into()),
                description: Some("Connect a family to a shoot (creates family_shoot relationship)".into()),
                input_schema: link_family_shoot_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "record_purchase".into(),
                title: Some("Record Purchase".into()),
                description: Some("Record a purchase amount for a family at a shoot".into()),
                input_schema: record_purchase_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "list_pending_shoot_galleries".into(),
                title: Some("List Pending Shoot Galleries".into()),
                description: Some("List all families with pending galleries for a shoot".into()),
                input_schema: shoot_name_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "shoot_status".into(),
                title: Some("Shoot Status".into()),
                description: Some("Get status overview and revenue for a shoot".into()),
                input_schema: shoot_name_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "get_shoot".into(),
                title: Some("Get Shoot".into()),
                description: Some("Get details about a specific shoot".into()),
                input_schema: shoot_name_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "list_families".into(),
                title: Some("List Families".into()),
                description: Some("List all families/clients (with optional search)".into()),
                input_schema: list_families_schema,
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            // ShootProof sync tools
            Tool {
                name: "sync_shootproof_galleries".into(),
                title: Some("Sync ShootProof Galleries".into()),
                description: Some("Import galleries from ShootProof export JSON, matching to families by name".into()),
                input_schema: schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "json_path": {
                            "type": "string",
                            "description": "Path to galleries JSON file from shootproof-cli export-galleries"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "If true, only preview matches without updating database"
                        }
                    },
                    "required": ["json_path"]
                })),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "sync_shootproof_orders".into(),
                title: Some("Sync ShootProof Orders".into()),
                description: Some("Import orders from ShootProof export JSON, updating emails and recording purchases".into()),
                input_schema: schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "json_path": {
                            "type": "string",
                            "description": "Path to orders JSON file from shootproof-cli export-orders"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "If true, only preview updates without modifying database"
                        }
                    },
                    "required": ["json_path"]
                })),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
        ];

        Ok(ListToolsResult {
            tools,
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "health" => self.0.handle_health(request).await.map_err(|e| McpError {
                code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                message: e.to_string().into(),
                data: None,
            }),
            "status" => self.0.handle_status(request).await.map_err(|e| McpError {
                code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                message: e.to_string().into(),
                data: None,
            }),
            "get_contact" => self
                .0
                .handle_get_contact(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "find_skater" => self
                .0
                .handle_find_skater(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "get_family" => self
                .0
                .handle_get_family(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "mark_gallery_sent" => {
                self.0
                    .handle_mark_gallery_sent(request)
                    .await
                    .map_err(|e| McpError {
                        code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                        message: e.to_string().into(),
                        data: None,
                    })
            }
            "list_pending_galleries" => self
                .0
                .handle_list_pending_galleries(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "competition_status" => self
                .0
                .handle_competition_status(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "create_shoot" => self
                .0
                .handle_create_shoot(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "mark_shoot_sent" => {
                self.0
                    .handle_mark_shoot_sent(request)
                    .await
                    .map_err(|e| McpError {
                        code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                        message: e.to_string().into(),
                        data: None,
                    })
            }
            "list_shoots" => self
                .0
                .handle_list_shoots(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "create_family" => self
                .0
                .handle_create_family(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "link_family_shoot" => {
                self.0
                    .handle_link_family_shoot(request)
                    .await
                    .map_err(|e| McpError {
                        code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                        message: e.to_string().into(),
                        data: None,
                    })
            }
            "record_purchase" => {
                self.0
                    .handle_record_purchase(request)
                    .await
                    .map_err(|e| McpError {
                        code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                        message: e.to_string().into(),
                        data: None,
                    })
            }
            "list_pending_shoot_galleries" => self
                .0
                .handle_list_pending_shoot_galleries(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "shoot_status" => self
                .0
                .handle_shoot_status(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "get_shoot" => self
                .0
                .handle_get_shoot(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "list_families" => self
                .0
                .handle_list_families(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "sync_shootproof_galleries" => self
                .0
                .handle_sync_shootproof_galleries(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            "sync_shootproof_orders" => self
                .0
                .handle_sync_shootproof_orders(request)
                .await
                .map_err(|e| McpError {
                    code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                    message: e.to_string().into(),
                    data: None,
                }),
            _ => Err(McpError {
                code: rmcp::model::ErrorCode::METHOD_NOT_FOUND,
                message: format!("Unknown tool: {}", request.name).into(),
                data: None,
            }),
        }
    }
}
