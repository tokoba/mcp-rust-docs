#[derive(Debug, Clone)]
pub struct DocsUseCase {
    pub http_repository: std::sync::Arc<dyn crate::repository::http::HttpRepository + Send + Sync>,
}

impl DocsUseCase {
    fn extract_main_content(
        &self,
        html: &str,
        selector: &str,
    ) -> Result<String, crate::error::Error> {
        let document = scraper::Html::parse_document(&html);

        let selector = scraper::Selector::parse(selector).map_err(|e| {
            tracing::error!("{} This error is due to a static selector configuration mistake on the crate side. Please create an issue if necessary.", e.to_string());
            crate::error::Error::ScraperSelectorParse(e.to_string())
        })?;

        let contents = document.select(&selector);

        let mut iter = contents.into_iter();

        if let Some(first) = iter.next() {
            let html = first.inner_html().to_string();
            return Ok(html);
        } else {
            Err(crate::error::Error::HtmlMainContentNotFound(String::from(
                "Element not found: section#main-content",
            )))
        }
    }

    fn cleanup_html(&self, html: &str) -> Result<String, crate::error::Error> {
        let re_class = regex::Regex::new(r#"\sclass=(".*?"|'.*?')"#).unwrap();
        let re_script = regex::Regex::new(r#"(?is)<script.*?</script>"#).unwrap();
        let re_toolbar =
            regex::Regex::new(r#"(?is)<rustdoc-toolbar.*?</rustdoc-toolbar>"#).unwrap();

        let result = re_class.replace_all(&html, "");
        let result = re_script.replace_all(&result, "");
        let result = re_toolbar.replace_all(&result, "");

        return Ok(result.to_string());
    }

    pub async fn fetch_document_index_page(
        &self,
        crate_name: &str,
        version: &str,
    ) -> Result<String, crate::error::Error> {
        let url = format!("https://docs.rs/{crate_name}/{version}/{crate_name}/index.html");

        let raw_html = self.http_repository.get(&url).await?;
        let main_html = self.extract_main_content(&raw_html, "section#main-content")?;
        let result = self.cleanup_html(&main_html)?;

        Ok(result)
    }
}
