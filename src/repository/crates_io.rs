#[async_trait::async_trait]
pub trait CratesIoRepository: std::fmt::Debug + Send + Sync {
    async fn search_crate(
        &self,
        keyword: &str,
    ) -> Result<Vec<crate::record::crates_io::CrateRecord>, crate::error::Error>;
}

#[derive(Debug, Default)]
pub struct CratesIoRepositoryImpl {}

#[async_trait::async_trait]
impl CratesIoRepository for CratesIoRepositoryImpl {
    async fn search_crate(
        &self,
        keyword: &str,
    ) -> Result<Vec<crate::record::crates_io::CrateRecord>, crate::error::Error> {
        let client = crate::cache::get_or_init_crates_io_api_client().await?;

        let query = crates_io_api::CratesQuery::builder()
            .page_size(10)
            .search(keyword)
            .sort(crates_io_api::Sort::Relevance)
            .build();

        let response = client
            .crates(query)
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                crate::error::Error::CratesIoApi(e.to_string())
            })?
            .crates
            .into_iter()
            .map(|c| crate::record::crates_io::CrateRecord {
                name: c.name,
                description: c.description,
                latest_stable_version: c.max_stable_version,
                latest_version: c.max_version,
                downloads: c.downloads,
                created_at: c.created_at.to_rfc3339(),
                updated_at: c.updated_at.to_rfc3339(),
                ..Default::default()
            })
            .collect::<Vec<crate::record::crates_io::CrateRecord>>();

        Ok(response)
    }
}
