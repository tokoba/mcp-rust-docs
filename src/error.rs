#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to initialize client: {0}")]
    InitializeClient(String),

    #[error("Network error: {0}")]
    CratesIoApi(String),

    #[error("HTTP request error: {0}")]
    Http(String),

    #[error("Failed to parse CSS Selector: {0}")]
    ScraperSelectorParse(String),

    #[error("Failed to parse HTML: {0}")]
    HtmlMainContentNotFound(String),
}

impl Into<rmcp::ErrorData> for Error {
    fn into(self) -> rmcp::ErrorData {
        rmcp::ErrorData::new(
            rmcp::model::ErrorCode(1),
            self.to_string(),
            Some(rmcp::serde_json::Value::String(self.to_string())),
        )
    }
}
