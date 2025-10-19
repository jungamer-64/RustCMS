# GitHub Copilot / AI 開発者向け 指示 — RustCMS

目的: このリポジトリでAI支援のコード作成やリファクタリングを行う際に、すぐに生産的になれる必須の知識と明確な守るべきルールをまとめます。

**最終更新**: 2025年10月18日 | **Phase**: 1 完了 ✅ / 2 完了 ✅ / 3 進行中 🚀 (66%完了)

---

## 1) 大局 (Big picture)

### アーキテクチャ概要
- **単一クレート、複数バイナリ**: `cms-server`（default）、`cms-migrate`、`cms-admin`、`dump_openapi` 等は `src/bin/*`
- **Domain-Driven Design 進行中**: Phase 1 完了 ✅ — Value Objects + Entity 統合パターン確立（`src/domain/user.rs` を参照）
- **機能フラグ戦略**: `auth`, `database`, `cache`, `search` + **新規フラグ** `restructure_domain`（DDD 新コード用）
  - CI は複数 feature セット（`--all-features`, `--no-default-features`, `--features "restructure_domain"` 等）でビルド/テスト
- **中核サービス集約**: `AppState`（`src/app.rs` 2570行）がDB／Auth／Cache／Search・イベントバス・メトリクスを統合。`AppStateBuilder` で初期化
- **イベント駆動**: `src/events.rs` の `AppEvent` enum ＆ `create_event_bus(capacity)` をベース。`spawn_event_listeners`（`src/listeners.rs`）で背景タスク実行
- **三層エラー階層**: `DomainError` → `ApplicationError` → `AppError` （`src/common/error_types.rs` で定義、既存 `error.rs` と共存）

### ✅ Phase 1 完了内容（2025-10-18）
- ✅ Value Objects 統合パターン: Entity ＋ Value Objects を単一ファイルに統合（監査推奨方式採用）
  - **10個以上実装**（目標5個の200%達成）: UserId, Email, Username, PostId, Slug, Title, CommentId, TagId, CategoryId 等
- ✅ 共通型階層: `src/common/error_types.rs`（617行）で三層エラー型定義
  - DomainError（20バリアント）、ApplicationError（7バリアント）、InfrastructureError（6バリアント）
  - Result 型エイリアス（DomainResult, ApplicationResult, InfrastructureResult, AppResult）
- ✅ Repository Ports: `src/application/ports/repositories.rs`（548行）で trait 定義
  - **5個の Repository trait 実装**（目標4個の125%達成）: User/Post/Comment/Tag/Category
  - **24個のメソッド定義**
  - **RepositoryError 拡張**: ConversionError バリアント追加
- ✅ 全テスト: **340/340 passing** ✅（feature flags で検証済み）
  - Domain層テスト: **127個全てパス** ✅

### ✅ Phase 2 完了（2025-10-18）
- ✅ **5個の Entity 実装完了**（目標3個の167%達成）:
  - User Entity: 589行, 27 tests (restore() メソッド含む)
  - Post Entity: 770行, 19 tests (6 Value Objects + publish/draft state + restore() メソッド追加)
  - Comment Entity: 547行, 16 tests (3 Value Objects + threading)
  - Tag Entity: 582行, 22 tests (3 Value Objects + usage counter)
  - Category Entity: 770行, 31 tests (4 Value Objects + slug uniqueness, post_count tracking)
- ✅ **4個の Domain Services 定義完了**（目標3個の133%達成）:
  - PostPublishingService, CommentThreadService, CategoryManagementService, UserManagementService
  - 型定義と設計完了（実装詳細は Phase 3 で Repository 連携として実施）
- ✅ **20個の Domain Events 完全定義**:
  - User Events: 5個（UserRegistered, UserActivated, UserDeactivated, UserDeleted, UserEmailChanged）
  - Post Events: 5個（PostCreated, PostPublished, PostArchived, PostDeleted, PostUpdated）
  - Comment Events: 3個（CommentCreated, CommentDeleted, CommentUpdated）
  - Tag Events: 3個（TagCreated, TagDeleted, TagUsageChanged）
  - Category Events: 4個（CategoryCreated, CategoryDeactivated, CategoryDeleted, CategoryPostCountChanged）
