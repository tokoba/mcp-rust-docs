#[derive(Debug, Clone)]
pub struct Handler {
    pub crates_io_use_case: crate::use_case::crates_io::CratesIoUseCase,
    pub docs_use_case: crate::use_case::docs::DocsUseCase,
    pub tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
    pub resource_map: crate::resource::ResourceMap,
}

#[rmcp::tool_handler]
impl rmcp::ServerHandler for Handler {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            instructions: Some("Retrieve Rust crates and documents.".into()),
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }

    fn list_resources(
        &self,
        request: Option<rmcp::model::PaginatedRequestParam>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ListResourcesResult, rmcp::ErrorData>> + Send + '_
    {
        self.resource_map.list_resources(request, context)
    }

    fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        self.resource_map.read_resource(request, context)
    }

    fn list_resource_templates(
        &self,
        request: Option<rmcp::model::PaginatedRequestParam>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ListResourceTemplatesResult, rmcp::ErrorData>>
    + Send
    + '_ {
        self.resource_map.list_resource_templates(request, context)
    }
}
