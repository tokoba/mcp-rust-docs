# src\repository\http.rs コードレビュー

## Executive Summary

### 対象ファイル

- src\repository\http.rs (28行, 2関数, 2エクスポート)

### 要約

- 本ファイルはHTTP取得処理を抽象化する`HttpRepository`トレイト、およびその実装`HttpRepositoryImpl`を提供し、主に非同期Webリクエストの統一インターフェースを担います。reqwestクライアントは`crate::cache::get_or_init_reqwest_client()`経由で非同期初期化・取得される設計。エラーは統一型でラッピングされtracingで即時ログされます。I/O境界明確化、Send/Sync準拠、Result型採用など、Rustの言語安全性にも配慮されていますが、Input検証・レスポンスボディ制限・タイムアウト・リトライ・Status Code多様性対応の不足など運用上の懸念点も把握できます。Scalabilityはクライアント共有実装次第。テストコード有無はこのチャンクからは不明。タイムアウト/リトライ等運用性改善や、不正引数・異常系テスト拡充などの保守性向上を推奨します。

### 主要公開API

- HttpRepository（トレイト型、getメソッド）
- HttpRepositoryImpl（構造体型）
- HttpRepositoryImpl::get（非同期Web取得/エラー処理/ログ対応）

### 公開クラス・構造体

| 種別     | 名前             | 公開範囲 | 責務      | 複雑度 |
|----------|------------------|----------|-----------|--------|
| Trait    | HttpRepository   | pub      | HTTP取得の抽象 | Low    |
| Struct   | HttpRepositoryImpl | pub   | HTTP取得の具象実装 | Low    |

### 重要指摘事項

| 深刻度  | 件数 | 主な内容                                  | 推定修正工数 |
|---------|------|-------------------------------------------|--------------|
| Critical | 0    | なし                                      | 0h           |
| High    | 1    | 入力値不正、タイムアウト/リトライ未考慮           | 1-2h         |
| Medium  | 2    | HTTP status多様化未対応、例外テスト不足          | 1-2h         |
| Low     | 2    | ログ粒度、レスポンスボディ無制限               | 1h           |

---

## コンポーネントインベントリー

| 種別      | 名前                 | 可視性 | 責務                       | 依存先                           | 複雑度 | LOC | 備考                              |
|-----------|----------------------|--------|----------------------------|-----------------------------------|--------|-----|-----------------------------------|
| Trait     | HttpRepository       | pub    | HTTP取得の抽象             | std::fmt::Debug, Send, Sync      | Low    | 5   | async_trait, error伝播            |
| Struct    | HttpRepositoryImpl   | pub    | HTTP取得の具体実装         | HttpRepository, reqwest, cache   | Low    | 3   | derive(Debug)                     |
| Function  | get                  | pub    | 非同期HTTP GET取得         | reqwest::Client, error, tracing  | Low    | 16  | impl async, 詳細は下記関数表参照   |

---

## 関数詳細テンプレート

### HttpRepository::get

