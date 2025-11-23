# SPECIFICATION.md — MCP Rust Docs Server 外部仕様・設計仕様

本書は、MCP Rust Docs Server（実装名: mcp-rust-docs）の外部仕様および設計仕様を示します。AIエージェントが本サーバーを正しく利用するために必要な情報を網羅しています。

---

## 1. プロジェクトの概要と目的

- **目的**
  - AIエージェントが公式の Rust 情報源である **crates.io** と **docs.rs** に対して、安全かつ再現可能な方法でアクセスするための **MCPサーバー**を提供します。
  - エージェントはこのサーバーの **MCPツール**経由で、クレートメタデータ取得、ドキュメントトップページ取得、全項目一覧取得、項目のあいまい検索、特定ページ取得を行えます。

- **主な機能**
  - crates.io クレート検索（最大10件・関連度順）
  - docs.rs の **トップページ**（Index）取得
  - docs.rs の **全項目一覧**取得（struct/enum/trait/function など）
  - docs.rs の **項目のあいまい検索**（キーワード）
  - docs.rs の **特定ページ**取得（相対パス指定）
  - MCPの **Resource** として運用指示（Instruction）を提供

- **サーバー情報（ServerInfo）**
  - instructions: "Retrieve Rust crates and documents."
  - capabilities: Tools と Resources をサポート
  - implementation: name="mcp-rust-docs", version=ビルド時のCARGO_PKG_VERSION, title="mcp-rust-docs", website_url="https://github.com/46ki75/mcp-rust-docs"

---

## 2. ユーザーのユースケース（AIエージェントの利用シナリオ）

- 🧭 標準フロー
  1. 🔎 **search_crate** でクレートを探索（例: "serde"）
  2. 📚 **retrieve_documentation_index_page** でトップページを取得し構造を把握
  3. 🧠 **search_documentation_items** で項目候補をキーワード検索（見つからない場合は **retrieve_documentation_all_items** で全量取得）
  4. 📄 **retrieve_documentation_page** で特定ページの本文（Markdown相当）を取得
  5. 🏷️ 回答へ出典（crate_name/version/href など）を明示

- 🚀 ショートカット
  - 既に特定ページの **href** が分かっている場合は、直接 **retrieve_documentation_page** を呼び出す

- 📌 運用指示（Instruction Resource）
  - 一般知識を用いず、必ず **MCPツール**を使用して情報を取得することを強制

---

## 3. 提供するMCPツールの詳細仕様

以下の5ツールを提供します。各ツールは **rmcp::tool** として実装され、結果は **rmcp::model::CallToolResult** で返却されます。

### 3.1. 🔎 search_crate

- **説明**: crates.io 上でクレートを名前キーワードで検索し、メタデータ（要約）を取得します
- **検索仕様**: 最大 **10件**、**関連度順**（relevance）
- **入力パラメータ**

```json
{
  "keyword": "serde"
}
```

- **返却形式**
  - 複数件の **text コンテンツ**を返します
  - 各 text は JSON 文字列化されたクレート要約（CrateSummaryEntity）

```json
{
  "name": "serde",
  "description": "A framework for serializing and deserializing Rust data structures efficiently and generically.",
  "latest_stable_version": "1.0.210",
  "latest_version": "1.0.210",
  "downloads": 219384922,
  "created_at": "2016-05-16T00:00:00Z",
  "updated_at": "2024-09-30T12:34:56Z"
}
```

- **エラー処理**
  - crates.io API初期化失敗: "Failed to initialize client: ..."（ErrorCode=1）
  - crates.io API呼び出し失敗: "Network error: ..."（ErrorCode=1）
  - JSON直列化失敗: "Serialization error: ..."（ErrorCode=1）
  - 返却は error メッセージを含む rmcp::ErrorData

- **入力バリデーション**
  - keyword: 空文字列の場合、空の検索結果を返却
  - クライアント側での入力検証を推奨（null、長大文字列など）

