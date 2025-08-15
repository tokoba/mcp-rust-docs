pub mod cache;
pub mod entity;
pub mod error;
pub mod record;
pub mod repository;
pub mod tool;
pub mod use_case;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crates_io_repository =
        std::sync::Arc::new(crate::repository::crates_io::CratesIoRepositoryImpl {});
    let crates_io_use_case = crate::use_case::crates_io::CratesIoUseCase {
        crates_io_repository,
    };

    use rmcp::ServiceExt;

    let tool = crate::tool::Tool::new(crates_io_use_case)
        .serve(rmcp::transport::stdio())
        .await?;

    tool.waiting().await?;

    Ok(())
}
