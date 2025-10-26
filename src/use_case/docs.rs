use tantivy::schema::Value;

#[derive(Debug, Clone)]
pub struct DocsUseCase {
    pub http_repository: std::sync::Arc<dyn crate::repository::http::HttpRepository + Send + Sync>,
}

impl DocsUseCase {
    pub(super) fn extract_main_content(
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

    pub async fn fetch_document_index_page(
        &self,
        crate_name: &str,
        version: &str,
    ) -> Result<String, crate::error::Error> {
        let url = format!("https://docs.rs/{crate_name}/{version}/{crate_name}/index.html");

        let raw_html = self.http_repository.get(&url).await?;
        let main_html = self.extract_main_content(&raw_html, "section#main-content")?;
        let markdown = html2md::rewrite_html(&main_html, true);

        Ok(markdown)
    }

    pub async fn fetch_document_page(
        &self,
        crate_name: &str,
        version: &str,
        path: &str,
    ) -> Result<String, crate::error::Error> {
        let url = format!("https://docs.rs/{crate_name}/{version}/{crate_name}{path}");

        let raw_html = self.http_repository.get(&url).await?;
        let main_html = self.extract_main_content(&raw_html, "section#main-content")?;
        let markdown = html2md::rewrite_html(&main_html, true);

        Ok(markdown)
    }

    pub(super) fn parse_all_items(
        &self,
        html: &str,
    ) -> Result<Vec<crate::entity::docs::Item>, crate::error::Error> {
        let document = scraper::Html::parse_document(html);
        let h3_selector = scraper::Selector::parse("section#main-content > h3").map_err(|e| {
            tracing::error!("{}", e);
            crate::error::Error::ScraperSelectorParse(e.to_string())
        })?;
        let ul_selector = scraper::Selector::parse("section#main-content > ul").map_err(|e| {
            tracing::error!("{}", e);
            crate::error::Error::ScraperSelectorParse(e.to_string())
        })?;
        let a_selector = scraper::Selector::parse("a").map_err(|e| {
            tracing::error!("{}", e);
            crate::error::Error::ScraperSelectorParse(e.to_string())
        })?;

        let h3_elements = document.select(&h3_selector).into_iter();
        let ul_elements = document.select(&ul_selector).into_iter();

        let zipped = h3_elements
            .zip(ul_elements)
            .collect::<Vec<(scraper::ElementRef<'_>, scraper::ElementRef<'_>)>>();

        let items = zipped
            .into_iter()
            .map(|(h3, ul)| {
                let r#type = h3.inner_html().trim().to_string();

                let items = ul
                    .select(&a_selector)
                    .into_iter()
                    .map(|a| {
                        let href = a.attr("href").map(|href| href.to_string());
                        let path = Some(a.inner_html());
                        crate::entity::docs::Item {
                            r#type: r#type.clone(),
                            href,
                            path,
                        }
                    })
                    .collect::<Vec<crate::entity::docs::Item>>();

                items
            })
            .flatten()
            .collect::<Vec<crate::entity::docs::Item>>();

        Ok(items)
    }

    pub async fn fetch_all_items(
        &self,
        crate_name: &str,
        version: &str,
    ) -> Result<Vec<crate::entity::docs::Item>, crate::error::Error> {
        let url = format!("https://docs.rs/{crate_name}/{version}/{crate_name}/all.html");

        let raw_html = self.http_repository.get(&url).await?;

        let items = self.parse_all_items(&raw_html)?;

        Ok(items)
    }

    pub async fn search_items(
        &self,
        crate_name: &str,
        version: &str,
        keyword: &str,
    ) -> Result<Vec<crate::entity::docs::Item>, crate::error::Error> {
        let items = self.fetch_all_items(crate_name, version).await?;

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("type", tantivy::schema::STORED);
        schema_builder.add_text_field("href", tantivy::schema::STORED);
        schema_builder.add_text_field("path", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let index_path = tempfile::tempdir().map_err(|e| {
            tracing::error!("{}", e);
            crate::error::Error::CreateTempDir(e.to_string())
        })?;

        let index = tantivy::Index::create_in_dir(&index_path, schema.clone())?;
        let mut index_writer: tantivy::IndexWriter = index.writer(50_000_000)?;

        let type_field = schema.get_field("type")?;
        let href_field = schema.get_field("href")?;
        let path_field = schema.get_field("path")?;

        for item in items {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(type_field, &item.r#type);
            if let Some(href) = &item.href {
                doc.add_text(href_field, href);
            }
            if let Some(path) = &item.path {
                doc.add_text(path_field, path);
            }
            index_writer.add_document(doc)?;
        }

        index_writer.commit()?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let query_parser = tantivy::query::QueryParser::for_index(&index, vec![path_field]);

        let query = query_parser.parse_query(keyword)?;
        let searcher = reader.searcher();

        let top_docs = searcher.search(&query, &tantivy::collector::TopDocs::with_limit(10))?;

        let mut result_items = Vec::new();

        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let item_type = retrieved_doc
                .get_first(type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let href = retrieved_doc
                .get_first(href_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let path = retrieved_doc
                .get_first(path_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let item = crate::entity::docs::Item {
                r#type: item_type,
                href,
                path,
            };

            result_items.push(item);
        }

        Ok(result_items)
    }
}

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn test_fetch_document_page() -> Result<(), crate::error::Error> {
        let http_repository = std::sync::Arc::new(crate::repository::http::HttpRepositoryImpl {});
        let use_case = crate::use_case::docs::DocsUseCase { http_repository };

        let res = use_case.fetch_all_items("serde", "latest").await;

        assert!(res.is_ok());

        Ok(())
    }
}
