# RustCMS 構造再編計画

## 📋 概要

本ドキュメントは、RustCMS をよりRustらしい安全で役割ごとに分割された構造に再編するための包括的な計画です。現在のコードベースは機能的には優れていますが、以下の点でさらなる改善の余地があります。

## 🔍 監査フィードバック（重要）

### 概要評価

- **アーキテクチャ設計**: ⭐⭐⭐⭐⭐ 優れている
- **実装例の質**: ⭐⭐⭐⭐☆ 高品質だがトランザクション不足
- **移行計画の実現性**: ⭐⭐⭐☆☆ 期間が楽観的
- **リスク管理**: ⭐⭐⭐☆☆ パフォーマンス評価が不十分
- **ドキュメント品質**: ⭐⭐⭐⭐⭐ 非常に詳細
- **総合スコア**: ⭐⭐⭐⭐☆ (4.0/5.0)

### 🔴 重大な懸念事項と対応

#### 1. Diesel ORM との相性問題

**問題**: DTO変換が verbose。ボイラープレートコード肥大化リスク

```rust
// 各エンティティに必要な変換コード（20-30行 × 多数エンティティ）
impl From<UserModel> for User {
    fn from(model: UserModel) -> Self {
        // 多数のフィールド変換
    }
}
```

**推奨対応**:

- **短期**: Diesel継続しつつ、マクロで変換コード最小化
- **中期**: 必要に応じてSQLxへの段階的移行を検討
- **実装例**: `#[derive(FromDbModel)]` カスタムマクロの作成

#### 2. 現実的なスケジュール見直し（重要）

**当初推定**: 7-11週間 → **修正推定**: 12-18週間（3-4.5ヶ月）

**根拠**:

- 既存4000+テストの移行コスト
- イベントリスナーの複雑な移行
- フィーチャーゲート検証

**段階別予測**:

- Phase 1: 2-3週間（+1週間余裕）
- Phase 2: 3-4週間（+1週間余裕）
- Phase 3: 3-4週間（+1週間余裕）
- Phase 4: 2-3週間（+1週間余裕）
- Phase 5: 2週間（+1週間余裕）

#### 3. トランザクション管理が必須

**問題**: Unit of Work パターンなし → データ一貫性リスク

**対応**:

```rust
// Unit of Work パターン例
pub struct UnitOfWork {
    users: Arc<UserRepository>,
    posts: Arc<PostRepository>,
    tx: Transaction,
}

impl UnitOfWork {
    pub async fn commit(self) -> Result<()> {
        self.tx.commit().await
    }

    pub async fn rollback(self) -> Result<()> {
        self.tx.rollback().await
    }
}
```

#### 4. パフォーマンスベンチマーク基準 (+5% 以内)

**対応**: Migration 前後で `cargo bench` を実行し、性能劣化を検証します。

##### 測定対象エンドポイント（10個）

| エンドポイント | メソッド | 説明 | 目標レイテンシ (p95) |
|-------------|---------|------|---------------------|
| `/api/v1/users/:id` | GET | ユーザー詳細取得 | < 50ms |
| `/api/v1/users` | POST | ユーザー登録 | < 150ms |
| `/api/v1/posts/:id` | GET | 投稿詳細取得 | < 70ms |
| `/api/v1/posts` | POST | 投稿作成 | < 200ms |
| `/api/v1/posts` | GET | 投稿一覧取得 (ページネーション) | < 100ms |
| `/api/v1/comments` | POST | コメント追加 | < 120ms |
| `/api/v1/search` | GET | 全文検索 | < 300ms |
| `/api/v1/auth/login` | POST | ログイン | < 200ms |
| `/api/v1/tags` | GET | タグ一覧 | < 50ms |
| `/api/v1/analytics/summary` | GET | 集計クエリ | < 500ms |

##### ベンチマーク実行手順

```bash
# === Phase 0: 基準測定 (構造再編開始前) ===

# 1. 現在の main ブランチで測定
git checkout main
cargo build --release --features "database,cache,search,auth"

# 2. ベンチマーク実行 (Criterion.rs)
cargo bench --bench api_benchmarks -- --save-baseline before

# 3. 結果の保存
cp target/criterion/*/base/estimates.json benches/baseline_before.json

# 4. メモリ使用量の測定
valgrind --tool=massif --massif-out-file=massif.out.before \
  cargo run --release --bin cms-server

# 5. データベースクエリ数の記録
psql -U postgres -d cms_test -c "\
  SELECT query, calls, total_time, mean_time \
  FROM pg_stat_statements \
  ORDER BY total_time DESC LIMIT 20;" \
  > benches/db_queries_before.txt
```

##### Phase 別のベンチマーク再実行

**Phase 1-2 完了時**:

```bash
cargo bench --bench api_benchmarks -- --baseline before
# 期待: 劣化 ±2% 以内 (Value Objects/Entities は影響小)
```

**Phase 3 完了時**:

```bash
cargo bench --bench api_benchmarks -- --baseline before
# 期待: 劣化 +3% 以内 (Repository 抽象化コスト)
```

**Phase 4 完了時**:

```bash
cargo bench --bench api_benchmarks -- --baseline before
# 期待: 劣化 +5% 以内 (ハンドラー再実装コスト)
```

