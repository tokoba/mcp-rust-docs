# src/use_case/docs.rs コードレビュー

## Executive Summary

### 対象ファイル
- src/use_case/docs.rs（183行, 7関数, 1構造体, 公開API多数）

### 要約

- DocsUseCaseはdocs.rsからドキュメントページ・アイテム一覧を取得し、HTML解析と全文検索を担うユースケース層。
- HTML解析にscraper、全文検索にtantivy、非同期HTTP通信にArc<dyn HttpRepository>を利用。
- エラー設計は独自crate::error::Errorで統一、Rust標準や外部クレートの例外をラップする堅牢な設計。
- async/awaitおよびSend/Sync境界を正しく考慮。
- parse/select/zipなどRust/クレート特有のAPI活用によりコード効率が高い。
- テストコードは最低限存在（fetch_all_itemsの正常系のみ）。
- 可観測性要素（tracingログ）がエラー箇所に限定されており、運用上の十分な情報は未整備。
- 一時ディレクトリや全文検索エンジンの短期利用が多いがリソースリーク対策は未明示。
- クラス・関数やデータ契約の責務分離は良好。エラー・境界値への設計上の考慮は十分だが、未検証領域も存在。
- 性能面は現状十分（短期インデックス、限定件数検索）。データ量100倍以上でのスケール限界の記載はなし。
- テストと観測の強化、不正データ対策、リファクタリング機会多数。

### 主要公開API

- fetch_document_index_page（main HTML→Markdown変換, async, エラー伝播）
- fetch_document_page（任意path HTML→Markdown変換, async）
- fetch_all_items（all.htmlから全アイテム抽出, async）
- search_items（全文検索でアイテム絞り込み, async）
- extract_main_content（セレクタ指定でmain-content抽出, sync）
- parse_all_items（main-content > h3/ul/aからItem集合構築, sync）

### 公開クラス・構造体

| 種別 | 名前         | 公開範囲 | 責務                                                         | 複雑度 |
|------|--------------|----------|--------------------------------------------------------------|--------|
| Struct | DocsUseCase | pub      | http_repository管理、ドキュメント取得・変換APIを提供         | Medium |
| Function | fetch_document_index_page | pub | Indexページ取得、main-content HTML抽出、Markdown変換 | Medium |
| Function | fetch_document_page | pub | 任意パスのページ取得、main-content抽出、Markdown変換   | Medium |
| Function | fetch_all_items | pub | all.html解析・Item集合抽出                                   | High |
| Function | search_items | pub | Item全文検索（tantivy一時DBを構築しキーワード検索）          | High |

### 重要指摘事項

| 深刻度 | 件数 | 主な内容                              | 推定修正工数 |
|--------|------|--------------------------------------|--------------|
| Critical | 0件 | 重大な脆弱性・データ破壊の懸念なし    | 0h           |
| High    | 2件 | テスト網羅性不足, リソースリーク未対策 | 4h           |
| Medium  | 4件 | 一部エラー設計, 観測/運用不足, 境界値不備 | 6h        |
| Low     | 5件 | 命名一貫性/冗長処理/ロギング拡張余地      | 3h           |


---

## 1. 仕様準拠評価（不明:仕様書未提供）

### 準拠状況サマリ

- **準拠率**: 不明（仕様書がこのチャンクには現れない）
- **主要逸脱**: 不明

### 詳細対応表

| 仕様項目 | 実装状況 | 該当箇所 | 備考     |
|----------|----------|----------|----------|
| 不明     | 不明     | 不明     | このチャンクには仕様書未登場 |

---

## 2. アーキテクチャと設計評価

### モジュール構成

- Use Case層: DocsUseCase（HTTP外部API＋HTML解析＋全文検索の責務集約）
- インフラ依存: Arc<dyn HttpRepository>（DI/抽象化）

### 設計原則の遵守

- **KISS**: 過度な複雑性なし
- **DRY**: コード重複なし（セレクタ処理等抽象化済み）
- **SOLID**: 責務分離と抽象化良好（HttpRepository, Error型, Item型）
- **Rust慣例**: Result主導, エラーラップ, Arc+Dyn Trait活用

### API詳細説明