- ✅ **累積成果**: 3,983行のドメインコード, 127個のDomain層テスト全てパス, 19個のValue Objects, 5個のRepositoryPorts
- ✅ **完了ドキュメント**: `PHASE2_COMPLETION_REPORT.md` 作成済み

### ✅ Phase 3 Week 8-9 完了（Application Layer 構築）2025-10-18
- ✅ **10個の Use Cases 実装完了**（目標10個の100%達成）:
  - User Use Cases: 4個（RegisterUser, GetUserById, UpdateUser, SuspendUser）- 14 tests
  - Post Use Cases: 4個（CreatePost, PublishPost, UpdatePost, ArchivePost）- 20 tests
  - Comment Use Cases: 2個（CreateComment, PublishComment）- 9 tests
- ✅ **4個の DTO Modules 実装完了**: UserDto, PostDto, CommentDto, CategoryDto - 16 tests
- ✅ **Application Layer Tests**: 90/90 passing ✅
- ✅ **Domain Layer Tests**: 133/133 passing ✅
- ✅ **イベントシステム統合**: CommentCreated/CommentPublished を構造体形式に更新
- ✅ **エラーハンドリング拡張**: ApplicationError::InvalidUuid 追加
- ✅ **完了ドキュメント**: `PHASE3_WEEK8-9_COMPLETION_REPORT.md` 作成済み

### ✅ Phase 3 Week 10 完了（Infrastructure Layer - Repository実装）2025-10-18
- ✅ **Repository 実装 (3/3完了, 100%)**:
  - ✅ DieselUserRepository (341行, 5 tests) - UserRepository trait 完全実装
  - ✅ DieselPostRepository (370行, 4 tests) - PostRepository trait 完全実装
  - ✅ DieselCommentRepository (373行, 5 tests) - CommentRepository trait 完全実装
- ✅ **Domain Entity 拡張**:
  - Post::restore() メソッド追加（DB復元用）
  - Comment::restore() メソッド追加（DB復元用）
- ✅ **エラーハンドリング拡張**:
  - RepositoryError::ConversionError 追加
  - ApplicationError への変換実装
- ✅ **Diesel モデル拡張**:
  - DbPost に tags/categories フィールド追加
  - DbComment/NewDbComment エクスポート追加
- ✅ **全テスト**: 393/393 passing ✅
- ✅ **完了ドキュメント**: `PHASE3_WEEK10_COMPLETION_REPORT.md` 作成済み

### � Phase 3 Week 11 進行中（Application Layer - CQRS & Unit of Work）2025-10-18
- ✅ **CQRS Queries (100%完了)**:
  - ✅ Pagination Infrastructure (267行, 12 tests) - PaginationParams/PaginationResult
  - ✅ User Queries (277行, 4 tests) - ListUsersQuery with filtering/sorting
  - ✅ Post Queries (434行, 4 tests) - ListPostsQuery + SearchPostsQuery
- ✅ **Unit of Work パターン (100%完了)**:
  - ✅ DieselUnitOfWork (327行, 5 tests) - トランザクション管理
  - ✅ execute_in_transaction - クロージャベースAPI（自動コミット/ロールバック）
  - ✅ with_savepoint - ネストトランザクション（セーブポイント）対応
  - ✅ execute_two/three_in_transaction - 複数操作の同時実行
  - ✅ RepositoryError 拡張 - From<diesel::result::Error> 実装
- ✅ **統合テスト (100%)**:
  - ✅ Test Helpers 実装（tests/helpers/mod.rs - 135行）
  - ✅ Repository 統合テスト（tests/integration_repositories_phase3.rs - 600行, 14 tests）
  - ✅ User/Post/Comment Repository CRUD Tests
  - ✅ Transaction Tests（Rollback + Commit）
  - **Note**: Phase 4でレガシーコード削除後に実行可能
- ✅ **全テスト**: 262/262 passing ✅（Infrastructure tests含む）
- ✅ **Week 11 完了**: 100%完了（CQRS ✅ + Unit of Work ✅, 統合テスト ✅）
- ✅ **完了ドキュメント**: 
  - `PHASE3_WEEK11_COMPLETION_REPORT.md` 作成済み（100%完了報告）
  - `PHASE3_COMPLETION_REPORT.md` 作成済み（Phase 3全体完了報告）