**Phase 5 完了時**:

```bash
cargo bench --bench api_benchmarks -- --baseline before
# 期待: 改善 -2% または同等 (旧コード削除による最適化)
```

##### 許容範囲と対応

| 劣化度 | 判定 | 対応 |
|-------|------|-----|
| **0~2%** | ✅ 優秀 | そのまま継続 |
| **2~5%** | ⚠️ 許容範囲 | 原因を記録、Phase 5 で最適化 |
| **5~10%** | 🔶 要調査 | ホットパスを特定し、最適化実施 |
| **>10%** | 🚨 Critical | ロールバック検討、設計見直し |

##### ベンチマーク結果の記録

**benches/results.md** に各 Phase の結果を記録:

```markdown
## Phase 0: 基準測定 (2025-10-16)

| エンドポイント | p50 | p95 | p99 |
|-------------|-----|-----|-----|
| GET /users/:id | 23ms | 45ms | 78ms |
| POST /users | 89ms | 142ms | 210ms |
| ...

## Phase 3 完了 (2025-11-20)

| エンドポイント | p50 | p95 | p99 | 変化率 |
|-------------|-----|-----|-----|-------|
| GET /users/:id | 24ms | 47ms | 81ms | +4.4% |
| POST /users | 92ms | 148ms | 215ms | +4.2% |
| ...

**判定**: ⚠️ 許容範囲内 (+4.2% 平均)。Phase 5 で最適化予定。
```

##### メモリプロファイリング

```bash
# Phase 0
cargo build --release
valgrind --tool=massif --massif-out-file=massif.out.phase0 \
  cargo run --release --bin cms-server &
sleep 60 && pkill cms-server
ms_print massif.out.phase0 > benches/memory_phase0.txt

# Phase 3-5 で同様に実行
# 期待: メモリ使用量 +10% 以内
```

##### データベースクエリ最適化

```bash
# Phase 0: 基準
psql -U postgres -d cms_test -c "SELECT COUNT(*) FROM pg_stat_statements;"
# 出力: 1200 queries

# Phase 3: Repository 抽象化後
# 期待: 1250 queries 以内 (+4% 以内)

# Phase 5: 最適化後
# 期待: 1180 queries (-2% 改善)
```

#### 5. ハイブリッドアプローチの導入

**すべて を抽象化しない**。パフォーマンスクリティカル部分は直接SQL:

```rust
// 通常エンドポイント: Repository パターン
pub async fn get_user(id: UserId) -> Result<UserDto> {
    self.user_repo.find_by_id(id).await
}

// パフォーマンスクリティカル: 直接SQL
#[inline]
pub async fn get_user_feed_optimized(id: UserId) -> Result<Vec<Post>> {
    sqlx::query_as!(...)
        .fetch_all(&self.pool)
        .await
}
```

#### 6. API バージョニング戦略

**実装**:

- v1（既存）と v2（新）を並行運用
- Migration 期間: 2リリースサイクル
- v1 終了予告: 3リリースサイクル前

### ✅ 推奨アクション（Phase 1 開始前）

**即座に実行**:

1. トランザクション管理 → Unit of Work 実装
2. ベンチマーク基準 → 移行前の性能測定
3. スケジュール確認 → 12-18週間の確保

**Phase 1 開始時**:

1. キャッシュ戦略文書化 → Decorator パターン
2. イベント統合方針明確化 → 既存リスナーとの互換性
3. API バージョニング詳細設計 → v1/v2 並行運用

## 🎯 再編の目的

1. **ドメイン駆動設計（DDD）の導入**: ビジネスロジックを明確に分離
2. **Rustのベストプラクティスの徹底**: 型安全性、エラーハンドリング、所有権の活用
3. **関心の分離**: レイヤードアーキテクチャの明確化
4. **テスタビリティの向上**: モックとDIの容易化
5. **保守性の向上**: モジュール間の依存関係の最小化

## 📊 現状分析

### 現在の構造の長所

✅ **機能別モジュール分割が明確**

- `handlers/`, `repositories/`, `models/` の3層構造
- Feature フラグによる柔軟な機能ON/OFF

✅ **イベント駆動アーキテクチャ**

- `events.rs` + `listeners.rs` による疎結合設計
- 横断的関心事の分離が実現済み

✅ **セキュリティへの配慮**

- `utils/security_validation.rs` による入力検証
- 安全なエンコーディング処理

### 現在の構造の課題

⚠️ **ドメインロジックの分散**

```text
問題: ビジネスロジックが handlers, repositories, models に分散
影響: 変更時に複数ファイルを修正する必要があり、整合性維持が困難
```

⚠️ **肥大化したモジュール**

```text
src/app.rs (2080行)
src/handlers/* (各ファイルが多機能)
src/utils/* (28個のユーティリティモジュール)
```

⚠️ **型安全性の不足**

```rust
// 例: 文字列ベースの識別子
pub fn get_user(&self, id: &str) -> Result<User>

// 望ましい形: NewType パターン
pub fn get_user(&self, id: UserId) -> Result<User>
```

⚠️ **レイヤー間の密結合**

```rust
// handlers が database の実装詳細に依存
#[cfg(feature = "database")]
pub async fn create_post(state: AppState) {
    state.database.pool.get()... // 直接プール操作
}
```

