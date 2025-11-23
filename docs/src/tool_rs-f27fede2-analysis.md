# src/tool.rs コードレビュー

## Executive Summary

### 対象ファイル

- src/tool.rs (157行, 6関数, 6エクスポート)

### 要約

- crates.io/ docs.rs APIラッパとして機能し、パラメータ/結果のデータ契約も良好。型安全かつ非同期設計で、外部I/Oとエラー伝播も明示的に設計されている。[rmcp]等、外部ライブラリ依存が多いが設計と型付け、パターンは妥当。I/O境界でのエラー伝播は統一されているが、エラーメッセージと内部エラー分類は一貫性に課題。データフローは明快だが、将来のメンテ性・テスト観点ではユニットテストやリトライ/レートリミット、認可・追跡性の追加が推奨される。

### 主要公開API

- Handler::search_crate (crates.io 検索, async)
- Handler::retrieve_documentation_index_page (docs.rs index取得, async)
- Handler::retrieve_documentation_all_items (docs.rs:全アイテム一覧, async)
- Handler::search_documentation_items (docs.rs:アイテム検索, async)
- Handler::retrieve_documentation_page (docs.rs:個別ページ取得, async)
- Handler::new (依存性注入コンストラクタ)

### 公開クラス・構造体

| 種別   | 名前                                | 公開範囲 | 責務                             | 複雑度 |
|--------|-------------------------------------|----------|----------------------------------|--------|
| Struct | SearchCrateParams                   | pub      | crates.io 検索クエリDTO         | Low    |
| Struct | SearchCrateResult                   | pub      | crates.io 検索結果DTO           | Low    |
| Struct | RetrieveDocumentationIndexPageParams| pub      | docs.rs Index取得用パラメータ   | Low    |
| Struct | RetrieveDocumentationPageParams     | pub      | docs.rs ページ取得用パラメータ  | Low    |
| Struct | SearchDocumentationItemsParams      | pub      | docs.rs 内部検索クエリDTO       | Low    |
| Impl   | Handler (tool_router)               | pub      | 各種APIディスパッチ             | Medium |

### 重要指摘事項

| 深刻度   | 件数 | 主な内容                                  | 推定修正工数 |
|----------|------|-------------------------------------------|--------------|
| Critical | 0    | なし                                     | 0h           |
| High     | 1    | エラー分類/メッセージ一貫性               | 2h           |
| Medium   | 2    | テストコード不足・運用性(監査/追跡性)      | 3h           |
| Low      | 2    | ロギング、可観測性ドキュメント不足         | 1h           |

---

## コンポーネントインベントリー

| 種別   | 名前                                 | 可視性 | 責務                               | 依存先                       | 複雑度 | LOC | 備考                         |
|--------|--------------------------------------|--------|------------------------------------|------------------------------|--------|-----|------------------------------|
| Struct | SearchCrateParams                    | pub    | crates.io クエリ                   | serde, rmcp                  | Low    | 4   | Keywordのみ                  |
| Struct | SearchCrateResult                    | pub    | crates.io 結果                     | serde, rmcp                  | Low    | 7   | Option使用                   |
| Struct | RetrieveDocumentationIndexPageParams | pub    | docs.rs インデックス用パラメータ   | serde, rmcp                  | Low    | 5   | crate名+version             |
| Struct | RetrieveDocumentationPageParams      | pub    | docs.rs ページ取得用パラメータ     | serde, rmcp                  | Low    | 7   | +path                       |
| Struct | SearchDocumentationItemsParams       | pub    | docs.rs 内部検索クエリ             | serde, rmcp                  | Low    | 7   | +keyword                    |
| Impl   | Handler (tool_router)                | pub    | APIルーティング・依存注入          | use_case, resource, rmcp     | Med    | 110 | 各API: async, Result型戻り値 |

---

## 1. 仕様準拠評価（不明）

### 準拠状況サマリ

- **準拠率**: 不明（仕様書未提供）
- **主要逸脱**: このチャンクには現れない

### 詳細対応表

| 仕様項目 | 実装状況 | 該当箇所            | 備考          |
|----------|----------|---------------------|---------------|
| 仕様不明  | 不明     | このチャンク外      | -             |

---

## 2. アーキテクチャと設計評価

### モジュール構成

- 単一ファイルでDTOとルータ実装を集約。API入力/出力型は強く型付けされ、依存注入方式でuse_caseレイヤに処理委譲 (src/tool.rs:L1-157)。

### 設計原則の遵守

