# DESIGN.md — MCP Rust Docs Server 詳細設計

この文書は、mcp-rust-docs プロジェクトの詳細な設計内容を実装者向けに整理したものです。コードを読み進める際の補助となるよう、各ファイル・各レイヤにおけるデータ構造、関数・トレイトの仕様、非同期処理、データ変換、エラーハンドリング、あいまい検索実装（tantivy）などを具体例と共に説明します。

- 対象リポジトリ: https://github.com/46ki75/mcp-rust-docs
- 対象コミット: refs/heads/main（最新）
- 言語: Rust（Edition 2024）
- 主要外部依存: rmcp, tokio, reqwest, crates_io_api, scraper, fast_html2md, tantivy, tempfile, thiserror, serde, serde_json, schemars, tracing
- 設計思想: クリーンアーキテクチャ（Entity/Use Case/Repository の分離、Interface Adapter として MCP Handler/Tool/Resource）
- 本書は ARCHITECTURE.md と SPECIFICATION.md を補完する詳細設計資料であり、実装の理解と拡張を容易にします。

---

## 0. 全体概要と責務分担

- **Interface Adapter層**: MCPのプロトコル仕様を実装する層（src/handler.rs, src/tool.rs, src/resource.rs）
- **Application層（Use Case）**: ビジネスロジック（src/use_case/crates_io.rs, src/use_case/docs.rs）
- **Domain層（Entity）**: 純粋なデータ構造（src/entity/*）
- **Infrastructure層（Repository/Cache/Error）**: 外部API呼び出し（src/repository/*）、クライアントキャッシュ（src/cache.rs）、統一エラー型（src/error.rs）

アプリ起動と依存注入は src/main.rs が担当します。非同期ランタイムとして tokio を使用し、MCPサーバは rmcp の transport-io（stdio）経由で起動されます。

---

## 1. 各ファイルの詳細な実装設計

### 1.1 src/main.rs の初期化処理と依存性注入

- 目的: アプリケーションのエントリポイント。リポジトリ実装を生成し、Use Case に注入し、Handler を起動（serve）して待機（waiting）します。
- フロー概要:
  1) crates.io リポジトリの具象実装生成（Arc）
  2) CratesIoUseCase に注入
  3) HTTP リポジトリ（reqwest ベース）の具象実装生成（Arc）
  4) DocsUseCase に注入
  5) Handler::new(crates_io_use_case, docs_use_case) → rmcp::ServerHandler を実装
  6) rmcp::ServiceExt::serve(stdio) → waiting

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crates_io_repository =
        std::sync::Arc::new(crate::repository::crates_io::CratesIoRepositoryImpl {});
    let crates_io_use_case = crate::use_case::crates_io::CratesIoUseCase {
        crates_io_repository,
    };

    let http_repository = std::sync::Arc::new(crate::repository::http::HttpRepositoryImpl {});
    let http_use_case = crate::use_case::docs::DocsUseCase { http_repository };

    use rmcp::ServiceExt;

    let tool = crate::handler::Handler::new(crates_io_use_case, http_use_case)
        .serve(rmcp::transport::stdio())
        .await?;
    tool.waiting().await?;
    Ok(())
}
```

- 依存注入ポリシー:
  - Use Caseは Repository トレイトに依存（DIP）
  - 具象リポジトリ実装は Infrastructure 層に閉じ込め、Use Caseは抽象（トレイト）を参照
  - 共有は Arc<dyn Trait + Send + Sync> で行い、スレッド安全性を担保

- 実行時挙動:
  - 標準入出力(stdio)でMCPクライアント（例えばVSCodeアシスタント等）と通信
  - tokioランタイムは1プロセス内に生成される

---

### 1.2 src/handler.rs の MCP ハンドラー実装

- 目的: MCPの ServerHandler を実装し、ServerInfo/Resource API を提供します。
- 構造体:

```rust
#[derive(Debug, Clone)]
pub struct Handler {
    pub crates_io_use_case: crate::use_case::crates_io::CratesIoUseCase,
    pub docs_use_case: crate::use_case::docs::DocsUseCase,
    pub tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
    pub resource_map: crate::resource::ResourceMap,
}
```

- ServerInfo:
  - instructions: "Retrieve Rust crates and documents."
  - capabilities: Tools + Resources 有効
  - implementation: name/version/title/website_url を提供
- Resource API:
  - list_resources(request, context): resource_map.list_resources に委譲
  - read_resource(request, context): resource_map.read_resource に委譲
  - list_resource_templates(request, context): resource_map.list_resource_templates に委譲
- rmcpのマクロ:

```rust
#[rmcp::tool_handler]
impl rmcp::ServerHandler for Handler { ... }
```

- 特記事項:
  - Handler は ToolRouter と ResourceMap を持ち、MCPクライアントからの呼び出しを Use Case に橋渡しします
  - 実装者は Handler に新ツール関数を追加することで拡張可能（rmcp::tool マクロ使用）

---

### 1.3 src/tool.rs の各ツール実装の詳細

- 目的: MCPツール(API)の公開と、パラメータ/戻り値型の定義（JsonSchema含む）
- ツール関数一覧
  - search_crate
  - retrieve_documentation_index_page
  - retrieve_documentation_all_items
  - search_documentation_items
  - retrieve_documentation_page

- ツールルータ:

```rust
#[rmcp::tool_router]
impl crate::handler::Handler {
    pub fn new(crates_io_use_case: ..., docs_use_case: ...) -> Self { ... }

    #[rmcp::tool]
    async fn search_crate(&self, Parameters(SearchCrateParams { keyword }): Parameters<SearchCrateParams>) -> Result<CallToolResult, ErrorData> { ... }

    #[rmcp::tool]
    async fn retrieve_documentation_index_page(&self, Parameters(RetrieveDocumentationIndexPageParams { crate_name, version }): Parameters<...>) -> Result<CallToolResult, ErrorData> { ... }

    #[rmcp::tool]
    async fn retrieve_documentation_all_items(&self, Parameters(RetrieveDocumentationIndexPageParams { crate_name, version }): Parameters<...>) -> Result<CallToolResult, ErrorData> { ... }

    #[rmcp::tool]
    async fn search_documentation_items(&self, Parameters(SearchDocumentationItemsParams { crate_name, version, keyword }): Parameters<...>) -> Result<CallToolResult, ErrorData> { ... }

    #[rmcp::tool]
    async fn retrieve_documentation_page(&self, Parameters(RetrieveDocumentationPageParams { crate_name, version, path }): Parameters<...>) -> Result<CallToolResult, ErrorData> { ... }
}
```

- パラメータ/結果のスキーマ定義（schemars + serde）:

```rust
#[derive(Debug, serde::Deserialize, rmcp::schemars::JsonSchema)]
pub struct SearchCrateParams { pub keyword: String }
#[derive(Debug, serde::Serialize, rmcp::schemars::JsonSchema)]
pub struct SearchCrateResult { /* name, description, ... */ }
...
```

- search_crate 実装詳細:
  - crates_io_use_case.search_crate(keyword) を呼び出し
  - 返却エンティティを serde_json::to_string で JSON文字列化し、rmcp::model::Content::text(...) に詰める
  - 注意点: 現在 unwrap を使用（失敗時 panic の可能性）
    - 改善案: map_err で ErrorData へ変換する安全なパスへ変更（下記例）

```rust
// 現在（panic可能性あり）
.map(|c| rmcp::model::Content::text(serde_json::to_string(&c).unwrap()))