- *補足*
  - 内部で crates_io_api のクライアントを使用（APIクライアント初期化に ~3秒のタイムアウト）
  - JSON直列化での障害は稀ですが、失敗時はエラー化されます

---

### 3.2. 📚 retrieve_documentation_index_page

- **説明**: docs.rs の指定クレート・バージョンの **トップページ**（Index）の本文を取得します
- **入力パラメータ**

```json
{
  "crate_name": "serde",
  "version": "latest"
}
```

- **返却形式**
  - 1件の **text コンテンツ**（Markdown相当）
  - HTMLから `section#main-content` 部分を抽出し Markdown に変換したテキスト

- **エラー処理**
  - HTTP失敗 / 非成功ステータス: "HTTP request error: ..."（ErrorCode=1）
  - セレクタ不正: "Failed to parse CSS Selector: ..."（ErrorCode=1）
  - main-contentが見つからない: "Failed to parse HTML: Element not found: section#main-content"（ErrorCode=1）

- *補足*
  - 取得URL: `https://docs.rs/{crate_name}/{version}/{crate_name}/index.html`
  - 本文は HTML → Markdown へ変換済み（html2md）

---

### 3.3. 🧾 retrieve_documentation_all_items

- **説明**: 指定クレート・バージョンの **全項目一覧**（struct/enum/trait/function など）を取得します
- **入力パラメータ**

```json
{
  "crate_name": "serde",
  "version": "latest"
}
```

- **返却形式**
  - 複数件の **text コンテンツ**を返します
  - 各 text は JSON 文字列化された項目（Item）

```json
{
  "type": "trait",
  "href": "/de/trait.Deserialize.html",
  "path": "Deserialize"
}
```

- **フィールド仕様（Item）**
  - **type**: 項目種別（例: trait, struct, enum, fn など）
  - **href**: ドキュメントへの相対URL（先頭 "/" 必須）
  - **path**: 項目の表示名（アンカーのテキスト。クレートによってはモジュールパスが含まれる場合あり）

- **エラー処理**
  - HTTP失敗 / 非成功ステータス: "HTTP request error: ..."（ErrorCode=1）
  - セレクタ不正: "Failed to parse CSS Selector: ..."（ErrorCode=1）

- *注意*
  - 大規模クレートでは返却件数が非常に多くなる可能性があります（クライアント側のパース・フィルタリングを推奨）

---

### 3.4. 🔍 search_documentation_items

- **説明**: 指定クレート・バージョンに対して **キーワードによる項目のあいまい検索** を行います
- **検索対象**
  - 項目の **path** フィールド（アンカーテキスト）に対して検索
  - 内部では一時tantivyインデックスを作成し **最大10件** を返却

- **入力パラメータ**

```json
{
  "crate_name": "serde",
  "version": "latest",
  "keyword": "Deserialize"
}
```

- **返却形式**
  - 複数件の **text コンテンツ**（各 text は JSON 文字列化された Item）

```json
{
  "type": "trait",
  "href": "/de/trait.Deserialize.html",
  "path": "Deserialize"
}
```

- **エラー処理**
  - HTTP失敗 / 非成功ステータス: "HTTP request error: ..."（ErrorCode=1）
  - tantivy / 検索クエリパース失敗: "..."（ErrorCode=1）
  - 一時ディレクトリ作成失敗: "Failed to create temporary directories."（ErrorCode=1）

- *補足*
  - 返却の **href** を **retrieve_documentation_page** の **path パラメータ**にそのまま渡して本文取得できます（先頭 "/" 必須）

---

### 3.5. 📄 retrieve_documentation_page

- **説明**: docs.rs の **特定ページ**を取得します
- **URL規約**
  - ベース: `https://docs.rs/{crate_name}/{version}/{crate_name}{path}`
  - 例: `https://docs.rs/serde/latest/serde/de/value/struct.BoolDeserializer.html`
  - 上記の場合、**path** は `"/de/value/struct.BoolDeserializer.html"`

