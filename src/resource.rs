#[derive(Debug, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<u32>,
    pub contents: rmcp::model::ResourceContents,
}

#[derive(Debug, Clone)]
pub struct ResourceMap {
    inner: std::sync::Arc<std::collections::HashMap<String, Resource>>,
}

impl ResourceMap {
    pub fn new() -> Self {
        let mut map = std::collections::HashMap::new();

        let uri = "str://mcp-rust-docs/instruction";
        let resource = Resource {
            uri: uri.to_owned(),
            name: "Instruction".to_owned(),
            description: Some(
                "Mandatory instructions for AI agents to use MCP tools when handling Rust documentation queries"
                    .to_owned(),
            ),
            mime_type: Some("text/plain".to_owned()),
            size: None,
            contents: rmcp::model::ResourceContents::TextResourceContents {
                uri: uri.to_owned(),
                mime_type: Some("text/plain".to_owned()),
                text: include_str!("./instruction.md").to_owned(),
            },
        };

        map.insert(resource.uri.to_owned(), resource);

        Self {
            inner: std::sync::Arc::new(map),
        }
    }

    pub fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ListResourcesResult, rmcp::ErrorData>> + Send + '_
    {
        async {
            let resources = self
                .inner
                .iter()
                .map(|(_k, v)| {
                    rmcp::model::Resource::new(
                        rmcp::model::RawResource {
                            uri: v.uri.clone(),
                            name: v.name.clone(),
                            description: v.description.clone(),
                            mime_type: v.mime_type.clone(),
                            size: v.size,
                        },
                        None,
                    )
                })
                .collect::<Vec<rmcp::model::Annotated<rmcp::model::RawResource>>>();

            Ok(rmcp::model::ListResourcesResult {
                next_cursor: None,
                resources,
            })
        }
    }

    pub fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        async {
            let uri = request.uri;

            let contents = match self.inner.get(&uri) {
                Some(resource) => Ok(rmcp::model::ReadResourceResult {
                    contents: vec![resource.contents.clone()],
                }),
                None => Err(rmcp::ErrorData::resource_not_found(
                    format!("Resource not found: {}", uri),
                    None,
                )),
            };
            contents
        }
    }

    pub fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::ListResourceTemplatesResult, rmcp::ErrorData>>
    + Send
    + '_ {
        std::future::ready(Ok(rmcp::model::ListResourceTemplatesResult::default()))
    }
}