// 改善案（panic排除）
.map(|c| {
    match serde_json::to_string(&c) {
        Ok(s) => rmcp::model::Content::text(s),
        Err(e) => return Err(rmcp::ErrorData::new(rmcp::model::ErrorCode(1), format!("Serialization error: {}", e), None)),
    }
})
```

- retrieve_documentation_*:
  - docs_use_case 側のメソッド呼び出し → Markdown相当のテキスト1件（index/page）または JSON文字列群（items/search）が返る

---

### 1.4 src/resource.rs のリソース実装

- 目的: MCP Resource API で返す静的リソース（Instruction）を提供
- Resource 構造体と ResourceMap:

```rust
#[derive(Debug, Clone)]
pub struct Resource { pub uri: String, pub name: String, pub description: Option<String>, pub mime_type: Option<String>, pub size: Option<u32>, pub contents: rmcp::model::ResourceContents }

#[derive(Debug, Clone)]
pub struct ResourceMap { inner: Arc<HashMap<String, Resource>> }
```

- 初期化:
  - Instruction リソースを include_str!("./instruction.md") で組み込み
  - URI: "str://mcp-rust-docs/instruction", name: "Instruction", mime_type: "text/plain"

```rust
contents: rmcp::model::ResourceContents::TextResourceContents {
  uri: uri.to_owned(),
  mime_type: Some("text/plain".to_owned()),
  text: include_str!("./instruction.md").to_owned(),
  meta: None
}
```

- API:
  - list_resources: RawResource のベクタを返却（タイトル付き）
  - read_resource: URIに一致すれば contents を返却、無ければ resource_not_found
  - list_resource_templates: 現状空

---

### 1.5 src/error.rs のエラーハンドリング設計

- 目的: 統一エラー型（thiserror利用）を定義し、rmcp::ErrorData へ変換可能にする
- エラー型:

```rust
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
    #[error("Failed to create temporary directories.")]
    CreateTempDir(String),
    #[error("{0}")]
    FuzzySearch(#[from] tantivy::error::TantivyError),
    #[error("{0}")]
    FuzzySearchQueryParse(#[from] tantivy::query::QueryParserError),
}
```

- MCPエラーへの変換:

```rust
impl Into<rmcp::ErrorData> for Error {
    fn into(self) -> rmcp::ErrorData {
        rmcp::ErrorData::new(
            rmcp::model::ErrorCode(1),
            self.to_string(),
            Some(rmcp::serde_json::Value::String(self.to_string())),
        )
    }
}
```

- 方針:
  - 例外はすべて Result<... , Error> で表現
  - MCPツール返却時は ErrorData に変換（ErrorCode=1固定）
  - メッセージは to_string() に依存（機密情報は含めない）

---

### 1.6 src/cache.rs のキャッシュ機構実装

- 目的: crates_io_api::AsyncClient と reqwest::Client を OnceCell により初期化/共有
- 設計:
  - tokio::sync::OnceCell による非同期安全な1回限り初期化
  - 'static ライフタイム参照を返す

```rust
static CRATE_IO_API_CLIENT: tokio::sync::OnceCell<crates_io_api::AsyncClient> = tokio::sync::OnceCell::const_new();

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
        }).await
}
```

- reqwest クライアント:

```rust
static REQWEST_CLIENT: tokio::sync::OnceCell<reqwest::Client> = tokio::sync::OnceCell::const_new();
pub async fn get_or_init_reqwest_client() -> Result<&'static reqwest::Client, crate::error::Error> {
    REQWEST_CLIENT.get_or_try_init(|| async { Ok(reqwest::Client::new()) }).await
}
```

- 考慮事項:
  - 初期化失敗は Error::InitializeClient または Error::Http で伝播
  - タイムアウト: crates_io_api側の初期化で 3秒設定
  - User-Agent: "mcp-rust-docs" を明示

---

## 2. エンティティ層の詳細設計（src/entity/）

### 2.1 crates_io.rs のデータ構造

- 目的: crates.io クレート情報のドメインエンティティ（UI/ツール返却用）

```rust
#[derive(Debug, serde::Serialize)]
pub struct CrateSummaryEntity {
    pub name: String,
    pub description: Option<String>,
    pub latest_stable_version: Option<String>,
    pub latest_version: String,
    pub downloads: u64,
    pub created_at: String,
    pub updated_at: String,
}
```

- フィールド意味:
  - name: クレート名
  - description: 説明（省略可）
  - latest_stable_version: 最新安定版（ない場合 None）
  - latest_version: 最新版（alpha/beta等含む可能性）
  - downloads: 総ダウンロード数
  - created_at/updated_at: ISO8601形式の日時文字列

- 直列化: serde::Serialize（MCPツール返却時に JSON 文字列化）

---

### 2.2 docs.rs のデータ構造

- 目的: docs.rs の項目（struct/enum/trait/fn/module など）を表現

```rust
#[derive(Debug, serde::Serialize)]
pub struct Item {
    pub r#type: String,
    pub href: Option<String>,
    pub path: Option<String>,
}
```

- フィールド意味:
  - type: 項目種別（例: "trait", "struct", "enum", "fn", "module"）
  - href: 相対リンク（"/de/trait.Deserialize.html" 等）
  - path: 表示名（アンカーテキスト、例: "serde::de::Deserialize"）

- 直列化: serde::Serialize（MCPツール返却時に JSON 文字列化）

---

## 3. レコード層の詳細設計（src/record/）

### 3.1 crates_io.rs のDTO設計

- 目的: 外部API（crates_io_api）のレスポンスから受け取ったフィールドを保持する内部DTO

```rust
#[derive(Debug, Default)]
pub struct CrateRecord {
    pub name: String,
    pub description: Option<String>,
    pub latest_stable_version: Option<String>,
    pub latest_version: String,
    pub downloads: u64,
    pub created_at: String,
    pub updated_at: String,
}
```

- 役割: リポジトリ層 → Use Case 層 へのデータ受け渡し用（Entityへの変換元）

### 3.2 データ変換ロジック

- Use Case（CratesIoUseCase.search_crate）は Record → Entity の単純コピーを行う:

```rust
let entities = crates.into_iter().map(|c| CrateSummaryEntity {
    name: c.name,
    description: c.description,
    latest_stable_version: c.latest_stable_version,
    latest_version: c.latest_version,
    downloads: c.downloads,
    created_at: c.created_at,
    updated_at: c.updated_at,
}).collect::<Vec<_>>();
```

---

## 4. リポジトリ層の詳細設計（src/repository/）

### 4.1 crates_io.rs の API アクセス実装

- 抽象トレイト:

```rust
#[async_trait::async_trait]
pub trait CratesIoRepository: std::fmt::Debug + Send + Sync {
    async fn search_crate(&self, keyword: &str)
        -> Result<Vec<crate::record::crates_io::CrateRecord>, crate::error::Error>;
}
```

- 具象実装: CratesIoRepositoryImpl
  - crates_io_api::AsyncClient を cache::get_or_init_crates_io_api_client() で取得
  - CratesQuery を組み立て（page_size=10, search(keyword), sort=Relevance）
  - client.crates(query).await で検索
  - 失敗時: Error::CratesIoApi(e.to_string()) + tracing::error ログ
  - 結果マッピング: crates_io_api のレスポンス構造体 → CrateRecord へ詰め替え

```rust
let query = crates_io_api::CratesQuery::builder()
    .page_size(10)
    .search(keyword)
    .sort(crates_io_api::Sort::Relevance)
    .build();