- **入力パラメータ**

```json
{
  "crate_name": "serde",
  "version": "latest",
  "path": "/de/trait.Deserialize.html"
}
```

- **返却形式**
  - 1件の **text コンテンツ**（Markdown相当）

- **エラー処理**
  - HTTP失敗 / 非成功ステータス: "HTTP request error: ..."（ErrorCode=1）
  - セレクタ不正: "Failed to parse CSS Selector: ..."（ErrorCode=1）
  - main-contentが見つからない: "Failed to parse HTML: Element not found: section#main-content"（ErrorCode=1）
  - パス形式不正: "Invalid path: must start with '/'"（ErrorCode=1）

- **入力バリデーション**
  - path: 必ず "/" で始まる相対パスである必要があります
  - クライアント側でのパス形式検証を強く推奨

- *重要*
  - **path** は必ず **"/" で始まる相対パス**を指定してください
  - **retrieve_documentation_all_items / search_documentation_items** で得られた **href** をそのまま渡すのが推奨です

---

## 4. 各ツールの入力パラメータ・出力形式・エラーハンドリング（まとめ）

- **入力パラメータの型**
  - すべて JSON オブジェクト（rmcp::schemars::JsonSchema に準拠）
  - search_crate: { keyword: String }
  - retrieve_documentation_index_page: { crate_name: String, version: String }
  - retrieve_documentation_all_items: { crate_name: String, version: String }
  - search_documentation_items: { crate_name: String, version: String, keyword: String }
  - retrieve_documentation_page: { crate_name: String, version: String, path: String }

- **出力形式**
  - MCP **CallToolResult**（成功時）
  - コンテンツは **text** が基本（Markdown相当の本文、または **文字列化されたJSON**）
  - JSONは「文字列化」されているため、クライアント側で **JSONパース** が必要

- **エラーハンドリング**
  - 失敗時は rmcp::ErrorData（ErrorCode=1、messageに詳細）
  - 代表的なエラー種別:
     - **InitializeClient**（APIクライアント初期化失敗）
     - **CratesIoApi**（crates.io 通信失敗）
     - **Http**（docs.rs へのHTTP失敗、ステータス非成功を含む）
     - **ScraperSelectorParse**（CSSセレクタパース失敗）
     - **HtmlMainContentNotFound**（main-content抽出失敗）
     - **CreateTempDir**（一時ディレクトリ作成失敗）
     - **Tantivy / QueryParserError**（あいまい検索のバックエンドエラー）
     - **SerializationError**（JSON直列化失敗）
     - **InvalidPath**（パス形式不正："/"で始まらない）

---

## 5. 提供するMCPリソースの仕様

- **Instruction リソース**（URI: `str://mcp-rust-docs/instruction`）
  - **name**: "Instruction"
  - **mime_type**: "text/plain"
  - **説明**: AIエージェントが Rust 関連クエリへ回答する際の **必須運用指示**。一般知識での回答を禁止し、**MCPツール**の利用を強制します
  - **contents**: 以下テキスト（抜粋）

```markdown
# MCP Tool Usage Instructions

**You must use MCP tools provided by this server** to search and retrieve Rust documentation.

## Important

- Do NOT provide general knowledge answers about Rust crates or documentation
- Always use these MCP tools for current, accurate crate information
- All data comes from latest docs.rs content
```

- **取得方法**
  - list_resources → read_resource で上記URIを指定

---

## 6. ユーザー操作と期待される応答の関係

- **操作フローとレスポンス例**

1) クレート検索（最大10件・関連度順）
```json
{
  "tool": "search_crate",
  "params": { "keyword": "serde" }
}
```
→ 複数の text コンテンツ（各行は JSON 文字列）を返却

2) トップページ（Index）取得
```json
{
  "tool": "retrieve_documentation_index_page",
  "params": { "crate_name": "serde", "version": "latest" }
}
```
→ 1件の text（Markdown相当）を返却

