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

#[derive(Debug, serde::Deserialize, rmcp::schemars::JsonSchema)]
pub struct RetrieveDocumentationPageParams {
    /// Name of the crate
    pub crate_name: String,

    /// Crate version. For v1.0.0, use `1.0.0`. For the latest version, use `latest`.
    pub version: String,

    /// This is not a search query; you need to know the exact link path in advance.
    pub path: String,
}

impl crate::handler::Handler {
    /// Search for crates on crates.io and retrieve crate summaries.
    #[rmcp::tool]
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

    /// Retrieves the top page of a specific version of a crate from docs.rs.
    #[rmcp::tool]
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

    /// Retrieves all items (structs, enums, functions, etc.) defined in the specified crate version from docs.rs.
    /// Use this when you are unsure where a particular item is located in the documentation.
    /// Returns a list of all discoverable items for the crate and version.
    #[rmcp::tool]
    async fn retrieve_documentation_all_items(
        &self,
        rmcp::handler::server::tool::Parameters(RetrieveDocumentationIndexPageParams {
            crate_name,
            version,
        }): rmcp::handler::server::tool::Parameters<RetrieveDocumentationIndexPageParams>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let response = self
            .docs_use_case
            .fetch_all_items(&crate_name, &version)
            .await
            .map_err(|e| e.into())?
            .into_iter()
            .map(|item| rmcp::model::Content::text(serde_json::to_string(&item).unwrap()))
            .collect::<Vec<rmcp::model::Content>>();

        Ok(rmcp::model::CallToolResult::success(response))
    }

    /// Retrieves a documentation page from docs.rs.
    /// The URL must follow the format `https://docs.rs/{crate_name}/{version}/{crate_name}{path}`,
    /// such as `https://docs.rs/serde/latest/serde/de/value/struct.BoolDeserializer.html`.
    /// In this example, `path` is `/de/value/struct.BoolDeserializer.html`.
    /// If you want to explore unknown modules or structs, you can first retrieve the top page.
    /// The 'Modules' section on the top page lists top-level modules,
    /// which you can follow to find the desired module.
    #[rmcp::tool]
    async fn retrieve_documentation_page(
        &self,
        rmcp::handler::server::tool::Parameters(RetrieveDocumentationPageParams {
            crate_name,
            version,
            path,
        }): rmcp::handler::server::tool::Parameters<RetrieveDocumentationPageParams>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let response = self
            .docs_use_case
            .fetch_document_page(&crate_name, &version, &path)
            .await
            .map_err(|e| e.into())?;

        let result = rmcp::model::Content::text(response);

        Ok(rmcp::model::CallToolResult::success(vec![result]))
    }
}
