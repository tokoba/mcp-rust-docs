# src/resource.rs コードレビュー

## コンポーネントインベントリー

| 種別      | 名前               | 可視性     | 責務                                             | 依存先                                | 複雑度 | LOC | 根拠行番号                | 備考                     |
|-----------|--------------------|------------|--------------------------------------------------|---------------------------------------|--------|-----|--------------------------|--------------------------|
| Struct    | `Resource`         | pub        | リソース情報（メタデータ＋内容）の表現            | rmcp::model::ResourceContents         | Low    | 7   | src/resource.rs:L1-7     | name/description等を保持 |
| Struct    | `ResourceMap`      | pub        | リソース管理（in-memory, 共有Mapでwrap）         | std::collections::HashMap, Arc        | Low    | 6   | src/resource.rs:L9-14    | 内部状態はArc<HashMap>   |
| Function  | `ResourceMap::new` | public     | リソースセット初期化、デフォルトリソース投入      | Resource, HashMap, Arc                | Low    | 22  | src/resource.rs:L16-39   | include_strで外部md読込  |
| Function  | `list_resources`   | public     | 全リソース一覧返却                               | RawResource, Annotated, Future        | Med    | 23  | src/resource.rs:L41-63   | 非同期                    |
| Function  | `read_resource`    | public     | 指定リソースの内容返却                           | ReadResourceRequestParam, ErrorData    | Med    | 22  | src/resource.rs:L65-86   | 非同期・見つからなければErr |
| Function  | `list_resource_templates` | public | テンプレート一覧返却（現状空のデフォルト）        | ListResourceTemplatesResult, Future    | Low    | 6   | src/resource.rs:L88-93   | 非同期fixed値             |

**合計関数数: 4（全て public）**  
**合計Struct数: 2（全て public）**

---

## 関数詳細テンプレート適用

### ResourceMap::new

