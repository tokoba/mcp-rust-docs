#[derive(Debug, Clone)]
pub struct Tool {
    pub crates_io_use_case: crate::use_case::crates_io::CratesIoUseCase,
    pub docs_use_case: crate::use_case::docs::DocsUseCase,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, rmcp::schemars::JsonSchema)]
pub struct SearchCrateParams {
    /// Keyword for searching crates on crates.io. Searches by crate name.
    pub keyword: String,
}

#[derive(Debug, serde::Serialize, rmcp::schemars::JsonSchema)]
pub struct SearchCrateResult {
    pub name: String,
    pub description: Option<String>,
    pub latest_stable_version: Option<String>,
    pub latest_version: String,
    pub downloads: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize, rmcp::schemars::JsonSchema)]
pub struct RetrieveDocumentationIndexPageParams {
    /// Name of the crate
    pub crate_name: String,

    /// Crate version. For v1.0.0, use `1.0.0`. For the latest version, use `latest`.
    pub version: String,
}

#[rmcp::tool_router]
impl Tool {
    pub fn new(
        crates_io_use_case: crate::use_case::crates_io::CratesIoUseCase,
        docs_use_case: crate::use_case::docs::DocsUseCase,
    ) -> Self {
        Self {
            crates_io_use_case,
            docs_use_case,
            tool_router: Self::tool_router(),
        }
    }

    #[rmcp::tool(description = "Search for crates on crates.io and retrieve crate summaries.")]
    async fn search_crate(
        &self,
        rmcp::handler::server::tool::Parameters(SearchCrateParams { keyword }): rmcp::handler::server::tool::Parameters<SearchCrateParams>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let entities = self
            .crates_io_use_case
            .search_crate(&keyword)
            .await
            .map_err(|e| e.into())?
            .into_iter()
            .map(|c| rmcp::model::Content::text(serde_json::to_string(&c).unwrap()))
            .collect::<Vec<rmcp::model::Content>>();

        Ok(rmcp::model::CallToolResult::success(entities))
    }

    #[rmcp::tool(
        description = "Retrieves the top page of a specific version of a crate from docs.rs."
    )]
    async fn retrieve_documentation_index_page(
        &self,
        rmcp::handler::server::tool::Parameters(RetrieveDocumentationIndexPageParams {
            crate_name,
            version,
        }): rmcp::handler::server::tool::Parameters<RetrieveDocumentationIndexPageParams>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let response = self
            .docs_use_case
            .fetch_document_index_page(&crate_name, &version)
            .await
            .map_err(|e| e.into())?;

        let result = rmcp::model::Content::text(response);

        Ok(rmcp::model::CallToolResult::success(vec![result]))
    }
}

#[rmcp::tool_handler]
impl rmcp::ServerHandler for Tool {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            instructions: Some("Retrieve Rust crates and documents.".into()),
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}