## 🏗️ 提案する新構造

### 1. ディレクトリ構造（レイヤードアーキテクチャ）

```text
src/
├── domain/                    # ドメイン層（ビジネスロジック）
│   ├── mod.rs
│   ├── entities/             # エンティティ（ビジネスオブジェクト）
│   │   ├── mod.rs
│   │   ├── user.rs          # User エンティティ + ビジネスルール
│   │   ├── post.rs          # Post エンティティ + ビジネスルール
│   │   └── api_key.rs
│   ├── value_objects/        # 値オブジェクト（不変、検証済み）
│   │   ├── mod.rs
│   │   ├── user_id.rs       # NewType パターン: struct UserId(Uuid)
│   │   ├── email.rs         # 検証済みEmail
│   │   ├── username.rs      # 検証済みUsername
│   │   ├── slug.rs          # 検証済みSlug
│   │   └── password.rs      # 検証済みPassword（ハッシュ化）
│   ├── services/             # ドメインサービス（複数エンティティにまたがるロジック）
│   │   ├── mod.rs
│   │   ├── user_service.rs  # ユーザー登録、認証ロジック
│   │   ├── post_service.rs  # 投稿公開、タグ管理
│   │   └── permission_service.rs
│   ├── events/               # ドメインイベント
│   │   ├── mod.rs
│   │   ├── user_events.rs
│   │   └── post_events.rs
│   └── errors.rs             # ドメイン固有エラー
│
├── application/              # アプリケーション層（ユースケース）
│   ├── mod.rs
│   ├── dto/                  # Data Transfer Objects
│   │   ├── mod.rs
│   │   ├── user_dto.rs
│   │   └── post_dto.rs
│   ├── commands/             # コマンド（書き込み操作）
│   │   ├── mod.rs
│   │   ├── create_user.rs
│   │   ├── update_post.rs
│   │   └── delete_user.rs
│   ├── queries/              # クエリ（読み取り操作）- CQRS パターン
│   │   ├── mod.rs
│   │   ├── get_user_by_id.rs
│   │   ├── list_posts.rs
│   │   └── search_posts.rs
│   ├── ports/                # ポート（インターフェース定義）
│   │   ├── mod.rs
│   │   ├── user_repository.rs    # trait UserRepository
│   │   ├── post_repository.rs    # trait PostRepository
│   │   ├── cache_service.rs      # trait CacheService
│   │   ├── search_service.rs     # trait SearchService
│   │   └── event_publisher.rs    # trait EventPublisher
│   └── use_cases/            # ユースケース実装
│       ├── mod.rs
│       ├── user/
│       │   ├── register_user.rs
│       │   ├── login_user.rs
│       │   └── update_profile.rs
│       └── post/
│           ├── create_post.rs
│           ├── publish_post.rs
│           └── delete_post.rs
│
├── infrastructure/           # インフラストラクチャ層（技術的実装）
│   ├── mod.rs
│   ├── database/            # データベース実装（Diesel）
│   │   ├── mod.rs
│   │   ├── connection.rs    # 接続プール管理
│   │   ├── schema.rs        # Diesel スキーマ
│   │   ├── repositories/    # リポジトリの具体実装
│   │   │   ├── mod.rs
│   │   │   ├── user_repository_impl.rs  # impl UserRepository
│   │   │   └── post_repository_impl.rs
│   │   └── models/          # DB モデル（Diesel用）
│   │       ├── mod.rs
│   │       ├── user_model.rs
│   │       └── post_model.rs
│   ├── cache/               # キャッシュ実装（Redis）
│   │   ├── mod.rs
│   │   ├── redis_cache.rs   # impl CacheService
│   │   └── memory_cache.rs
│   ├── search/              # 検索実装（Tantivy）
│   │   ├── mod.rs
│   │   ├── tantivy_search.rs  # impl SearchService
│   │   └── indexer.rs
│   ├── auth/                # 認証実装（biscuit-auth）
│   │   ├── mod.rs
│   │   ├── biscuit_auth.rs
│   │   ├── webauthn.rs
│   │   └── session_store.rs
│   ├── events/              # イベント実装
│   │   ├── mod.rs
│   │   ├── event_bus.rs     # impl EventPublisher
│   │   └── listeners/
│   │       ├── search_listener.rs
│   │       └── cache_listener.rs
│   └── config/              # 設定管理
│       ├── mod.rs
│       └── settings.rs
│
├── presentation/             # プレゼンテーション層（Web API）
│   ├── mod.rs
│   ├── http/
│   │   ├── mod.rs
│   │   ├── routes.rs        # ルート定義
│   │   ├── handlers/        # HTTPハンドラ（薄い層）
│   │   │   ├── mod.rs
│   │   │   ├── user_handlers.rs
│   │   │   ├── post_handlers.rs
│   │   │   ├── auth_handlers.rs
│   │   │   └── health_handlers.rs
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   ├── auth_middleware.rs
│   │   │   ├── rate_limit.rs
│   │   │   └── logging.rs
│   │   ├── extractors/      # Axum extractors
│   │   │   ├── mod.rs
│   │   │   ├── authenticated_user.rs
│   │   │   └── pagination.rs
│   │   └── responses/       # HTTP レスポンス
│   │       ├── mod.rs
│   │       ├── api_response.rs
│   │       └── error_response.rs
│   └── openapi/             # OpenAPI ドキュメント
│       ├── mod.rs
│       └── specs.rs
│
├── shared/                   # 共有ユーティリティ
│   ├── mod.rs
│   ├── types/               # 共通型定義
│   │   ├── mod.rs
│   │   ├── result.rs        # 統一Result型
│   │   └── pagination.rs
│   ├── telemetry/           # 監視・ロギング
│   │   ├── mod.rs
│   │   ├── tracing.rs
│   │   └── metrics.rs
│   └── utils/               # 純粋関数ユーティリティ
│       ├── mod.rs
│       ├── datetime.rs
│       ├── encoding.rs
│       └── validation.rs
│
├── lib.rs                   # ライブラリルート
└── main.rs                  # アプリケーションエントリーポイント
```