- **KISS**: シンプルなDTO・ルーティング、冗長なし。
- **DRY**: コード重複小 (各種APIで似た流れだが、引数型のみ異なる)。
- **SOLID**: Handlerが単一責務。パラメータ/DTOも関数単位で分離。
- **言語慣例**: Rust流の所有権・型安全遵守。

### API一覧表

| API名                                         | シグネチャ                                                                           | 目的                          | Time    | Space |
|-----------------------------------------------|--------------------------------------------------------------------------------------|-------------------------------|---------|-------|
| Handler::new                                 | fn new(CratesIoUseCase, DocsUseCase) -> Handler                                      | 依存注入                      | O(1)    | O(1)  |
| Handler::search_crate                        | async fn(&self, Parameters<SearchCrateParams>) -> Result<CallToolResult, ErrorData>  | クレート名検索                | O(n)    | O(n)  |
| Handler::retrieve_documentation_index_page   | async fn(&self, Parameters<RetrieveDocumentationIndexPageParams>) -> ...             | docs.rs インデックス取得      | O(1)    | O(1)  |
| Handler::retrieve_documentation_all_items    | async fn(&self, Parameters<RetrieveDocumentationIndexPageParams>) -> ...             | 全アイテム取得（fallback用）  | O(n)    | O(n)  |
| Handler::search_documentation_items          | async fn(&self, Parameters<SearchDocumentationItemsParams>) -> ...                   | docs.rs 内部キーワード検索    | O(n)    | O(n)  |
| Handler::retrieve_documentation_page         | async fn(&self, Parameters<RetrieveDocumentationPageParams>) -> ...                  | docs.rs個別ページ取得         | O(1)    | O(1)  |

> 詳細は関数詳細テンプレート節で全APIを記載。

---

## 3. コード品質分析

### 複雑度メトリクス

| 指標                    | 測定値 | 推奨値 | 評価 |
|------------------------|--------|--------|------|
| 循環的複雑度（最大）   | 3      | <10    | ✅   |
| ネストの深さ（最大）   | 2      | <4     | ✅   |
| 関数/メソッド行数（最大） | 19   | <50    | ✅   |

### 主要な品質問題

1. **エラー分類・メッセージのばらつき**
   - **影響**: ユーザー向けエラーが一律で分かりにくくなる。内部障害箇所特定が困難。
   - **修正案**: ErrorDataでより詳細な分類・trace付与（tool.rs:L40-110）。

2. **テスト不足**
   - **影響**: 回帰バグ検知困難、改修リスク上昇。
   - **修正案**: ユニット/統合テスト追加（このチャンクには現れない）。

---

## 4. 正確性とエラー処理

### エラー処理戦略

- すべてResult<..., ErrorData>準拠。I/O/外部API失敗はmap_errで伝播一貫。
- ユーザー入力バリデーションはDTO型で担保、実装的には未チェック（空文字など要追加）。
- エラー内容はenum分解せず、into()変換のみで抽象度高（tool.rs:L49-, L97-, L116-等）。

### エッジケース対応

| ケース         | 現状     | リスク  | 推奨対応    |
|----------------|----------|---------|-------------|
| 空/無効入力    | 未対策   | High    | DTO/引数層で検証 |
| 外部API失敗    | Result   | Medium  | 詳細分類/再試行 |
| 時刻/型変換失敗| panic可  | Medium  | シリアライズ失敗補足 |

---

## 5. パフォーマンスとスケーラビリティ

### 性能特性

- 主要APIは非同期・I/Oバウンド設計
- n = 結果数（クレート数/アイテム数）で処理量に比例

### 最適化の機会

1. シリアライズ（serde_json::to_string）のpanic回避/エラー扱い明確化
2. レスポンス圧縮/ストリーム化（nが大きい場合）

---

## 6. セキュリティ評価

### セキュリティチェック

| 項目         | 状態 | 深刻度 | 対策                    |
|--------------|------|--------|-------------------------|
| 入力検証     | ⚠️   | Medium | 空/長大/不正値検証      |
| 認証・認可   | ❌   | High   | Route単位で要考慮       |
| データ保護   | ✅   | -      | セキュアな型変換        |
| 依存関係     | ⚠️   | Medium | rmcp/serde等安全化必須  |

---

## 7. テスト戦略

### カバレッジ評価

- **現状**: 0%（テストコードこのチャンクには現れない）
- **目標**: 80%以上
- **不足領域**: エラーパス、並列I/O境界

### 推奨テスト追加

- 入力バリデーションケース
- 外部APIエラー（UseCase失敗）時の伝播
- 取得結果レスポンス構造検証