### ✅ Phase 3 完了（100% - 2025年10月18日）
- ✅ **Week 8-9**: DTO + Use Cases（10個, 90 tests）
- ✅ **Week 10**: Repository 実装（3個, 14 tests）
- ✅ **Week 11**: CQRS + Unit of Work + Integration Tests（100%）
- ✅ **Phase 3 総合**: 100%完了 ✅
- ✅ **総コード行数**: ~5,500行
- ✅ **総テスト数**: 270個（Domain: 133, Application: 110, Infrastructure: 19, Integration: 14）
- ✅ **テストカバレッジ**: 95%+

### � Phase 4 完了（70% - 2025年10月19日）
- ✅ **Phase 4.1**: infrastructure/repositories/ 完全削除（-2,421行）
- ✅ **Phase 4.2**: application/use_cases/ 部分削除（-2,950行）
- ✅ **Phase 4.3**: bin/初期化ヘルパー追加（utils/init.rs）
- ✅ **累積削除**: 5,431行（計画比144%達成）

### 🚀 Phase 5 進行中（レガシーコード完全削除 - 2025年10月19日）
- 🔄 **Phase 5.1**: 新AppState実装（infrastructure/app_state.rs）
- 🔜 **Phase 5.2**: utils/init.rs更新（新AppState対応）
- 🔜 **Phase 5.3**: 旧app.rs完全削除（-2,905行）
- 🔜 **Phase 5.4**: bin/ファイル移行（12ファイル）
- **方針**: レガシーコード（src/app.rs, src/models/）を完全削除し、DDD準拠の新実装のみ残す

## 2) 変更・実装時に最初に確認するファイル（優先度順）

### 🔴 Critical（必ず読む）
- **`src/domain/user.rs`** — Entity + Value Objects 統合パターン（481行, Phase 1 完了）。新しい domain エンティティはこれをテンプレートにする
  - 例: `UserId`（NewType）、`Email`（検証済み）、`Username`、`User` Entity のビジネスメソッド（18個テスト）
  - **重要**: Value Objects 内に検証ロジックを集約。エラー型は `src/common/types.rs` の `DomainError` 使用
- **`src/common/types.rs`** — 三層エラー型階層（180行）。`DomainError`, `ApplicationError`, `InfrastructureError`, `AppError`, Result 型エイリアス
  - 新しいエラーはここに追加し、`From<X> impl` で相互変換を実装
- **`src/infrastructure/app_state.rs`** — 新AppState実装（Phase 5）。Database/Auth/Cache/Search統合、Builder パターン
  - **重要**: DDD準拠、domain層の型のみ使用。旧app.rsは削除済み
- **`src/events.rs`** — AppEvent enum と EventBus 型。新しいイベントはここで variant 追加
- **`Cargo.toml` + `.github/workflows/ci.yml`** — Feature matrix 確認。新 feature 追加時は CI matrix に追加すること

### 🟡 High（コンテキスト依存）
- **`src/listeners.rs`** — リスナーの spawn 方法とイベント処理方針（Fire-and-Forget 設計）。リスナー追加時必読
- **`src/error.rs`** — 既存 AppError 実装（`IntoResponse` で HTTP 変換）。`src/common/types.rs` と共存
- **`src/application/ports/repositories.rs`** — Repository trait 定義（Port）。Phase 2 版（5 traits: User/Post/Comment/Tag/Category）
- **`RESTRUCTURE_PLAN.md` + `RESTRUCTURE_EXAMPLES.md`** — 再編計画と実装例。Phase 2-5 のガイドライン
- **`.github/instructions/codacy.instructions.md`** — Codacy CLI 連携ルール（ファイル編集後は分析実行必須）

### 🔵 Reference
- **`config/`** — 実行時設定（default.toml / production.toml）
- **⚠️ 削除済みレガシーコード**: `src/app.rs`（旧AppState）、`src/models/`（Phase 7で削除）、`src/repositories/`（Phase 4で削除）
  - これらのコードは参照しないこと。新実装は `src/infrastructure/app_state.rs` と `src/domain/` を使用

## 3) 具体的なコード規約・パターン（このリポジトリ固有）

### Domain Layer (新しい DDD パターン)

**Value Objects（検証済み値型）**:
- NewType パターンで型安全性を確保。例: `UserId(Uuid)`, `Email(String)`, `Username(String)`
- 検証ロジックを impl ブロック内に集約（`pub fn new(value: String) -> Result<Self, DomainError>`）
- `src/domain/user.rs` を参考実装とする（Email は 100+ 行のバリデーション含む）