#### 代表的関数詳細: `search_items`
| 項目 | 内容 |
|------|------|
| **シグネチャ** | pub async fn search_items(&self, crate_name: &str, version: &str, keyword: &str) -> Result<Vec<Item>, Error> |
| **可視性** | public (`pub`) |
| **型パラメータ/制約** | なし |
| **ミュータビリティ** | &self（不変借用） |
| **所有権/参照の変化** | Items:一度クローン（fetch_all_items経由）、tantivy DB temporary ownership |
| **入力引数** | crate_name: &str, version: &str, keyword: &str（null/空の扱い未対策）|
| **戻り値** | Result<Vec<Item>, Error>（失敗時はError型返却）|
| **事前条件** | HTTP API正常, HTML構造崩れていないこと |
| **事後条件** | 検索結果最大10件, エラーはError型にラップ |
| **不変条件** | tantivy Index一時構築のみ, Item型の型保証 |
| **副作用** | HTTP通信、FS一時ディレクトリ作成、ローカルDB構築（tempdir）|
| **パニック/例外** | tantivy APIのunwrap/expectなし、Error変換済み |
| **エラーモデル** | crate::error::Error（独自、全箇所でラップ）|
| **計算量** | O(n)（item数に比例）、index構築O(n)、検索O(logn) |
| **並行性/スレッド安全性** | Arc+Send/Sync/Tokio非同期 |
| **安全でない操作** | unsafeなし（このチャンクには現れない）|
| **呼び出し関係** | fetch_all_items → parse_all_items → search_items内部tantivy処理 |
| **使用例** |
```rust
let result = use_case.search_items("serde", "latest", "Serialize").await;
// 期待: Serdeの「Serialize」関連アイテムが最大10件返る
```
| **エッジケース** | 空keyword/空item/HTML構造不正/ネットワーク障害/ファイル属性エラー|
| **根拠行番号** | src/use_case/docs.rs:L91-182 |

#### mermaid データフロー図
```mermaid
flowchart TD
  "search_items" -->|"fetch_all_items"| "parse_all_items"
  "search_items" -->|"tantivy temp DB構築"| "IndexWriter"
  "search_items" -->|"keyword検索"| "TopDocs"
  "search_items" -->|"Item復元"| "result_items"
```
上記の図は `search_items` (L91-182) の主要分岐を示す。

---

## 3. コード品質分析

### 複雑度メトリクス

| 指標                  | 測定値 | 推奨値 | 評価    |
|----------------------|-------|-------|---------|
| 循環的複雑度（最大） | 7     | <10   | ✅      |
| ネストの深さ（最大） | 4     | <4    | ⚠️      |
| 関数/メソッド行数（最大） | 91 | <50   | ⚠️      |

### 主な品質問題

1. **大きな関数: search_items**
   - **影響**: 読みやすさ・保守性低下、テスト困難
   - **修正案**: tantivy DB構築・検索・Item復元を小関数へ分離
2. **テスト網羅不足**
   - **影響**:障害時検知不可
   - **修正案**: 境界値・エラーパス追加
3. **一時ディレクトリクリーンアップ未保証（tempdir）**
   - **影響**:FS一時ファイルリーク、リソース枯渇の懸念
   - **修正案**: drop管理・明示的削除
4. **ロギング不足**
   - **影響**:障害時解析困難
   - **修正案**:開始/終了通知、リクエスト失敗時ログ

---

## 4. 正確性とエラー処理

### エラー処理戦略

- サンプル中はmap_errで他クレートエラー→独自Error型変換
- レポジトリの抽象化により各種Errorをラップ
- tracingによるエラー時ログ出力
- unwrap/expect不使用（安全）

### エッジケース対応

| ケース    | 現状 | リスク | 推奨対応         |
|-----------|------|--------|------------------|
| 空入力    | 一部未対応 | Medium | keyword, crate名, version null検証追加 |
| 404/ネットワーク | エラー変換済み | 低 | ロギング拡張 |
| HTML構造変更 | Error化 | Medium | selector定数外部設定 |
| 異常データ  | Error化 | Medium | ロギング拡張 |

---

## 5. パフォーマンスとスケーラビリティ

### 性能特性

- **時間計算量**: Item数依存O(n)、tantivyインデックス構築/検索は高速
- **空間計算量**: 一時的にItem全件メモリ＋tempディスク利用（Max 50MB）
- **ボトルネック**: 大量Item時のtempdir消費・index構築時間

### 最適化の機会（優先度順）