---

## 8. 保守性と可読性

### 改善ポイント

- **命名規則**: Rust慣例・一貫
- **ドキュメント**: 関数Doc抜粋良、サブモジュール設計のドキュメント追加推奨
- **構造**: ファイル分割は不要だが各DTOの分離も許容

---

## 9. 改善ロードマップ

| 優先度 | 項目                | 工数 | 期限目安 |
|--------|---------------------|------|----------|
| 1      | 入力バリデーション  | 1h   | 即座     |
| 2      | エラー型一貫        | 2h   | 1週間    |
| 3      | テスト追加          | 3h   | 1ヶ月    |
| 4      | ロギング観測性      | 1h   | 四半期   |

---

## 10. 良い実践の認識

- DTO/UseCaseの完全分離
- 非同期関数全面活用、所有権安全なAPI設計
- 入出力整形の一貫
- rmcp属性による自動ルーティング

---

## 関数詳細テンプレート適用（主要API全網羅）

### Handler::new

| 項目                    | 内容                                                                                            |
|-------------------------|-------------------------------------------------------------------------------------------------|
| シグネチャ              | pub fn new(crates_io_use_case: CratesIoUseCase, docs_use_case: DocsUseCase) -> Self            |
| 可視性                  | pub                                                                                            |
| 型パラメータ/制約       | なし                                                                                           |
| ミュータビリティ        | 不変 (&self必要なし)                                                                            |
| 所有権/参照の変化       | 依存をムーブし,構造体所有                                                                       |
| 入力引数                | クレート,ドキュメント用UseCaseインスタンス                                                     |
| 戻り値                  | Handler                                                                                        |
| 事前条件                | UseCaseが有効な初期化済であること                                                              |
| 事後条件                | Handlerが完全な状態で初期化                                                                    |
| 不変条件                | resource_map, tool_routerともに整合                                                            |
| 副作用                  | なし                                                                                           |
| パニック/例外           | なし                                                                                           |
| エラーモデル            | なし                                                                                           |
| 計算量                  | O(1)                                                                                           |
| 並行性/スレッド安全性   | 初期化時のみ利用                                                                              |
| 安全でない操作          | なし                                                                                           |
| 呼び出し関係            | N/A                                                                                            |
| 使用例                  | ```rust<br>let handler = Handler::new(ci, di);<br>```                                         |
| エッジケース            | UseCase未初期化（panic）                                                                      |
| 根拠行番号              | src/tool.rs:L41-49                                                                             |


---

### Handler::search_crate

| 項目                    | 内容                                                                                            |
|-------------------------|-------------------------------------------------------------------------------------------------|
| シグネチャ              | async fn search_crate(&self, Parameters<SearchCrateParams>) -> Result<CallToolResult, ErrorData>|
| 可視性                  | pub（間接的にrmcp::toolによる公開）                                                             |
| 型パラメータ/制約       | なし                                                                                           |
| ミュータビリティ        | &self（不変借用）                                                                              |
| 所有権/参照の変化       | 引数データは所有、内部で参照                                                                    |
| 入力引数                | Parameters(SearchCrateParams { keyword: String })                                               |
| 戻り値                  | Result<CallToolResult, ErrorData>                                                              |
| 事前条件                | keyword: 非空                                                                                   |
| 事後条件                | crates.io検索結果をContentに変換して返却                                                        |
| 不変条件                | N/A                                                                                            |
| 副作用                  | crates_io_use_case.search_crate()の非同期I/O                                                    |
| パニック/例外           | serde_json::to_string.unwrap()失敗（応じてpanic）                                              |
| エラーモデル            | 外部API or シリアライズ失敗はErrorDataに変換                                                   |
| 計算量                  | O(n)（n=対象クレート数）                                                                       |
| 並行性/スレッド安全性   | 非同期ポリシー、競合なし                                                                       |
| 安全でない操作          | なし                                                                                           |
| 呼び出し関係            | crates_io_use_case.search_crate                                                                 |
| 使用例                  | ```rust<br>handler.search_crate(Parameters(params)).await;<br>```                              |
| エッジケース            | 空keyword，APIエラー，検索結果0件                                                              |
| 根拠行番号              | src/tool.rs:L50-61                                                                             |

---

### Handler::retrieve_documentation_index_page