### 2. 主要パターンの適用

#### 2.1 NewType パターン（型安全性）

**Before:**

```rust
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

// 問題: 型エラーを検出できない
fn get_user(id: Uuid) -> User { ... }
fn get_post(id: Uuid) -> Post { ... }

// 誤用例（コンパイルエラーにならない）
let user_id = user.id;
let post = get_post(user_id); // 本来はエラーであるべき
```

**After:**

```rust
// domain/value_objects/user_id.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

// domain/value_objects/post_id.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostId(Uuid);

// これで型エラーが検出される
fn get_user(id: UserId) -> User { ... }
fn get_post(id: PostId) -> Post { ... }

let user_id = UserId::new();
let post = get_post(user_id); // コンパイルエラー！
```

#### 2.2 検証済み値オブジェクト

**Before:**

```rust
// handlers/users.rs
pub async fn create_user(payload: CreateUserRequest) -> Result<User> {
    // バリデーションがハンドラ層に散在
    if payload.email.is_empty() {
        return Err(AppError::BadRequest("Email required".into()));
    }
    if !payload.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email".into()));
    }
    // ... ビジネスロジック
}
```

**After:**

```rust
// domain/value_objects/email.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    /// メールアドレスを検証して作成
    pub fn new(value: String) -> Result<Self, DomainError> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    fn validate(value: &str) -> Result<(), DomainError> {
        if value.is_empty() {
            return Err(DomainError::InvalidEmail("Email required".into()));
        }
        if !value.contains('@') {
            return Err(DomainError::InvalidEmail("Invalid format".into()));
        }
        if value.len() > 254 {
            return Err(DomainError::InvalidEmail("Email too long".into()));
        }
        // より厳密な検証（RFC 5322準拠）
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Email {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// handlers/users.rs
pub async fn create_user(payload: CreateUserRequest) -> Result<User> {
    // バリデーションは型レベルで保証される
    let email = Email::new(payload.email)?; // ここで検証完了
    let username = Username::new(payload.username)?;

    // 以降は検証済みデータとして扱える
    user_service.register(email, username).await
}
```

#### 2.3 リポジトリパターン（依存性逆転）

**Before:**

```rust
// handlers/users.rs
pub async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<User>> {
    // ハンドラが直接データベース実装に依存
    let user = state.database.get_user_by_id(&id).await?;
    Ok(Json(user))
}
```

**After:**

```rust
// application/ports/user_repository.rs
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;
    async fn delete(&self, id: UserId) -> Result<(), RepositoryError>;
}

// infrastructure/database/repositories/user_repository_impl.rs
pub struct DieselUserRepository {
    pool: DbPool,
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        // Diesel実装の詳細
    }
}

// application/use_cases/user/get_user_by_id.rs
pub struct GetUserByIdUseCase<R: UserRepository> {
    user_repo: Arc<R>,
}

impl<R: UserRepository> GetUserByIdUseCase<R> {
    pub async fn execute(&self, id: UserId) -> Result<UserDto, ApplicationError> {
        let user = self.user_repo.find_by_id(id).await?
            .ok_or(ApplicationError::UserNotFound)?;
        Ok(UserDto::from(user))
    }
}

// presentation/http/handlers/user_handlers.rs
pub async fn get_user(
    Path(id): Path<Uuid>,
    State(use_case): State<Arc<GetUserByIdUseCase<DieselUserRepository>>>,
) -> Result<Json<ApiResponse<UserDto>>> {
    let user_id = UserId::from_uuid(id);
    let user_dto = use_case.execute(user_id).await?;
    Ok(Json(ApiResponse::success(user_dto)))
}
```

#### 2.4 CQRS パターン（読み書き分離）

