static CRATE_IO_API_CLIENT: tokio::sync::OnceCell<crates_io_api::AsyncClient> =
    tokio::sync::OnceCell::const_new();

pub async fn get_or_init_crates_io_api_client()
-> Result<&'static crates_io_api::AsyncClient, crate::error::Error> {
    CRATE_IO_API_CLIENT
        .get_or_try_init(|| async {
            let client = crates_io_api::AsyncClient::new(
                "mcp-rust-docs",
                std::time::Duration::from_millis(3000),
            )
            .map_err(|e| {
                tracing::error!("{}", e);
                crate::error::Error::InitializeClient(e.to_string())
            })?;

            Ok(client)
        })
        .await
}

static REQWEST_CLIENT: tokio::sync::OnceCell<reqwest::Client> = tokio::sync::OnceCell::const_new();

pub async fn get_or_init_reqwest_client() -> Result<&'static reqwest::Client, crate::error::Error> {
    REQWEST_CLIENT
        .get_or_try_init(|| async { Ok(reqwest::Client::new()) })
        .await
}