```

- 設計思想:
  - **トレイト指向（Repositoryパターン）**によりテスト容易性/差し替え容易性を確保
  - APIクライアントは OnceCell で共有し、接続プールなどを有効活用

---

### 4.2 http.rs の HTTP クライアント実装

- 抽象トレイト:

```rust
#[async_trait::async_trait]
pub trait HttpRepository: std::fmt::Debug + Send + Sync {
    async fn get(&self, url: &str) -> Result<String, crate::error::Error>;
}
```

- 具象実装: HttpRepositoryImpl
  - reqwest::Client を cache::get_or_init_reqwest_client() で取得
  - client.get(url).send().await してステータスコードを確認
  - !is_success() の場合 Error::Http("Failed to fetch URL ...: status") を返却
  - response.text().await を文字列として返却
  - 失敗時は tracing::error ログ + Error::Http(e.to_string())

```rust
if !response.status().is_success() {
    return Err(crate::error::Error::Http(format!(
        "Failed to fetch URL {}: {}",
        url,
        response.status()
    )));
}
```

- 設計思想:
  - **インフラ抽象化**により HTTP 実装を隠蔽
  - タイムアウト・リトライ・ボディ制限は reqwest 設定/拡張で追加可能（現行は最小構成）

---

## 5. ユースケース層の詳細設計（src/use_case/）

### 5.1 crates_io.rs のビジネスロジック

- 構造体:

```rust
#[derive(Debug, Clone)]
pub struct CratesIoUseCase {
    pub crates_io_repository:
        Arc<dyn crate::repository::crates_io::CratesIoRepository + Send + Sync>,
}
```

- 関数: search_crate(&self, keyword: &str) -> Result<Vec<CrateSummaryEntity>, Error>
  - Repository.search_crate(keyword) を await
  - Record → Entity へ変換
  - 返却: Entityのベクタ
  - 失敗: Error をそのまま伝播

---

### 5.2 docs.rs のドキュメント処理ロジック

- 構造体:

```rust
#[derive(Debug, Clone)]
pub struct DocsUseCase {
    pub http_repository: Arc<dyn crate::repository::http::HttpRepository + Send + Sync>,
}
```

- 主関数:
  1) fetch_document_index_page(crate_name, version) → Indexページの main-content を抽出して Markdown 変換
  2) fetch_document_page(crate_name, version, path) → 指定ページの main-content を抽出して Markdown 変換
  3) fetch_all_items(crate_name, version) → all.html を取得して parse_all_items で項目抽出
  4) search_items(crate_name, version, keyword) → all_items をインデックス化してキーワード検索

#### 5.2.1 HTMLパースとMarkdown変換の詳細

- main-content 抽出:

```rust
pub(super) fn extract_main_content(&self, html: &str, selector: &str)
-> Result<String, crate::error::Error> {
    let document = scraper::Html::parse_document(&html);
    let selector = scraper::Selector::parse(selector).map_err(|e| Error::ScraperSelectorParse(e.to_string()))?;
    let mut iter = document.select(&selector).into_iter();
    if let Some(first) = iter.next() {
        Ok(first.inner_html().to_string())
    } else {
        Err(Error::HtmlMainContentNotFound("Element not found: section#main-content".to_string()))
    }
}
```

- Markdown変換: fast_html2md（html2md::rewrite_html）を利用

```rust
let markdown = html2md::rewrite_html(&main_html, false);
```

- all_items抽出ロジック（parse_all_items）:
  - "section#main-content > h3" と "section#main-content > ul" をzip
  - 各 h3（カテゴリ見出し: "Structs", "Enums", "Traits" 等）に紐づく ul 内の a を列挙し Item を生成
  - href は a.attr("href")、path は a.inner_html

```rust
let items = zipped.into_iter().map(|(h3, ul)| {
    let r#type = h3.inner_html().trim().to_string();
    let items = ul.select(&a_selector).into_iter().map(|a| {
        let href = a.attr("href").map(|href| href.to_string());
        let path = Some(a.inner_html());
        crate::entity::docs::Item { r#type: r#type.clone(), href, path }
    }).collect::<Vec<_>>();
    items
}).flatten().collect::<Vec<_>>();
```

#### 5.2.2 あいまい検索の実装（tantivy 使用）

- 目的: Item の path に対するキーワード検索（上位10件）
- スキーマ:
  - type: STORED
  - href: STORED
  - path: TEXT | STORED（検索対象）

```rust
let mut schema_builder = tantivy::schema::Schema::builder();
schema_builder.add_text_field("type", tantivy::schema::STORED);
schema_builder.add_text_field("href", tantivy::schema::STORED);
schema_builder.add_text_field("path", tantivy::schema::TEXT | tantivy::schema::STORED);
let schema = schema_builder.build();
```

- インデックス作成:
  - tempfile::tempdir() で一時ディレクトリを作成（失敗は Error::CreateTempDir）
  - Index::create_in_dir(schema)
  - IndexWriter(50MB) を作成
  - items を TantivyDocument に追加し commit
  - reader を生成、QueryParser(path_field) で keyword をパース、TopDocs limit 10

```rust
let index = tantivy::Index::create_in_dir(&index_path, schema.clone())?;
let mut index_writer: tantivy::IndexWriter = index.writer(50_000_000)?;
...
index_writer.add_document(doc)?; index_writer.commit()?;
let reader = index.reader_builder().reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay).try_into()?;
let query_parser = tantivy::query::QueryParser::for_index(&index, vec![path_field]);
let query = query_parser.parse_query(keyword)?;
let top_docs = searcher.search(&query, &tantivy::collector::TopDocs::with_limit(10))?;
```

- 結果復元:
  - doc から type/href/path を取り出し、Item を再構築して返却

---

## 6. 関数、impl、trait の詳細な仕様

以下、主な関数/トレイト/impl の仕様要点（シグネチャ・役割・副作用・エラー）を列挙します。

- main.rs::main()
  - async fn main() -> Result<(), Box<dyn Error>>
  - 役割: 依存性注入、サーバ起動、待機
  - 副作用: rmcp stdio I/O
  - エラー: 外部実行失敗は ? により伝播

- handler.rs::ServerHandler 実装（get_info/list_resources/read_resource/list_resource_templates）
  - 役割: MCPプロトコルの基本API
  - 副作用: なし（resource_map 内の読み取りのみ）
  - エラー: Resource not found 等は rmcp::ErrorData で返却

- tool.rs::Handler::new
  - fn new(crates_io_use_case: CratesIoUseCase, docs_use_case: DocsUseCase) -> Self
  - 役割: ツールルータ生成、ResourceMap 初期化
  - 副作用: なし

- tool.rs::Handler::search_crate
  - async fn search_crate(&self, Parameters<SearchCrateParams>) -> Result<CallToolResult, ErrorData>
  - 役割: crates.io 検索、結果を JSON文字列コンテンツ群として返却
  - エラー: UseCase 失敗 → ErrorData、直列化失敗現在は panic の可能性（改善要）

- tool.rs::Handler::retrieve_documentation_index_page
  - async fn ... -> Result<CallToolResult, ErrorData>
  - 役割: docs.rs index を取得し Markdown相当テキストで返却
  - エラー: HTTP失敗、CSSセレクタ不正、main-content 異常等

- tool.rs::Handler::retrieve_documentation_all_items / search_documentation_items
  - 役割: 全項目一覧 or キーワード検索（JSON文字列コンテンツ群で返却）
  - エラー: 上記に準拠

- tool.rs::Handler::retrieve_documentation_page
  - 役割: href相当の path を付与して特定ページ取得、Markdown相当テキスト返却
  - エラー: HTTP/CSS/Main content 関連の失敗

- repository/crates_io.rs::CratesIoRepository::search_crate
  - async fn ... -> Result<Vec<CrateRecord>, Error>
  - 役割: crates.io_api 経由で最大10件を関連度順に取得
  - エラー: API失敗 → Error::CratesIoApi

- repository/http.rs::HttpRepository::get
  - async fn get(&self, url: &str) -> Result<String, Error>
  - 役割: 一般HTTP GET、ステータスチェック、本文テキスト返却
  - エラー: ネットワーク/HTTP失敗は Error::Http

- use_case/docs.rs::DocsUseCase::fetch_document_index_page/fetch_document_page
  - 役割: 指定URLからHTML取得 → main-content抽出 → Markdown変換
  - エラー: HTTP/CSS/Main content/変換の失敗

- use_case/docs.rs::DocsUseCase::fetch_all_items/parse_all_items
  - 役割: all.html を取得して Item にパース
  - エラー: HTTP/CSSパースの失敗

- use_case/docs.rs::DocsUseCase::search_items
  - 役割: 全Itemを tantivy に格納し keyword 検索（pathに対して）
  - エラー: tempfile, tantivy, QueryParser などの失敗は Error::FuzzySearch*, CreateTempDir

---

## 7. 非同期処理の詳細設計

- async/await の使用パターン:
  - Use Case → Repository はすべて非同期（外部I/O のため）
  - Handler/Tool 関数も async で rmcp ツール呼び出しに対応
  - OnceCell は async 初期化をサポート（get_or_try_init(async || {...})）

- エラーハンドリングと Result 型:
  - Repository/Use Case は Result<..., crate::error::Error> を返却
  - Tool 層は Result<..., rmcp::ErrorData> を返却（Error を Into 変換）
  - unwrap の使用箇所が一部にあるため、運用上は panic を避ける改修が望ましい

- 並行処理の考慮事項:
  - reqwest::Client と crates_io_api::AsyncClient は OnceCell で共有（スレッド安全）
  - Arc<dyn Trait + Send + Sync> で依存を共有
  - tantivy の一時インデックスは関数内スコープで作成・破棄（tempdirのライフサイクルに注意）

---

## 8. データ変換とシリアライズ

- serde を使用した JSON シリアライズ:
  - Entity（CrateSummaryEntity/Item）は serde::Serialize
  - MCPツールは JSON文字列として text コンテンツに詰めて返却
  - 現在 unwrap を使用しているため、直列化失敗への安全策が必要（ErrorData変換）

- HTML から Markdown への変換ロジック:
  - fast_html2md（html2md::rewrite_html）で main-content を Markdown相当テキストへ変換
  - 変換時の第二引数（convert_img）は false（画像等の変換はオフ）

- データ検証とエラー処理:
  - path（retrieve_documentation_page の引数）は "/" 始まりが期待されるが、現行コードでは厳密検証なし
    - 改善案: 入力検証を追加し、不正な path は ErrorData("Invalid path: must start with '/'") を返す
  - URL 生成時は crate_name/version/path をフォーマットするが、SSRF対策として外部URLへの書き換えは不可（http リポジトリ層は指定URLの GET のみ）

---

## 9. 代表的な使用例（抜粋）

- crates.io でクレート検索:

```json
{
  "tool": "search_crate",
  "params": { "keyword": "serde" }
}
```

- docs.rs トップページ取得:

```json
{
  "tool": "retrieve_documentation_index_page",
  "params": { "crate_name": "serde", "version": "latest" }
}
```

- 全項目一覧取得:

```json
{
  "tool": "retrieve_documentation_all_items",
  "params": { "crate_name": "serde", "version": "latest" }
}
```

- 項目のあいまい検索:

```json
{
  "tool": "search_documentation_items",
  "params": { "crate_name": "serde", "version": "latest", "keyword": "Deserialize" }
}
```

- 特定ページ取得:

```json
{
  "tool": "retrieve_documentation_page",
  "params": { "crate_name": "serde", "version": "latest", "path": "/de/trait.Deserialize.html" }
}
```

---

## 10. セキュリティ/入力検証/可観測性

- 入力検証:
  - crate_name/version/path/keyword は文字列。path は "/" 始まりの相対パスを期待
  - 将来的にバリデーション関数を Tool 層に追加して安全性を向上させることが推奨

- SSRF対策:
  - docs.rs へのアクセスURLはフォーマットで固定ドメインを使用
  - ユーザ入力から直接ドメインを切り替えることはしない

- 可観測性（tracing）:
  - エラー時 tracing::error を記録（Repository層/DocsUseCase内）
  - 今後、tool名・入出力サイズ・レイテンシなどの構造化ログ追加が望ましい

---

## 11. テスト戦略（現状と推奨）

- 現状: src/use_case/docs.rs に最小の正常系テスト（fetch_all_items("serde", "latest")）
- 推奨追加:
  - エラー系: 404、CSSセレクタ不正、main-content 欠落
  - 境界値: keyword 空文字、巨大レスポンス、path 不正
  - ツール層: JSON直列化失敗時（unwrap 排除後）に正しく ErrorData を返すか
  - レポジトリ層: ネットワーク断・タイムアウト・ステータスコード 4xx/5xx

---

## 12. 拡張・改善提案（短期ロードマップ）

1) unwrap 排除（tool.rs 全ての JSON 直列化）  
2) retrieve_documentation_page の path 入力検証（"/"始まり必須）  
3) HTTPタイムアウト設定とリトライ戦略の導入（reqwest ClientBuilder）  
4) tantivy 一時インデックスのクリーンアップ保証（tempdir のスコープ厳密化）  
5) ロギングの粒度拡張（INFO/DEBUGで開始/終了・件数・レイテンシなど）

---

## 13. 付録：重要APIのミニ仕様（要約）

- **search_crate(keyword: String)**
  - 入力: keyword（クレート名に対する検索語）
  - 出力: JSON文字列コンテンツ（最大10件）
  - エラー: InitializeClient/CratesIoApi/SerializationError（改善後）

- **retrieve_documentation_index_page(crate_name, version)**
  - 入力: crate_name（例: "serde"）, version（例: "latest"）
  - 出力: Markdown相当テキスト1件
  - エラー: Http/ScraperSelectorParse/HtmlMainContentNotFound

- **retrieve_documentation_all_items(crate_name, version)**
  - 出力: JSON文字列コンテンツ群（Item）
  - エラー: Http/ScraperSelectorParse

- **search_documentation_items(crate_name, version, keyword)**
  - 出力: JSON文字列コンテンツ群（Item）
  - エラー: Http/CreateTempDir/FuzzySearch/FuzzySearchQueryParse

- **retrieve_documentation_page(crate_name, version, path)**
  - 入力: path は "/" 始まりの相対パス
  - 出力: Markdown相当テキスト1件
  - エラー: Http/ScraperSelectorParse/HtmlMainContentNotFound

---

## 14. 要点まとめ

- **クリーンアーキテクチャ**により層分離が明確。Use Case は Repository トレイトの抽象に依存、Interface Adapter（Handler/Tool）は MCP に適合。
- **非同期I/O中心**（tokio, reqwest, crates_io_api）。OnceCell によるクライアントキャッシュで効率化。
- **HTML→Markdown** 変換と **tantivy検索** により、docs.rs の情報を扱いやすく加工して返却。
- **統一エラー型**（thiserror）で運用の一貫性を維持。MCP ErrorData への変換対応。
- **安全性**: path の相対パス厳密化/unwrap削除/ログ強化などで、堅牢性が一層向上する。

---

### 参考コードスニペット（改善提案： unwrap 排除）

```rust
// 例: tool.rs 内での直列化安全化（search_crate）
let entities = self
    .crates_io_use_case
    .search_crate(&keyword)
    .await
    .map_err(|e| e.into())?
    .into_iter()
    .map(|c| serde_json::to_string(&c)
        .map(rmcp::model::Content::text)
        .map_err(|e| rmcp::ErrorData::new(rmcp::model::ErrorCode(1), format!("Serialization error: {}", e), None))
    )
    .collect::<Result<Vec<_>, rmcp::ErrorData>>()?;
```

---

## 15. 付録：ディレクトリ構造（抜粋）

```
src/
├─ main.rs
├─ handler.rs
├─ tool.rs
├─ resource.rs
├─ cache.rs
├─ error.rs
├─ entity/
│  ├─ crates_io.rs
│  └─ docs.rs
├─ record/
│  └─ crates_io.rs
├─ repository/
│  ├─ crates_io.rs
│  └─ http.rs
└─ use_case/
   ├─ crates_io.rs
   └─ docs.rs
```

---

以上が mcp-rust-docs の詳細設計です。実装者は本資料を参照し、コード・仕様・エラー・非同期処理の観点を把握してから拡張・保守を進めてください。🦀