```rust
// application/commands/create_post.rs
pub struct CreatePostCommand {
    pub title: String,
    pub content: String,
    pub author_id: UserId,
}

pub struct CreatePostHandler<R: PostRepository, E: EventPublisher> {
    repo: Arc<R>,
    events: Arc<E>,
}

impl<R: PostRepository, E: EventPublisher> CreatePostHandler<R, E> {
    pub async fn handle(&self, cmd: CreatePostCommand) -> Result<PostId, ApplicationError> {
        // 1. ドメインエンティティを作成
        let post = Post::create(
            Title::new(cmd.title)?,
            Content::new(cmd.content)?,
            cmd.author_id,
        )?;

        // 2. 永続化
        self.repo.save(&post).await?;

        // 3. イベント発行
        self.events.publish(PostCreatedEvent::new(post.id())).await?;

        Ok(post.id())
    }
}

// application/queries/list_posts.rs
pub struct ListPostsQuery {
    pub page: u32,
    pub per_page: u32,
    pub author_id: Option<UserId>,
}

pub struct ListPostsHandler<R: PostRepository> {
    repo: Arc<R>,
}

impl<R: PostRepository> ListPostsHandler<R> {
    pub async fn handle(&self, query: ListPostsQuery) -> Result<Page<PostDto>, ApplicationError> {
        let posts = self.repo.find_paginated(
            query.page,
            query.per_page,
            query.author_id,
        ).await?;

        let dtos = posts.into_iter().map(PostDto::from).collect();
        Ok(Page::new(dtos, query.page, query.per_page))
    }
}
```

#### 2.5 Result型の統一とエラーハンドリング

```rust
// domain/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    #[error("Post cannot be published: {0}")]
    CannotPublish(String),
}

// application/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("User not found")]
    UserNotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

// infrastructure/database/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),
}

// presentation/http/responses/error_response.rs
impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApplicationError::UserNotFound => {
                (StatusCode::NOT_FOUND, "USER_NOT_FOUND", self.to_string())
            }
            ApplicationError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string())
            }
            ApplicationError::Domain(e) => {
                (StatusCode::BAD_REQUEST, "DOMAIN_ERROR", e.to_string())
            }
            ApplicationError::Repository(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "REPOSITORY_ERROR", "Internal error".to_string())
            }
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}
```

### 3. 依存性注入とテスタビリティ

```rust
// lib.rs
pub struct AppContainer {
    // Repositories
    user_repo: Arc<dyn UserRepository>,
    post_repo: Arc<dyn PostRepository>,

    // Services
    cache_service: Arc<dyn CacheService>,
    search_service: Arc<dyn SearchService>,
    event_publisher: Arc<dyn EventPublisher>,

    // Use cases
    create_user: Arc<CreateUserHandler>,
    get_user: Arc<GetUserByIdUseCase<dyn UserRepository>>,
    // ... etc
}

impl AppContainer {
    pub async fn new(config: Config) -> Result<Self> {
        // Infrastructure layer
        let db_pool = create_db_pool(&config).await?;
        let user_repo = Arc::new(DieselUserRepository::new(db_pool.clone()));
        let post_repo = Arc::new(DieselPostRepository::new(db_pool));

        let cache_service = Arc::new(RedisCache::new(&config).await?);
        let search_service = Arc::new(TantivySearch::new(&config)?);
        let event_publisher = Arc::new(EventBus::new());

        // Application layer
        let create_user = Arc::new(CreateUserHandler::new(
            user_repo.clone(),
            event_publisher.clone(),
        ));

        let get_user = Arc::new(GetUserByIdUseCase::new(user_repo.clone()));

        Ok(Self {
            user_repo,
            post_repo,
            cache_service,
            search_service,
            event_publisher,
            create_user,
            get_user,
        })
    }
}

// テストでのモック使用
#[cfg(test)]
mod tests {
    use super::*;

    struct MockUserRepository {
        users: Mutex<HashMap<UserId, User>>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
            Ok(self.users.lock().unwrap().get(&id).cloned())
        }

        // ... other methods
    }

    #[tokio::test]
    async fn test_get_user_use_case() {
        let mock_repo = Arc::new(MockUserRepository::new());
        let use_case = GetUserByIdUseCase::new(mock_repo.clone());

        // テストデータの準備
        let user_id = UserId::new();
        mock_repo.insert(user_id, create_test_user());

        // テスト実行
        let result = use_case.execute(user_id).await;
        assert!(result.is_ok());
    }
}
```

## 📅 移行計画（段階的リファクタリング）

### Phase 1: 基礎固め（1-2週間）

**目標**: 新しい構造の基盤を作成し、既存コードと並行稼働

1. **新ディレクトリ構造の作成**

   ```bash
   mkdir -p src/{domain,application,infrastructure,presentation,shared}
   mkdir -p src/domain/{entities,value_objects,services,events}
   mkdir -p src/application/{dto,commands,queries,ports,use_cases}
   # ... etc
   ```

2. **共通型定義の移行**
   - `shared/types/` の作成
   - Result型の統一
   - エラー型階層の定義

3. **Value Objects の実装**
   - `UserId`, `PostId`, `Email`, `Username` などを `domain/value_objects/` に作成
   - 検証ロジックを型レベルに移動

4. **Port定義（インターフェース）**
   - `application/ports/` に trait 定義
   - 既存のリポジトリメソッドをインターフェースとして抽出

**検証**: 新旧両方の構造でビルドが通ること

### Phase 2: ドメイン層の構築（2-3週間）

**目標**: ビジネスロジックをドメイン層に集約

1. **エンティティの移行**
   - `models/user.rs` → `domain/entities/user.rs`
   - ビジネスルールをメソッドとして実装
   - 不変条件を型システムで保証

2. **ドメインサービスの抽出**
   - 複数エンティティにまたがるロジックを抽出
   - 認証、権限管理などのロジックを移動