| 項目                | 内容                                                                                                  |
|---------------------|-----------------------------------------------------------------------------------------------------|
| シグネチャ          | `pub fn new() -> Self`                                                                              |
| 可視性              | public                                                                                              |
| 型パラメータ/制約   | なし                                                                                                |
| ミュータビリティ    | マップ生成時のみmutable。返却値はArcで不変本体                                                       |
| 所有権              | map（HashMap）がArcに格納され、ResourceはHashMapで所有される                                         |
| 入力引数            | なし                                                                                                |
| 戻り値              | `Self` (`ResourceMap`; 内部はArc<HashMap<String, Resource>>）                                         |
| 事前条件            | なし                                                                                                |
| 事後条件            | Instructionリソースが1件格納されたResourceMapが生成される                                            |
| 不変条件            | None（ResourceMapは外から内部を直接変えられない）                                                    |
| 副作用              | include_str!によるファイル読込のみ                                                                 |
| パニック/例外       | include_str!ファイルがない場合ビルドエラー/パニック                                                  |
| エラーモデル        | なし                                                                                                |
| 計算量              | O(1)                                                                                                |
| 並行性/スレッド安全 | Arcによる参照カウントでReadOnly。MutではないがCloneで共有可能                                       |
| 安全でない操作      | なし                                                                                                |
| 呼び出し関係        | なし                                                                                                |
| 使用例              | `let res_map = ResourceMap::new();`（Instructionが1件格納済）                                       |
| エッジケース        | - include_str!の対象ファイルが存在しない場合                                              |
| 根拠行番号          | src/resource.rs:L16-39                                                                              |

---

### ResourceMap::list_resources

| 項目                | 内容                                                                                                                 |
|---------------------|----------------------------------------------------------------------------------------------------------------------|
| シグネチャ          | `pub fn list_resources(&self, _request: Option<PaginatedRequestParam>, _context: RequestContext<RoleServer>) -> impl Future<Output=Result<ListResourcesResult, ErrorData>> + Send + '_` |
| 可視性              | public                                                                                                               |
| 型パラメータ/制約   | ライフタイム`'_`、FutureはSend                                                                                          |
| ミュータビリティ    | 不変`&self`（Arc<HashMap>の参照）                                                                                      |
| 所有権              | Resourceインスタンスの値はCloneされて返される                                                                        |
| 入力引数            | PaginatedRequestParam（現状未使用）, RequestContext<RoleServer>（未使用）                                            |
| 戻り値              | Future（成功: ListResourcesResult{ resources: Vec<Annotated<RawResource>> }, 失敗: ErrorData）                        |
| 事前条件            | None                                                                                                                  |
| 事後条件            | 存在する全リソースをRawResource形式で返却                                                                            |
| 不変条件            | ResourceMapの内部状態は変化しない                                                                                     |
| 副作用              | なし                                                                                                                  |
| パニック/例外       | なし                                                                                                                  |
| エラーモデル        | 失敗しない（現状では必ずOk返却）                                                                                      |
| 計算量              | O(n)（n=リソース件数; mapの全要素走査）                                                                               |
| 並行性/スレッド安全 | Arc持ち回しによるRead Only; Lock不要                                                                                   |
| 安全でない操作      | なし                                                                                                                  |
| 呼び出し関係        | rmcp::model::Resource::new, Annotated, HashMap::iter                                                                  |
| 使用例              | `let fut = resource_map.list_resources(None, ctx);`                                                                   |
| エッジケース        | - リソースが空の場合（空Vec返却）                                                                                     |
| 根拠行番号          | src/resource.rs:L41-63                                                                                                |

---

### ResourceMap::read_resource

| 項目                | 内容                                                                                                                        |
|---------------------|-----------------------------------------------------------------------------------------------------------------------------|
| シグネチャ          | `pub fn read_resource(&self, request: ReadResourceRequestParam, _context: RequestContext<RoleServer>) -> impl Future<Output=Result<ReadResourceResult, ErrorData>> + Send + '_` |
| 可視性              | public                                                                                                                      |
| 型パラメータ/制約   | ライフタイム`'_` FutureはSend                                                                                                |
| ミュータビリティ    | 不変`&self`                                                                                                                 |
| 所有権              | リクエストのuri文字列はmove、ResourceはCloneして返す                                                                        |
| 入力引数            | ReadResourceRequestParam（必須）, RequestContext<RoleServer>（未使用）                                                      |
| 戻り値              | Future（成功: ReadResourceResult{contents: Vec<ResourceContents>}、失敗: ErrorData）                                        |
| 事前条件            | uriが有効な登録済リソースのuriである                                                                                        |
| 事後条件            | 該当リソースがあればcontents返却、なければリソース未発見エラー返却                                                          |
| 不変条件            | ResourceMapの内部状態は変化しない                                                                                           |
| 副作用              | なし                                                                                                                        |
| パニック/例外       | なし                                                                                                                        |
| エラーモデル        | Resource not found: uriが一致しなければErrorData::resource_not_found                                                        |
| 計算量              | O(1)（HashMap getのみ）                                                                                                     |
| 並行性/スレッド安全 | Arc内HashMapでRead Only; スレッド安全                                                                                        |
| 安全でない操作      | なし                                                                                                                        |
| 呼び出し関係        | HashMap::get, ErrorData::resource_not_found                                                                                 |
| 使用例              | `let fut = resource_map.read_resource(ReadResourceRequestParam{uri: "..."} , ctx);`                                         |
| エッジケース        | - uriが未格納の場合（not foundエラー）                                                                                      |
| 根拠行番号          | src/resource.rs:L65-86                                                                                                      |

---

### ResourceMap::list_resource_templates

| 項目                | 内容                                                                                                  |
|---------------------|-----------------------------------------------------------------------------------------------------|
| シグネチャ          | `pub fn list_resource_templates(&self, _request: Option<PaginatedRequestParam>, _context: RequestContext<RoleServer>) -> impl Future<Output=Result<ListResourceTemplatesResult, ErrorData>> + Send + '_` |
| 可視性              | public                                                                                              |
| 型パラメータ/制約   | ライフタイム`'_` FutureはSend                                                                       |
| ミュータビリティ    | 不変`&self`                                                                                         |
| 所有権              | なし                                                                                                |
| 入力引数            | PaginatedRequestParam（未使用）, RequestContext<RoleServer>（未使用）                                |
| 戻り値              | Future（成功: 空のListResourceTemplatesResult, 失敗:なし）                                          |
| 事前条件            | なし                                                                                                |
| 事後条件            | 常に空テンプレート一覧返却                                                                          |
| 不変条件            | なし                                                                                                |
| 副作用              | なし                                                                                                |
| パニック/例外       | なし                                                                                                |
| エラーモデル        | 失敗しない                                                                                          |
| 計算量              | O(1)                                                                                                |
| 並行性/スレッド安全 | スレッド安全                                                                                        |
| 安全でない操作      | なし                                                                                                |
| 呼び出し関係        | ListResourceTemplatesResult::default                                                                |
| 使用例              | `let fut = resource_map.list_resource_templates(None, ctx);`                                        |
| エッジケース        | - 常に空返却のみ                                                                                    |
| 根拠行番号          | src/resource.rs:L88-93                                                                              |


---

## データフロー・呼び出し関係

```mermaid
flowchart TD
  "ResourceMap::new"["ResourceMap::new (L16-39)"] --> "ResourceMap"
  "ResourceMap::list_resources"["list_resources (L41-63)"] --> "HashMap::iter"
  "ResourceMap::read_resource"["read_resource (L65-86)"] --> "HashMap::get"
  "list_resources" --> "Resource::new"
  "read_resource" --> "ErrorData::resource_not_found"
```
> 上記の図は src/resource.rs の関数ごとの主要メソッド呼び出しを示す。このファイルのみの範囲で記述。

### データフロー要約
- **ResourceMap::new** … 初期化時にInstructionリソースのみ挿入。  
- **list_resources** … HashMap全件iter→Resourceの情報をRawResourceに変換してVecで返す。  
- **read_resource** … リクエストuriでHashMap→該当Resource.contentsを返す。なければErrorData返す。
- **list_resource_templates** … 固定値（空）をFutureで返す。

---

## Edge Cases, Bugs, and Security

### 主要エッジケース

| エッジケース    | 入力例                | 期待動作                | 実装                      | 状態           |
|----------------|----------------------|-------------------------|--------------------------|----------------|
| 不明なuri      | "str://unknown"      | resource_not_found返却  | `read_resource`         | 実装済         |
| リソース0件    | mapが空              | 空リスト返却            | list_resources           | 構造上不可逆    |
| mdファイル無し | instruction.md無し   | ビルドエラーorパニック  | new/含まないとbuild失敗   | Rustビルドで検出|
| 大量リソース   | 10000件              | O(n)で全件返却          | list_resources           | スケール線形    |
| コンテンツ拡張 | 非TextResource追加   | contentsにそのまま格納  | 拡張は不可(現状1種のみ)   | 拡張性低        |

### セキュリティ/メモリ安全性

- **メモリ安全**: 全て所有権/借用/Arcで安全設計（unsafeブロック無し、直接パニックもなし; src/resource.rs:L1-98）
- **外部入力検証**: uriのみHashMap key検索、危険なI/O一切なし
- **共有状態**: Arc<HashMap>不変運用でスレッドセーフ（競合性・デッドロックなし）

---

## Contracts/Edge Cases

### データ契約（型・値）

- `Resource`の各フィールドはCloneされて返却、外部操作不可
- HashMapのkeyはuri(String)で一意、コリジョン時は上書き
- ResourceContentsの型が変化する場合、呼び出し側で変換（本ファイル内はText固定）

### 事前/事後条件、不変条件

- **list_resources/read_resource**: ResourceMapの変更不可、不変
- **read_resource**: 該当なしでNotFoundエラー
- **全メソッド**: 外部mutable操作不可設計、Arcによるイミュータブル共有を徹底

---

## 検出されたBugs/Security Risks

- **重大なリスクなし**（ファイルI/Oや外部入力依存が極めて限定的; 設計的安全）

---

## テスト/カバレッジ

- **テストコード不明**: このチャンクには現れない
- 分岐点（NotFound/通常/空リソース）が検証対象
- データ拡張時のregression test追加推奨

---

## Performance/Scalability

- **list_resources**: O(n)の線形スキャン。リソース数の増加に従い応答時間増加（現状は問題なし; n≒1）
- **read_resource**: O(1)の高速key検索(HashMap)
- **初期化時**: ファイル取り込み、リソース増でメモリ使用増大。ただし実装用途から十分軽量

### スケール観点
- n=10,000超でlist_resourcesはボトルネック化。Shard/DB導入時は別設計推奨

---

## Tradeoffs

| 判断事項                  | 採用理由（推定）           | 影響範囲         | リスク                  |
|--------------------------|----------------------------|------------------|-------------------------|
| Arc<HashMap> Immutable   | スレッドセーフ、超効率     | 全API            | 変更不能で拡張性制限    |
| include_str!利用         | デプロイ簡素化、即時読込   | 初期データ       | バイナリ肥大・柔軟性低  |

---

## 改善点/Refactoring

- リソースが動的追加・削除される運用にする場合はRwLock+mutable構造が必須
- list_resourcesはページネーション拡張引数未対応 → 分割実装推奨

---

## 可観測性・運用性

- ログ/メトリクス/エラートレーシング未実装: 本用途では不要だが拡張設計時には必須
- 設定値/Feature flag等は現状無

---

## 良い実践・優れた設計

- **Arc<HashMap>によるイミュータブル共有**: 多スレッド下で競合・ロック不要に設計、保守容易（src/resource.rs:L9-14, L16-39）
- **型安全/所有権遵守**: 全メソッドでRustの所有権/可変性の設計原則を厳密運用
- **Future/非同期対応API一式**：今後のI/O非同期化、実API統合への備えとして先進（src/resource.rs:L41-93）

---