3) 項目のあいまい検索（pathのアンカーテキストを対象）
```json
{
  "tool": "search_documentation_items",
  "params": { "crate_name": "serde", "version": "latest", "keyword": "Deserialize" }
}
```
→ 複数の text コンテンツ（各行は JSON 文字列 Item）を返却

4) 特定ページ取得（hrefをpathとして渡す）
```json
{
  "tool": "retrieve_documentation_page",
  "params": { "crate_name": "serde", "version": "latest", "path": "/de/trait.Deserialize.html" }
}
```
→ 1件の text（Markdown相当）を返却

5) 全項目一覧取得（キーワード検索のフォールバック）
```json
{
  "tool": "retrieve_documentation_all_items",
  "params": { "crate_name": "tokio", "version": "latest" }
}
```
→ 複数の text コンテンツ（各行は JSON 文字列 Item）を返却

- **期待されるクライアント側の処理**
  - JSON文字列の **パース** と **フィルタリング**
  - **出典の明示**（crate_name, version, href/path）
  - **運用指示（Instruction）**の遵守

---

## 7. 制約事項と前提条件

- **データソース**
  - crates.io と docs.rs の **公式情報のみ**を利用（真偽不明な非公式ソースは利用しない）

- **バージョン指定**
  - "latest" を指定可能
  - 特定バージョンは `"1.0.0"` のように **完全表記**で指定

- **検索対象**
  - 項目検索は **path（アンカーの表示テキスト）** に対して行われます
  - クレートによってはアンカー表示に **モジュールパス**が含まれる場合もありますが、含まれないケースもあり得ます

- **パス指定**
  - **retrieve_documentation_page** の **path** は **相対パス（"/"始まり）**である必要があります
  - **retrieve_documentation_all_items / search_documentation_items** の **href** をそのまま渡すのが最も安全

- **結果件数**
  - **search_crate**: 最大10件
  - **search_documentation_items**: 上位10件（tantivyによる検索）

- **エラー時の推奨対策**
  - バージョンを "latest" に変更して再試行
  - キーワードの再調整（単数/複数、略称/正式名など）
  - まずトップページからモジュール構造を辿る（Index の "Modules" を参照）

- **拡張/負荷**
  - 大規模クレートでは **retrieve_documentation_all_items** の返却が非常に大きくなる可能性があるため、まず **search_documentation_items** を推奨
  - ネットワークや外部サービスの負荷・レートリミットに配慮し、適切にリトライやバックオフを実施してください

- **セキュリティ / 状態管理**
  - 状態を持たない設計（リソースやツール結果は全て stateless）
  - 機密情報の取り扱いはありません（公開情報のみ）

---

## 付録（データスキーマの参考）

- **CrateSummaryEntity**
```json
{
  "name": "String",
  "description": "String | null",
  "latest_stable_version": "String | null",
  "latest_version": "String",
  "downloads": "u64",
  "created_at": "String (ISO8601)",
  "updated_at": "String (ISO8601)"
}
```

- **Item**
```json
{
  "type": "String",
  "href": "String | null",
  "path": "String | null"
}
```

---

## 参考（ServerInfo 返却の目安）

- **instructions**: "Retrieve Rust crates and documents."
- **capabilities**: tools/resources を有効化
- **implementation**:
  - name: "mcp-rust-docs"
  - version: CARGO_PKG_VERSION
  - title: "mcp-rust-docs"
  - website_url: "https://github.com/46ki75/mcp-rust-docs"

---

## ✅ 最重要ポイント（運用指示の要約）

- **必ず MCPツールを利用**し、一般知識ではなく **公式情報**（crates.io / docs.rs）を根拠とする
- **検索結果のJSON文字列はクライアントでパース**して利用
- **出典**（crate_name / version / href など）を回答へ明示
- **パス指定は "/" 始まりの相対パス**（href をそのまま流用）

---