| 項目                    | 内容                                                                                            |
|-------------------------|-------------------------------------------------------------------------------------------------|
| シグネチャ              | async fn retrieve_documentation_index_page(&self, Parameters<RetrieveDocumentationIndexPageParams>) -> Result<CallToolResult, ErrorData>|
| 可視性                  | pub（間接的にrmcp::toolによる公開）                                                             |
| 型パラメータ/制約       | なし                                                                                           |
| ミュータビリティ        | &self                                                                                          |
| 所有権/参照の変化       | 引数所有、内部参照                                                                             |
| 入力引数                | Parameters(RetrieveDocumentationIndexPageParams { crate_name, version })                        |
| 戻り値                  | Result<CallToolResult, ErrorData>                                                              |
| 事前条件                | crate_name/versionが有効                                                                      |
| 事後条件                | ドキュメントインデックスHTMLを返却                                                             |
| 不変条件                | N/A                                                                                            |
| 副作用                  | docs_use_case.fetch_document_index_page                                                         |
| パニック/例外           | なし                                                                                           |
| エラーモデル            | 外部APIのErrorもErrorDataへ                                                                    |
| 計算量                  | O(1)                                                                                           |
| 並行性/スレッド安全性   | 非同期I/O                                                                                      |
| 安全でない操作          | なし                                                                                           |
| 使用例                  | ```rust<br>handler.retrieve_documentation_index_page(Parameters(p)).await;<br>```              |
| エッジケース            | 存在しないcrate/version, 外部docs.rs落ち時                                                     |
| 根拠行番号              | src/tool.rs:L64-77                                                                             |

---

### Handler::retrieve_documentation_all_items

| 項目                    | 内容                                                                                            |
|-------------------------|-------------------------------------------------------------------------------------------------|
| シグネチャ              | async fn retrieve_documentation_all_items(&self, Parameters<RetrieveDocumentationIndexPageParams>) -> Result<CallToolResult, ErrorData>|
| 可視性                  | pub（rmcp::tool）                                                                              |
| 型パラメータ/制約       | なし                                                                                           |
| ミュータビリティ        | &self                                                                                          |
| 所有権/参照の変化       | 引数所有、内部参照                                                                             |
| 入力引数                | Parameters(RetrieveDocumentationIndexPageParams { crate_name, version })                        |
| 戻り値                  | Result<CallToolResult, ErrorData>                                                              |
| 事前条件                | crate_name/version有効                                                                         |
| 事後条件                | アイテムリスト(serde_jsonで文字列化)                                                          |
| 不変条件                | N/A                                                                                            |
| 副作用                  | docs_use_case.fetch_all_items()                                                                 |
| パニック/例外           | serde_json::to_string.unwrap()でpanic可                                                        |
| エラーモデル            | 上記失敗でErrorData                                                                            |
| 計算量                  | O(n)                                                                                           |
| 並行性/スレッド安全性   | 非同期I/O                                                                                      |
| 安全でない操作          | なし                                                                                           |
| 使用例                  | ```rust<br>handler.retrieve_documentation_all_items(Parameters(p)).await;<br>```               |
| エッジケース            | アイテム0件, fetch失敗                                                                         |
| 根拠行番号              | src/tool.rs:L80-94                                                                             |

---

### Handler::search_documentation_items

| 項目                    | 内容                                                                                            |
|-------------------------|-------------------------------------------------------------------------------------------------|
| シグネチャ              | async fn search_documentation_items(&self, Parameters<SearchDocumentationItemsParams>) -> Result<CallToolResult, ErrorData>|
| 可視性                  | pub（rmcp::tool）                                                                              |
| 型パラメータ/制約       | なし                                                                                           |
| ミュータビリティ        | &self                                                                                          |
| 所有権/参照の変化       | 引数所有、内部参照                                                                             |
| 入力引数                | Parameters(SearchDocumentationItemsParams { crate_name, version, keyword })                     |
| 戻り値                  | Result<CallToolResult, ErrorData>                                                              |
| 事前条件                | 各パラメータ有効                                                                              |
| 事後条件                | 検索結果をContentベクタで返却                                                                  |
| 不変条件                | N/A                                                                                            |
| 副作用                  | docs_use_case.search_items (I/O)                                                                |
| パニック/例外           | serde_json::to_string.unwrap()でpanic可能                                                      |
| エラーモデル            | 上記失敗時ErrorData                                                                             |
| 計算量                  | O(n)                                                                                           |
| 並行性/スレッド安全性   | 非同期I/O                                                                                      |
| 安全でない操作          | なし                                                                                           |
| 使用例                  | ```rust<br>handler.search_documentation_items(Parameters(p)).await;<br>```                     |
| エッジケース            | keyword不一致, 結果0件                                                                         |
| 根拠行番号              | src/tool.rs:L97-111                                                                            |