| 項目                   | 内容                                                                 |
|------------------------|----------------------------------------------------------------------|
| **シグネチャ**         | `async fn get(&self, url: &str) -> Result<String, crate::error::Error>` |
| **可視性**             | pub (トレイト経由/継承者に公開)                                      |
| **型パラメータ/制約**   | Self: std::fmt::Debug + Send + Sync                                |
| **ミュータビリティ**   | &self（不変借用、内的状態変更なし）                                  |
| **所有権/参照の変化**   | urlは参照型（&str）で受取、戻り値はResult<String,...>（所有権転送）  |
| **入力引数**           | url: &str（HTTP/HTTPS URL文字列、要正規化/妥当性未検証）             |
| **戻り値**             | Result<String, crate::error::Error>                                 |
| **事前条件**           | urlが正当なURL形式、HTTP/HTTPSへのアクセスが可能                    |
| **事後条件**           | 成功時: html本文取得。失敗時: エラー型返却、tracingでログ出力        |
| **不変条件**           | self, url不変。非同期呼出しにつき外からの影響なし                   |
| **副作用**             | reqwest経由のHTTP通信、tracingでのエラーログ                        |
| **パニック/例外**      | パニックなし（Resultで伝播）。unwrap等の強制終了なし                |
| **エラーモデル**       | ネットワーク/HTTP層のエラー、body取得エラー全てcrate::error::Errorでwrap |
| **計算量**             | 時間: ネットワークI/O依存（O(1)～O(ネットワーク応答時間））、空間: レスポンス本文分 |
| **並行性/スレッド安全性** | クライアント共有が適切なら問題なし                                  |
| **安全でない操作**     | なし                                                                  |
| **呼び出し関係**       | Self::get → cache::get_or_init_reqwest_client → reqwest::Client.get |
| **使用例**             | ```rust<br>let repo = HttpRepositoryImpl{};<br>let html = repo.get(\"https://example.com\").await?;<br>``` |
| **エッジケース**       | 空文字列、http以外、404, 500応答、ボディ巨大、ネットワーク切断        |
| **根拠行番号**         | src\repository\http.rs:L10-26                                         |

#### アルゴリズム・処理フロー

1. 非同期でreqwestクライアントを`cache`から取得（L13）。
2. .get(url)でリクエスト初期化し.send().awaitでHTTPリクエスト実行（L14）。
3. ネットワークエラー時はtracingでログしErrorラッピングして返却（L14-16）。
4. レスポンスStatusが!is_success（例：4xx,5xx）は失敗扱い（L18-22）。
5. エラー時はStatusコードを含めたメッセージでError返却。
6. .text().awaitでHTML本文取得、失敗時は同様にエラーラッピング（L23-25）。
7. Okなら内容文字列を返却。

#### 引数表

| 引数  | 型         | 意味                           | 必須 | 検証           | 備考               |
|-------|------------|--------------------------------|------|----------------|--------------------|
| url   | &str       | 取得先URL（HTTP/HTTPS推奨）     | Yes  | 形式検証無     | 空文字未チェック   |

#### 戻り値表

| 戻り値                       | 意味                         |
|------------------------------|------------------------------|
| Ok(String)                   | レスポンスボディ（HTML等）        |
| Err(crate::error::Error)     | 通信/HTTP/変換/ロジックエラー     |

#### 使用例

```rust
// 正常例
let repo = HttpRepositoryImpl {};
let html = repo.get("https://example.com").await?;  // htmlには応答本文

// エラー例
let err = repo.get("").await.expect_err("Empty URLはエラー");
```

#### エッジケース具体例

| エッジケース         | 入力例    | 期待動作                                 | 実装 | 状態 |
|---------------------|-----------|------------------------------------------|------|------|
| 空文字列            | ""        | Err(HTTP Error)                          | ×    | 場合による |
| 不正プロトコル      | "ftp://..." | Err(HTTP Error)                      | ◯    | 済   |
| 存在しないURL       | "https://none" | Err(HTTP Error)                       | ◯    | 済   |
| 404応答             | "https://.../none" | Err(HTTP Error)                  | ◯    | 済   |
| タイムアウト        | "https://10.0.0.0/timeout" | Err(HTTP Error)             | ◯    | 要強化 |
| 巨大レスポンス      | 大容量URL | OOMリスク（制限なし）                   | ×    | 未実装 |

#### データ契約

- 入力: &str（URL文字列、空・不正値防止が必要）
- 出力: UTF-8文字列想定。バイナリは不適。
- エラー: crate::error::Error型準拠。内包文言は外部ソース由来も含むため処理側でサニタイズ推奨。

#### 事前/事後条件

- 事前: urlが有効なHTTP/HTTPS endpointであること
- 事後: エラー時は原因保全、htmlは完全テキストで返却

#### 不変条件

- セッション/クッキー等は考慮なし、都度クライアント初期化依存。

#### 副作用・パニック可能性

- tracing::errorでロギング（L15, L24）（INFO/WARN粒度差異なし）。
- パニック不発生（すべてResultで拾う）。

#### 所有権/借用/ミュータビリティ

- urlは不変借用。レスポンスボディは所有権取得。
- Selfは&self:状態変化なし（stateless）。

#### 代替案とトレードオフ

| 設計案            | 利点                      | 欠点         | リスク         | 移行コスト | 推奨度 |
|-------------------|---------------------------|--------------|---------------|------------|--------|
| 現行実装          | 単純で汎用的              | 入力検証/タイムアウト不足 | OOM/DoS      | -          | ⭐⭐⭐  |
| 入力値検証追加    | 無効URL早期除外           | 複雑性増      | 誤判定増加     | 0.5h       | ⭐⭐⭐  |
| タイムアウト追加  | レスポンス長大/DoS防御    | 設計追加      | 既存コード非互換 | 1h         | ⭐⭐   |

---

## データフロー・呼び出し関係

### メインデータフロー

```mermaid
flowchart TD
    "request: get(&self, url)" -->|"クライアント取得"| "get_or_init_reqwest_client"
    "get_or_init_reqwest_client" -->|"Client返却/Err"| "client.get(url).send()"
    "client.get(url).send()" -->|"Result"| "response"
    "response" -->|"!is_success"| "エラー返却"
    "response" -->|"is_success"| "response.text().await"
    "response.text().await" -->|"Result"| "Ok(html) or エラー返却"
```
上記の図は`HttpRepositoryImpl::get` (src\repository\http.rs:L13-26)の主要データフローを示す。

---

## Bugs/Security/Edge Cases

- **入力検証なし**: 空文字列/異常パス指定でpanicはないが、下層依存先挙動不定。要明示チェック（L13）。
- **ボディ制約なし**: 巨大レスポンスでOOMの懸念。Streamによる読み出し/制限必須（未実装）。
- **タイムアウト/リトライ**: デフォルト未指定。外部依存先設定に依拠。意図的なタイムアウトも未考慮。
- **ログ粒度**: tracing::errorのみ、情報として適切だがINFO/WARN不使用。

| 項目           | 状態 | 深刻度 | 対策                     |
|----------------|------|--------|--------------------------|
| 入力検証       | ❌   | High   | &str空・不正検証追加      |
| 認証・認可     | 不明 | -      | -                        |
| データ保護     | N/A  | -      | -                        |
| 依存関係       | reqwest,tracing等 | - | Cargo audit推奨    |

---

## Contracts/Edge Cases

- 引数: url（&str)は空・不正時エラーを明示化推奨
- 戻値:ネットワーク異常・タイムアウト時も必ずError型でラップ
- 巨大・バイナリ応答は明示的制限・バリデーションが必要