3. **ドメインイベントの定義**
   - 既存の `events.rs` を `domain/events/` に分割
   - イベント駆動設計の強化

**検証**: ドメイン層のユニットテスト作成

### Phase 3: アプリケーション層の構築（2-3週間）

**目標**: ユースケースを明確に定義

1. **DTOの作成**
   - HTTPリクエスト/レスポンス用の型を定義
   - ドメインエンティティとの変換ロジック

2. **Use Caseの実装**
   - 既存のハンドラからビジネスロジックを抽出
   - CQRSパターンでコマンドとクエリを分離

3. **リポジトリ実装の移行**
   - `infrastructure/database/repositories/` に実装を移動
   - Port（trait）を実装する形に変更

**検証**: アプリケーション層の統合テスト作成

### Phase 4: プレゼンテーション層のリファクタリング（1-2週間）

**目標**: ハンドラを薄い層に変更

1. **ハンドラの簡素化**
   - ビジネスロジックを全てUse Caseに委譲
   - HTTPリクエスト/レスポンスの変換のみを担当

2. **ミドルウェアの整理**
   - 認証、レート制限などを `presentation/http/middleware/` に集約

3. **OpenAPI仕様の更新**
   - 新しいDTO構造に合わせて更新

**検証**: E2Eテストによる動作確認

## 🔀 並行開発ポリシー

### 原則

構造再編期間中も **緊急バグ修正** と **クリティカルな機能追加** は継続できるようにします。ただし、開発の混乱を避けるため、以下のルールを設けます。

### Phase 別ポリシー

#### Phase 1-2 (週1-7): 機能フリーズ期間

**ルール**: **新機能追加は原則禁止** (ドメイン層の基礎を固めるため)

- ✅ **許可**: クリティカルなバグ修正（セキュリティ、データ損失リスク）
- ✅ **許可**: ドキュメント更新、テストの追加
- ❌ **禁止**: 新エンドポイント追加
- ❌ **禁止**: 既存エンドポイントの大幅な変更

**緊急対応フロー**:

```bash
# 1. main ブランチから緊急修正ブランチを作成
git checkout main
git pull origin main
git checkout -b hotfix/critical-bug-123

# 2. 修正実装とテスト
cargo test --workspace

# 3. PR 作成 (レビュー必須)
gh pr create --title "[HOTFIX] Critical Bug #123" --label "hotfix"

# 4. マージ後、Phase 1-2 ブランチに cherry-pick
git checkout phase2-domain-layer
git cherry-pick <hotfix-commit-hash>
```

#### Phase 3 (週8-11): 限定的な新機能許可

**ルール**: **軽微な機能追加のみ許可** (旧構造で実装)

- ✅ **許可**: 既存エンドポイントへのパラメータ追加
- ✅ **許可**: バグ修正、パフォーマンス改善
- ⚠️ **条件付き許可**: 新エンドポイント（旧ハンドラーで実装し、Phase 4 で移行）
- ❌ **禁止**: Application Layer の直接変更

**新機能追加フロー**:

```bash
# 1. 旧構造 (src/handlers/) で実装
# src/handlers/new_feature.rs
pub async fn new_endpoint(/* ... */) -> Result<Json<Response>> {
    // 旧スタイルで実装
}

# 2. Phase 4 移行リストに追加
echo "- [ ] new_endpoint の移行" >> MIGRATION_CHECKLIST.md

# 3. マージ後、Phase 4 で新構造に移行
```

#### Phase 4-5 (週12-16): 新構造への移行期間

**ルール**: **新機能は新構造でのみ実装**

- ✅ **推奨**: 新エンドポイントは `/api/v2` で実装（新ハンドラー）
- ✅ **許可**: 旧エンドポイント (`/api/v1`) のバグ修正
- ❌ **禁止**: 旧ハンドラー (`src/handlers/`) への新機能追加

**新機能追加フロー**:

```bash
# 1. 新構造で実装
# src/presentation/http/handlers/new_feature.rs
pub async fn new_endpoint_v2(
    State(app_state): State<AppState>,
    Json(request): Json<NewFeatureRequest>,
) -> Result<Json<NewFeatureResponse>> {
    // Use Case 経由で実装
    let use_case = app_state.container.new_feature_use_case();
    let result = use_case.execute(request).await?;
    Ok(Json(result))
}

# 2. /api/v2 ルーティングに追加
app.route("/api/v2/new-feature", post(new_endpoint_v2))
```

### 競合解決ガイドライン

#### 旧構造と新構造の衝突時

**優先順位**:

1. **セキュリティ修正**: 最優先（両方に適用）
2. **データ整合性バグ**: 高優先（両方に適用）
3. **新機能**: Phase に応じて旧 or 新で実装

**衝突例と対応**:

```rust
// 例: User エンティティに新フィールド追加が必要

// Phase 1-2 中の対応
// → 旧 models/user.rs に追加し、Phase 2 完了後に domain/entities/user.rs に移行

// Phase 3-4 中の対応
// → domain/entities/user.rs に直接追加（新構造が優先）
```

### コミュニケーションルール

- **Slack/Discord**: `#restructure-wip` チャンネルで進捗共有
- **PR ラベル**: `restructure-phase-N` ラベルで Phase 識別
- **週次ミーティング**: 毎週金曜に進捗と競合確認