1. tantivy index再利用（all_items取得→indexパース分離、高速化）
2. tempdir解放と短期利用制御
3. 検索結果最大件数設定
4. HTMLパース・セレクタキャッシュ
5. 外部APIまとめて一括取得

---

## 6. セキュリティ評価

### セキュリティチェック

| 項目           | 状態 | 深刻度 | 対策                                 |
|----------------|------|--------|--------------------------------------|
| 入力検証       | ⚠️   | Medium | 空値検証、length上限                 |
| 認証・認可     | ❌   | Low    | そもそも公開APIのみ対応、このチャンクには現れない |
| データ保護     | ✅   | Low    | 機密データ扱いなし                   |
| 依存関係       | ✅   | Low    | 標準的クレートのみ、脆弱性未検出     |

---

## 7. テスト戦略

### カバレッジ評価

- **現状**: 10%（fetch_all_items/正常系のみ）
- **目標**: 80%以上
- **不足領域**: エラー系、search_items, 境界値, tempdir, パース不正

### 推奨テスト追加

1. 空/不正なcrate_name, version, path: 失敗確認
2. 検索keywordヒットなし: 空リスト検証
3. 一時ディレクトリ異常: Error型返却
4. HTML構造不正（main-content欠落）: エラー検証
5. 大量Item時の性能・リソースリーク確認

---

## 8. 保守性と可読性

### 改善ポイント

- **命名規則**: 一貫性あり、「main_content」/「all_items」等直感的
- **ドキュメント**: 関数Doc不足（公開APIは追記推奨）
- **構造**: search_itemsの関数分割、共通パース部の外部化

---

## 9. 改善ロードマップ

### 優先度別アクション

| 優先度 | 項目                   | 工数 | 期限目安 |
|--------|------------------------|------|----------|
| 1      | テスト強化              | 2h   | 即座     |
| 2      | tempdirライフサイクル管理| 1h   | 1週間    |
| 3      | search_items関数分割    | 2h   | 1ヶ月    |
| 4      | ロギング拡張            | 1h   | 四半期   |

---

## 10. 良い実践の認識

- **Arc<dyn Trait>によるDI設計**（テスト容易性高）
- **Errorラップ/変換戦略**（エラー型が一貫し統合）
- **async/await＋Send/Sync設計**（スレッド安全性保証）
- **tracingコンテキスト利用**（運用容易性の片鱗）

---

## コンポーネントインベントリー（本チャンク対応分）

| 種別 | 名前                         | 可視性 | 責務                                     | 依存先            | 複雑度 | LOC | 備考                 |
|------|------------------------------|--------|-------------------------------------------|-------------------|--------|-----|----------------------|
| Struct | DocsUseCase                | pub    | ドキュメント取得・検索                   | HttpRepository    | Medium | 13  | DI/Arc               |
| Function | extract_main_content     | pub(super) | main-content抽出(html, selector)        | scraper           | Low    | 15  | HTMLセレクタ汎用     |
| Function | fetch_document_index_page| pub    | Indexページ取得・Markdown変換             | http_repository, html2md | Medium | 22 |                      |
| Function | fetch_document_page      | pub    | 任意ページ取得・Markdown変換              | http_repository, html2md | Medium | 26 |                      |
| Function | parse_all_items          | pub(super) | h3/ul/a抽出→Item構築                    | scraper           | Medium | 41  | 集約パース           |
| Function | fetch_all_items          | pub    | all.html取得・parse_all_items呼び出し     | http_repository   | Medium | 11  |                      |
| Function | search_items             | pub    | 全アイテム全文検索（tantivy一時DB）       | tantivy, tempdir  | High   | 91  | 主要分岐＋DB操作     |
| Struct | Item                       | pub    | ドキュメント内アイテム表現（type, href, path） | -                 | Low    | 不明 | 外部定義。           |

---

## Dependencies & Interactions

### 内部依存

```mermaid
graph TD
  "fetch_document_index_page" --> "extract_main_content"
  "fetch_document_page" --> "extract_main_content"
  "fetch_all_items" --> "parse_all_items"
  "search_items" --> "fetch_all_items"
  "parse_all_items" --> "scraper"
```
（src/use_case/docs.rs:L13-182）

### 外部依存