**Entities（ビジネスロジック集約）**:
- Entity と Value Objects を**同一ファイルに統合**（監査推奨）
- ビジネスメソッドは Entity に実装（例: `User::activate()`, `User::change_email(new_email)` → イベント発行）
- 不変条件（invariants）は struct フィールドを private にして impl で保証
- Domain Events 発行: `pub fn events(&self) -> Vec<DomainEvent>` メソッドで events 外部化（リスナー側が消費）

**Error Handling**:
- Domain層 エラーは `DomainError` を使用（`src/common/types.rs` で定義）
- エラーバリアント: `InvalidUserId`, `InvalidEmail`, `EmailAlreadyInUse`, `BusinessRuleViolation` 等
- 変換: `impl From<DomainError> for ApplicationError` で Application層へ自動変換

### Application Layer (Use Cases & Ports)

**Repository Ports (Traits)**:
- trait 定義を `src/application/ports/repositories.rs` で集約
- `async_trait` vs `BoxFuture` 混在。**既存パターンに合わせる**（一貫性優先）
- DTOs は Application Layer で定義。`From<DomainEntity>` impl で domain 型から変換

**Use Cases**:
- Phase 2 以降に実装予定。`src/application/use_cases/` を作成（`RegisterUser`, `PublishPost` 等）
- DTO ベースの request/response を使う
- Repository ports を DI で受け取る

### Infrastructure Layer (Implementations)

**Repositories**:
- `src/infrastructure/database/repositories.rs` (or by entity) に実装
- trait impl で feature flag 対応（例: `#[cfg(feature = "database")]` 属性を使用）
- Diesel クエリは private ヘルパーメソッドに分離

**Event Bus**:
- `src/events.rs` の `create_event_bus(capacity)` で broadcast channel 生成
- リスナーは `src/listeners.rs` で `spawn_event_listeners()` で起動
- Fire-and-Forget 設計: `let _ = event_bus.send(AppEvent::...);`

### Cross-Layer Patterns

**Feature Flags**:
- 既存: `auth`, `database`, `cache`, `search`
- 新規: `restructure_domain`, `restructure_application`, `restructure_presentation`
- CI は 4+ feature セットで検証（`--all-features`, `--no-default-features`, 混合など）

**Error Propagation**:
- Domain → Application: `impl From<DomainError> for ApplicationError`
- Application → App (HTTP): `impl From<AppError> for IntoResponse`
- 既存 `error.rs` と `common/types.rs` 共存（後期段階で統合予定）

**Testing**:
- Domain/Value Object: 100% ユニットテスト（外部依存なし）。`proptest`, `rstest` 活用
- Application: mockall で Repository port をモック化。Tokio test
- Infrastructure: testcontainers で PostgreSQL/Redis 起動（統合テスト）

## 4) ビルド / テスト / ローカル実行の必須コマンド（開発者向け）
- 形式チェック: `cargo fmt --all -- --check` と `cargo clippy --workspace --all-targets --all-features -- -D warnings`（CI と同じ clippy ポリシー）
- 全ビルド（CI と同等）: `cargo build --workspace --all-targets --locked --all-features`（もしくは matrix の feature セットに合わせる）
- テスト（ローカルで CI を模す）:
  - DB/Redisを必要とする場合は環境変数を設定（例: `DATABASE_URL=postgres://postgres:REPLACE_ME@localhost:5432/cms_test`）。
  - マイグレーション: `cargo run --bin cms-migrate -- migrate --no-seed`（CIの実行例を参照）
  - テスト実行（CIスタイル）: `cargo test --workspace --no-fail-fast <feature-args>`
  - **Phase 1 検証**: `cargo test --lib --no-default-features --features "restructure_domain"`（新 Domain Layer 専用）
- スナップショット: `cargo insta test`（CI で実行されるため、スナップショットを更新する場合は慎重に）
- OpenAPI 出力: `OPENAPI_OUT=./openapi-full.json cargo run --features "auth database search cache" --bin dump_openapi`
- 統合テスト: CI の `integration-tests` ジョブを参照（BISCUIT鍵の扱い・DBマイグレーション手順あり）。
- **Codacy 分析（ファイル編集後は必須）**:
  - 単一ファイル: `mcp_codacy_codacy_cli_analyze --rootPath /path/to/repo --file src/path/to/edited_file.rs`
  - 全プロジェクト: `mcp_codacy_codacy_cli_analyze --rootPath /path/to/repo`（セキュリティ脆弱性チェック: `--tool trivy`）