### ドキュメント更新義務

新機能追加時は以下を更新:

- [ ] `CHANGELOG.md` にエントリ追加
- [ ] 該当 Phase の `MIGRATION_CHECKLIST.md` に移行タスク追加（Phase 3 以降）
- [ ] API ドキュメント (`docs/API.md`) 更新

---

### Phase 5: クリーンアップと最適化（1週間）

**目標**: 古い構造を削除し、ドキュメント更新

1. **旧コードの削除**
   - `src/handlers/`, `src/repositories/`, `src/models/` の削除
   - `src/utils/` の必要最小限への削減

2. **ドキュメント更新**
   - ARCHITECTURE.md の全面改訂
   - 各モジュールのREADME作成

3. **パフォーマンス検証**
   - ベンチマークテストの実行
   - 必要に応じて最適化

**検証**: 全テストスイートの実行、カバレッジ確認

## 🎓 学習リソース

### Rustのベストプラクティス

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Zero To Production In Rust](https://www.zero2prod.com/)

### アーキテクチャパターン

- [Domain-Driven Design (DDD)](https://martinfowler.com/bliki/DomainDrivenDesign.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)

## 📊 期待される効果

### 1. 型安全性の向上

```rust
// Before: ランタイムエラーの可能性
fn transfer(from: Uuid, to: Uuid, amount: f64)

// After: コンパイル時にエラー検出
fn transfer(from: AccountId, to: AccountId, amount: Money)
```

### 2. テスタビリティの向上

- モックとスタブの容易化
- ユニットテスト、統合テスト、E2Eテストの明確な分離

### 3. 保守性の向上

- 変更の影響範囲を最小化
- 新機能追加時の改修箇所が明確

### 4. パフォーマンスの維持

- ゼロコスト抽象化（Rustの強み）
- コンパイル時最適化による高速化

## � Feature Flag 戦略

### 目的

構造再編の各 Phase を **feature flag で段階的に有効化** し、旧構造と新構造を並行稼働させることで、リスクを最小化します。

### 新規 Feature Flags

#### Phase 別フラグ

```toml
# Cargo.toml に追加
[features]
# === 既存フラグ ===
default = ["auth", "cache", "compression", "database", "email", "search"]
auth = ["dep:argon2", "dep:biscuit-auth"]
cache = ["dep:deadpool-redis", "dep:redis"]
database = ["dep:deadpool-diesel", "dep:diesel", ...]
search = ["dep:tantivy"]

# === 構造再編フラグ (Phase 別) ===
restructure_domain = []          # Phase 1-2: Value Objects + Entities
restructure_application = []     # Phase 3: Use Cases + Repositories
restructure_presentation = []    # Phase 4: 新ハンドラー

# === レガシー維持フラグ ===
legacy_handlers = []             # 旧ハンドラーを残す (Phase 4-5 で使用)
legacy_repositories = []         # 旧リポジトリを残す (Phase 3-5 で使用)

# === 統合フラグ ===
full_restructure = [
    "restructure_domain",
    "restructure_application",
    "restructure_presentation"
]
```

### Phase 別の Feature Flag 使用方針

#### Phase 1-2: ドメイン層構築

**有効化**: `restructure_domain`

```rust
// src/domain/mod.rs
#[cfg(feature = "restructure_domain")]
pub mod value_objects;

#[cfg(feature = "restructure_domain")]
pub mod entities;

// 旧コードは引き続き動作
#[cfg(not(feature = "restructure_domain"))]
pub use crate::models::*;
```

**CI ビルド**:

```yaml
# .github/workflows/ci.yml
- name: Build with restructure_domain
  run: cargo build --features "database,cache,search,restructure_domain"

- name: Build without restructure_domain (legacy)
  run: cargo build --features "database,cache,search"
```

#### Phase 3: アプリケーション層構築

**有効化**: `restructure_application` (depends on `restructure_domain`)

```rust
// src/application/mod.rs
#[cfg(feature = "restructure_application")]
pub mod use_cases;

#[cfg(feature = "restructure_application")]
pub mod ports;

// 旧リポジトリは legacy_repositories フラグで維持
#[cfg(all(not(feature = "restructure_application"), feature = "legacy_repositories"))]
pub use crate::repositories::*;
```

**Cargo.toml 依存関係**:

```toml
[features]
restructure_application = ["restructure_domain"]  # domain 必須
```

#### Phase 4: プレゼンテーション層リファクタリング

**有効化**: `restructure_presentation`

```rust
// src/routes/mod.rs
pub fn configure_routes(app: Router) -> Router {
    #[cfg(feature = "restructure_presentation")]
    {
        app.nest("/api/v2", v2_routes())  // 新ハンドラー
    }

    #[cfg(any(not(feature = "restructure_presentation"), feature = "legacy_handlers"))]
    {
        app.nest("/api/v1", v1_routes())  // 旧ハンドラー
    }
}
```

**API バージョニング**:

- `/api/v1`: 旧ハンドラー (`legacy_handlers` フラグで制御)
- `/api/v2`: 新ハンドラー (`restructure_presentation` フラグで制御)

#### Phase 5: クリーンアップ

**無効化**: `legacy_handlers`, `legacy_repositories` を削除

```bash
# Phase 5 開始時
git rm src/handlers/
git rm src/repositories/

# Cargo.toml から legacy フラグを削除
sed -i '/legacy_handlers/d' Cargo.toml
sed -i '/legacy_repositories/d' Cargo.toml
```

### 環境変数による実行時切り替え

**開発環境**: 新旧並行稼働

```bash
# .env
ENABLE_RESTRUCTURE_DOMAIN=true
ENABLE_RESTRUCTURE_APPLICATION=false  # まだ Phase 3 未完了
```

```rust
// src/app.rs
pub fn create_app_state() -> AppState {
    let use_new_domain = std::env::var("ENABLE_RESTRUCTURE_DOMAIN")
        .unwrap_or("false".into()) == "true";

    if use_new_domain {
        #[cfg(feature = "restructure_domain")]
        {
            // 新ドメイン層使用
        }
    } else {
        // 旧モデル使用
    }
}
```

### CI/CD での Feature Flag 検証

```yaml
# .github/workflows/feature-matrix.yml
strategy:
  matrix:
    features:
      # === 既存構造 (baseline) ===
      - "database,cache,search,auth"

      # === Phase 1-2: ドメイン層のみ ===
      - "database,cache,search,auth,restructure_domain"

      # === Phase 3: アプリケーション層追加 ===
      - "database,cache,search,auth,restructure_domain,restructure_application"

      # === Phase 4: 完全移行 ===
      - "database,cache,search,auth,full_restructure"

      # === レガシー維持 (Phase 4-5 移行期) ===
      - "database,cache,search,auth,full_restructure,legacy_handlers"

steps:
  - name: Build with feature set
    run: cargo build --features "${{ matrix.features }}"

  - name: Test with feature set
    run: cargo test --features "${{ matrix.features }}"
```

### Production デプロイメント戦略

#### Stage 1: カナリアリリース (10% トラフィック)

```bash
# デプロイ設定
cargo build --release --features "full_restructure,legacy_handlers"

# Nginx でトラフィック分割
upstream backend {
    server new-backend:8080 weight=1;  # 10%
    server old-backend:8080 weight=9;  # 90%
}
```

#### Stage 2: 段階的拡大 (50% トラフィック)

```bash
# 2週間後、問題なければ50%に
upstream backend {
    server new-backend:8080 weight=5;  # 50%
    server old-backend:8080 weight=5;  # 50%
}
```

#### Stage 3: 完全移行 (100% トラフィック)

```bash
# 4週間後、完全移行
cargo build --release --features "full_restructure"
# legacy_handlers フラグを削除
```

### Feature Flag 削除計画

| Phase | Flag | 削除タイミング |
|-------|------|--------------|
| Phase 1-2 | `restructure_domain` | Phase 5 完了後 (default に統合) |
| Phase 3 | `restructure_application` | Phase 5 完了後 (default に統合) |
| Phase 4 | `restructure_presentation` | Phase 5 完了後 (default に統合) |
| Phase 4-5 | `legacy_handlers` | Phase 5 完了時 (即削除) |
| Phase 3-5 | `legacy_repositories` | Phase 5 完了時 (即削除) |

### ドキュメント記載

`README.md` に Feature Flags セクションを追加:

```markdown
## Feature Flags

### 構造再編関連 (Phase 1-5)

- `restructure_domain`: 新ドメイン層を有効化 (Value Objects, Entities)
- `restructure_application`: 新アプリケーション層を有効化 (Use Cases, Repositories)
- `restructure_presentation`: 新プレゼンテーション層を有効化 (`/api/v2`)
- `full_restructure`: 上記すべてを有効化

### レガシー維持 (移行期のみ)

- `legacy_handlers`: 旧ハンドラー (`/api/v1`) を維持
- `legacy_repositories`: 旧リポジトリを維持

**推奨**: Phase 5 完了後は `full_restructure` をデフォルトに統合し、legacy フラグは削除されます。
```

---

## �🚨 リスクと対策

### リスク1: 移行期間中の開発停滞

**対策**:

- 機能追加は一時凍結し、リファクタリングに集中
- 各フェーズごとに動作確認を徹底

### リスク2: パフォーマンスの劣化

**対策**:

- 各フェーズでベンチマークテストを実行
- ボトルネックが見つかった場合は即座に最適化

### リスク3: テストカバレッジの低下

**対策**:

- 移行前にカバレッジを測定
- 各フェーズで同等以上のカバレッジを維持

## ✅ チェックリスト

### 移行完了の条件

- [ ] 全テストがパスする
- [ ] テストカバレッジが移行前と同等以上
- [ ] ベンチマークテストで性能劣化がない
- [ ] Clippy警告がゼロ
- [ ] ドキュメントが更新されている
- [ ] 既存APIの互換性が保たれている

## 📝 まとめ

本計画は、RustCMSをより安全で保守しやすい構造に再編するための包括的なロードマップです。段階的なアプローチにより、リスクを最小化しながら、モダンなRustのベストプラクティスを適用します。

**次のステップ**:

1. この計画をチームでレビュー
2. Phase 1の作業を開始
3. 週次で進捗を確認し、必要に応じて計画を調整