---

## テスト（このチャンクでは不明）

- ユニット/統合テスト有無は本チャンクからは不明
- 必要なテストケース
  - 正常: HTTP 200応答・html返却
  - 異常(URL不正/400/404/500/Timeout等): 各ケースで正しくError返却
  - 巨大レスポンス/OOM耐性テスト(必要)

---

## パフォーマンス・スケーラビリティ

- 性能はネットワークI/O依存（O(n)は応答サイズ依存, メモリ=全データ読込量）
- クライアント共有設計（cache利用）でリソース消費を最小化
- 10倍,100倍スケールでは巨大レスポンス制御・多並列アクセス時のMutex競合等に注意

---

## Dependencies & Interactions

### 内部依存

- cache::get_or_init_reqwest_client (L13): 非同期クライアント管理
- crate::error::Error: 統一エラー管理
- tracing: ログ出力(L15, L24)

### 外部依存

| ライブラリ    | バージョン  | 役割                | 代替案   | 選定理由     | リスク         |
|---------------|------------|---------------------|----------|--------------|----------------|
| reqwest       | 不明       | 非同期HTTPクライアント | hyper    | 普及度/async | 小(破壊的変更) |
| tracing       | 不明       | ログ記録            | log, slog| 先進log/tracing| 低             |
| async-trait   | 不明       | トレイトのasync化   | N/A      | マクロ普及    | 低             |

### 被依存推定

- repositoryパターン利用層（ユースケース層、Webハンドラ等）
- 単体ではstateless,テスト容易性高

---

## 観測性・運用性

- ログ: tracing::errorのみ、INFO/WARN/DEBUG未活用
- メトリクス: 無し。reqwest/tokio依存ならメトリクスフック化可能
- 設定: タイムアウト/ボディサイズ制限は未対応
- 障害対応: エラー時ロギングのみ、リトライ等は手動
- SLA/SLO: レイテンシ/可用性は別層で定義が望ましい

---

## 良い実践

- トレイトによる抽象化でテスト容易
- 統一エラー型で一元管理・ロギング即時化
- セッション管理依存（cache）の分離による疎結合
- async/await & 非同期設計準拠
- パニック未発生保証、Result型安全志向

---

【以上：src\repository\http.rs (L1-28)根拠】