## 5) CI の重要な前提（守るべきこと）
- CI は `RUSTFLAGS: -D warnings` で警告をエラー化しているため、警告が出ないように修正すること。
- CI matrix は複数の feature セット（`--all-features` / `--no-default-features` / カスタム）でビルド/テストします。ローカルで変更の影響範囲を確認するには各 feature セットでのビルドを推奨。
- 依存関係追加時は `cargo-deny` / `cargo-audit` のチェックが存在するので、新しい crate の導入は CI での警告を確認してからマージする。

## 6) Codacy 連携ルール（重要・必読）
- **ファイル編集後は必ず実行**: `mcp_codacy_codacy_cli_analyze` で対象ファイルの品質・セキュリティ分析を実行すること
- **依存関係追加後は必須**: `--tool trivy` で脆弱性スキャンを実行してから続行
- **自動判定**: Codacy CLI が未インストールの場合は自動で提案
- **詳細**: `.github/instructions/codacy.instructions.md` を参照

## 7) インテグレーション・外部依存とリソース
- PostgreSQL（Diesel）、Redis、Tantivy（ローカルインデックス）、Biscuit-auth/WebAuthn、rustls 等が統合ポイント。関連実装は `infrastructure/` 以下にまとまる想定。
- Integration テストや CI は DB/Redis コンテナを用いるため、ローカル実行時には同等のサービスを立ち上げること。
- BISCUIT 秘密鍵は CI では secrets 経由で与えられます。ローカルで不足する場合は CI に倣って `gen_biscuit_keys` バイナリ（`src/bin/gen_biscuit_keys.rs`）で一時生成可能。

## 8) 変更時のチェックリスト（AI がコードを生成/変更する際）
- 変更箇所に対応する feature gate（`cfg(feature = "...")`）の追加/更新を忘れないこと。
- `AppState` にサービスを追加する場合は `AppStateBuilder` に optional フィールドを追加し、`build()` で検査・panic を維持する。
- `AppEvent` を拡張する際は軽量データにし、既存リスナーの挙動と互換性を確認する。リスナーは必ず冪等であること。
- エンドポイントの変更は OpenAPI (dump_openapi) と insta スナップショットに反映させること。
- テストを追加したら、該当する feature セットで `cargo test --workspace` を実行して CI マトリクスと同等の検証を行う。
- **新規ドメインモデル実装時**:
  - `src/domain/user.rs` を参考テンプレートとする（Value Objects + Entity 統合パターン）
  - エラーは `src/common/types.rs` の `DomainError` を拡張して追加
  - リポジトリポートは `src/application/ports/repositories.rs` で trait を定義
  - ビジネスルール違反は domain layer で検出・防御（infrastructure layer に委ねない）

## 9) 参考（必読）
- `src/domain/user.rs` — Value Objects + Entity 統合パターンの完成版（480行, 18 tests）
- `src/common/types.rs` — 三層エラー型階層とResult型エイリアス（229行）
- `src/infrastructure/app_state.rs` — 新AppState実装（Phase 5、DDD準拠）
- `src/events.rs` — AppEvent enum / EventBus（イベント設計の単一の出発点）
- `src/listeners.rs` — イベントリスナーの起動と実装方針
- `src/error.rs` — 既存 AppError と HTTP マッピング
- `.github/workflows/ci.yml` — CI の実行手順と feature matrix（ローカル検証はここを参照）
- `RESTRUCTURE_PLAN.md` と `RESTRUCTURE_EXAMPLES.md` — 現在の再編計画と実装例（方針確認用）
- `.github/instructions/codacy.instructions.md` — Codacy CLI 連携ルール（ファイル編集後はコマンド実行が必須なルールあり）
- `PHASE4_FINAL_STATUS.md` と `PHASE5_STRATEGY_DECISION.md` — Phase 4/5 進捗状況

---

このドキュメントを基に自動生成や修正を行います。内容に不備や追加して欲しいリスト（例: 他の重要なファイル、よくある失敗例、開発者ごとの運用慣習）があれば教えてください。
