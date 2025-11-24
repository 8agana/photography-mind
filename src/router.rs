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
        // Minimal schemas (untyped) just to advertise availability
        let object_schema = std::sync::Arc::new(
            serde_json::json!({ "type": "object" })
                .as_object()
                .cloned()
                .unwrap_or_default(),
        );

        let tools = vec![
            Tool {
                name: "health".into(),
                title: Some("Health".into()),
                description: Some("Check SurrealDB connectivity and config surface".into()),
                input_schema: object_schema.clone(),
                icons: None,
                annotations: None,
                output_schema: None,
                meta: None,
            },
            Tool {
                name: "status".into(),
                title: Some("Status".into()),
                description: Some("Counts key photography tables".into()),
                input_schema: object_schema.clone(),
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
            _ => Err(McpError {
                code: rmcp::model::ErrorCode::METHOD_NOT_FOUND,
                message: format!("Unknown tool: {}", request.name).into(),
                data: None,
            }),
        }
    }
}