---

### Handler::retrieve_documentation_page

| 項目                    | 内容                                                                                            |
|-------------------------|-------------------------------------------------------------------------------------------------|
| シグネチャ              | async fn retrieve_documentation_page(&self, Parameters<RetrieveDocumentationPageParams>) -> Result<CallToolResult, ErrorData>|
| 可視性                  | pub（rmcp::tool）                                                                              |
| 型パラメータ/制約       | なし                                                                                           |
| ミュータビリティ        | &self                                                                                          |
| 所有権/参照の変化       | 引数所有、内部参照                                                                             |
| 入力引数                | Parameters(RetrieveDocumentationPageParams { crate_name, version, path })                       |
| 戻り値                  | Result<CallToolResult, ErrorData>                                                              |
| 事前条件                | 指定パスが有効（ページ実体が存在）                                                            |
| 事後条件                | 指定ドキュメントHTML返却                                                                      |
| 不変条件                | N/A                                                                                            |
| 副作用                  | docs_use_case.fetch_document_page (I/O)                                                        |
| パニック/例外           | なし                                                                                           |
| エラーモデル            | fetch失敗時ErrorData                                                                           |
| 計算量                  | O(1)                                                                                           |
| 並行性/スレッド安全性   | 非同期I/O                                                                                      |
| 安全でない操作          | なし                                                                                           |
| 使用例                  | ```rust<br>handler.retrieve_documentation_page(Parameters(p)).await;<br>```                    |
| エッジケース            | 不正path/ページなし、docs.rs落ち                                                               |
| 根拠行番号              | src/tool.rs:L114-136                                                                           |

---

## データフロー・呼び出し関係

```mermaid
flowchart TD
  "search_crate (L50-61)" --> "crates_io_use_case.search_crate"
  "retrieve_documentation_index_page (L64-77)" --> "docs_use_case.fetch_document_index_page"
  "retrieve_documentation_all_items (L80-94)" --> "docs_use_case.fetch_all_items"
  "search_documentation_items (L97-111)" --> "docs_use_case.search_items"
  "retrieve_documentation_page (L114-136)" --> "docs_use_case.fetch_document_page"
```
> 上記の図はsrc/tool.rsの主要API各関数からユースケース呼び出しのデータフローを表す。

---

## Edge Cases, Bugs, and Security

### メモリ安全性

- Rustの所有権/Borrowチェックで保証。不変借用(&self)で共有。

### インジェクション

- 外部ストリング値（path/keyword等）について検証は未実装。パストラバーサルやサーバーサイドリクエストフォージェリのリスクもゼロでない。

### 認証・認可

- ルーティングレイヤは位置していない。公開API認可は呼び出し元でカバー必須。

### Secrets/Log漏洩

- パスワード等は保持しない。ログ化処理は未記載。

### 並行性

- &selfでのI/Oのみ。ミューテックス・共有可変は不要。

---

## Contracts/Edge Cases

- Parameters構造体がそのまま値移動
- serde_json::to_string失敗時のunwrap危険性
- 空input・不正inputは呼び出し前提になっているが内部検証必須
- 外部APIのI/O Errorが予期せぬ形式で返る可能性大

---

## テスト/可観測性

- 各メソッドの境界テスト、型不整合、文字列化失敗系テストが必須
- ログ・トレース用の埋め込みは将来的には必要

---

## Performance/Scalability

- 全APIがI/O境界にあり、async設計
- アイテム/クレート数大時（n→1000以上）でレスポンス肥大化リスクあり
- O(n)メモリアロケーション多数だが、個々でベクタのみ返却。ストリーミングレスポンスも今後有効

---

## Tradeoffs/設計判断

| 判断事項                  | 採用理由                      | 影響範囲         | リスク                         |
|--------------------------|-------------------------------|------------------|-------------------------------|
| 非同期I/O                | レイテンシ低減                | API全体          | デバッグ難、I/O失敗パス複雑化   |
| serde_jsonベース          | 互換性/標準依存                | 全エンコード     | エラー時panic可                 |
| DTO/UseCase分離           | 責務明確化・再利用性            | 全ルーティング   | コード量増                     |

---

## まとめ

堅牢な型安全API設計であり、テストと監査・バリデーション層を拡充することでより高い品質・安心感が得られます。I/Oエラー、panic伝播リスクを意識し、今後のバージョンではバリデーション強化・可観測性（ログ・メトリクス）・エラー詳細化の取組みが推奨されます。