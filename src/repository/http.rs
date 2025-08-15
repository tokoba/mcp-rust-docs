#[async_trait::async_trait]
pub trait HttpRepository: std::fmt::Debug + Send + Sync {
    async fn get(&self, url: &str) -> Result<String, crate::error::Error>;
}

#[derive(Debug)]
pub struct HttpRepositoryImpl {}

#[async_trait::async_trait]
impl HttpRepository for HttpRepositoryImpl {
    async fn get(&self, url: &str) -> Result<String, crate::error::Error> {
        let client = crate::cache::get_or_init_reqwest_client().await?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                crate::error::Error::Http(e.to_string())
            })?
            .text()
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                crate::error::Error::Http(e.to_string())
            })?;

        Ok(response)
    }
}