| ライブラリ名 | バージョン | 役割         | 代替案           | 選定理由             | リスク       |
|--------------|----------|--------------|------------------|----------------------|-------------|
| scraper      | 0.12.x   | HTML解析     | select, kuchiki  | Rust標準/軽量        | 互換性良好   |
| tantivy      | 0.19.x   | 全文検索      | sled, quickwit   | 強力・高速            | tempdir消費  |
| html2md      | 0.1.x    | HTML→Markdown| pulldown_cmark   | シンプル・直感的      | 精度可変     |
| tempfile     | 3.x      | 一時ディレクトリ| std::env::temp  | 簡便・安全           | クリーンアップ要 |
| tracing      | 0.1.x    | ログ出力     | log, slog        | 機能豊富・普及        | ログ濃度     |

---

## データフローと呼び出し関係（主要関数抜粋: search_items）

### 入力→変換→出力フロー

1. fetch_all_itemsで全Item取得
2. tantivy::Schemaインスタンス生成
3. tempfile::tempdir()で一時ディレクトリ生成
4. tantivy::Index::create_in_dirでインデックス作成
5. 全Itemをtantivy::TantivyDocumentで格納
6. IndexWriterでcommit実行
7. reader_builder→QueryParserでkeyword検索
8. TopDocsで上位10件取得
9. TantivyDocument→Item復元→Vec<Item>返却

### I/O境界

- HTTP: http_repository.get（外部docs.rs API）
- FS: tempfile, tantivy（ローカル一時ファイル）
- ログ: tracing::error

---

## Bugs/Security（詳細）

- tempdirの利用時、drop管理が不十分な可能性（src/use_case/docs.rs:L108-151）→再利用、明示削除推奨
- fetch_all_itemsで大量データ時メモリ枯渇の懸念（src/use_case/docs.rs:L91-182）
- Item型フィールドnull/空値時、DB格納orMarkdown変換未検証（src/use_case/docs.rs:L165-182）

---

## Contracts/Edge Cases

| エッジケース           | 入力例             | 期待動作                | 実装 | 状態    |
|-----------------------|--------------------|-------------------------|------|--------|
| 空keyword             | ""                 | 空リスト返却            | ?    | 未検証 |
| HTML main-content空   | main欠如           | Error(HtmlMainContentNotFound)| Yes | OK     |
| 404/Network失敗       | HTTP失敗           | Error型返却             | Yes  | OK     |
| selector不正          | "section@@wrong"   | Error(ScraperSelectorParse)   | Yes | OK     |
| アイテム属性欠落      | href/path:none     | Optionalとして格納       | Yes  | OK     |
| tempdir失敗           | OS I/Oエラー       | Error型返却             | Yes  | OK     |

---

## Tests

- test_fetch_document_page（fetch_all_items, Crate名"serde", version"latest",正規系のみ：src/use_case/docs.rs:L185-195）
- エラー系・大量データ系・検索keyword系は未実装

---

## Performance/Scalability

- Item数が10倍/100倍の場合、一時メモリ/FS消費比例増・tantivyインデックス構築時間増大（src/use_case/docs.rs:L91-182）
- search_itemsで10件限定返却によりレイテンシ抑制
- tantivy→sled/quickwit等本格導入の場合、インデックス永続化・分散ストレージ化でスケール容易

---

## Trade-offs

| 設計案              | 利点                     | 欠点             | リスク              | 移行コスト | 推奨度 |
|---------------------|--------------------------|------------------|---------------------|------------|-------|
| tantivy一時Index    | 高速・依存少             | tempdir消費大    | リーク/スケール限界  | -          | ⭐⭐⭐   |
| Arc<dyn Trait>DI    | テスト容易・責務分離     | vtableコスト     | 小規模では冗長      | -          | ⭐⭐⭐   |
| scraper             | HTML解析容易             | selector依存     | HTML構造変化に脆い  | -          | ⭐⭐⭐   |

---

## Refactoring

- search_items：約91行の分割/責務抽出（tantivy init, search, result mappingで3分割）
- testモジュール強化（エラー系/パラメータ境界値/性能テスト）
- tempdir管理の明示化
- selector/定数の外部化

---

## Observability

- tracing::errorで例外時のみ出力、通常INFO/DEBUGは未出力
- tempdir/FS関連の操作成功/失敗ログ追加推奨
- 構造化ログ（key-value型）導入も選択肢

---

【本チャンク分のレビューは以上です。次チャンク、他ファイルとの依存グラフ・統合は別途。】