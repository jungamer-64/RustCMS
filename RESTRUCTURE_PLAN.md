# RustCMS 構造再編計画

## 📋 概要

本ドキュメントは、RustCMS をよりRustらしい安全で役割ごとに分割された構造に再編するための包括的な計画です。現在のコードベースは機能的には優れていますが、以下の点でさらなる改善の余地があります。

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

## 🚨 リスクと対策

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
