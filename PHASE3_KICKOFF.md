# Phase 3: Application Layer 構築 — キックオフドキュメント

**開始日**: 2025年10月18日  
**期間**: Week 8-11（3-4週間）  
**目標**: Use Cases + DTOs + Repository 実装 + CQRS

---

## 📊 Phase 3 目標

| カテゴリ | 目標 | 優先度 |
|---------|------|--------|
| **DTOs** | 6-8個 | 🔴 最高 |
| **Use Cases** | 8-10個 | 🔴 最高 |
| **Repository 実装** | 5個 | 🟡 高 |
| **CQRS Queries** | 3-5個 | 🟡 高 |
| **Unit of Work** | 1個 | 🟢 中 |
| **統合テスト** | 30個+ | 🟡 高 |

---

## 🎯 Phase 3 完了基準

### 必須（Must Have）

- [x] Phase 2 完了（Domain Layer 構築）
- [ ] すべての DTO が Domain Entity から変換可能
- [ ] すべての Use Case がトランザクション境界を明示
- [ ] すべての Repository が Port を実装
- [ ] CQRS で読み書きが分離されている
- [ ] 統合テスト 30個以上実装
- [ ] すべてのテストがパス（340 + 30 = 370個）

### 推奨（Should Have）

- [ ] Use Case が mockall でテスト可能
- [ ] Query 層でキャッシュ統合
- [ ] Unit of Work でトランザクション管理
- [ ] パフォーマンステスト実施

### 任意（Nice to Have）

- [ ] GraphQL API 実装
- [ ] WebSocket 統合
- [ ] リアルタイム通知機能

---

## 📅 Week 8-9: DTO + Use Case 実装（2週間）

### 🎯 目標

- **DTOs**: 6-8個実装（User/Post/Comment + Request/Response 型）
- **Use Cases**: 8-10個実装（User 4個 + Post 4個 + Comment 2個）
- **テスト**: 各 Use Case に単体テスト実装

### 📋 タスク詳細

#### 1. DTO 実装（6-8個）

##### User DTOs
```rust
// src/application/dto/user.rs
pub struct UserDto                  // レスポンス用
pub struct CreateUserRequest        // 登録用
pub struct UpdateUserRequest        // 更新用
pub struct UserListDto              // 一覧用（最小限フィールド）
```

**実装方針**:
- Domain Entity → DTO: `impl From<User> for UserDto`
- DTO → Domain Entity: コンストラクタで検証
- Validation: `validator` crate 使用（メールアドレス等）

##### Post DTOs
```rust
// src/application/dto/post.rs
pub struct PostDto                  // レスポンス用
pub struct CreatePostRequest        // 作成用
pub struct UpdatePostRequest        // 更新用
pub struct PostListDto              // 一覧用
pub struct PublishPostRequest       // 公開用
```

##### Comment DTOs
```rust
// src/application/dto/comment.rs
pub struct CommentDto               // レスポンス用
pub struct CreateCommentRequest     // 作成用
pub struct CommentListDto           // 一覧用
```

##### 共通 DTOs
```rust
// src/application/dto/common.rs
pub struct PaginationRequest        // ページネーション
pub struct PaginationResponse<T>    // ページネーション結果
pub struct ErrorResponse            // エラーレスポンス
```

#### 2. User Use Cases（4個）

##### RegisterUserUseCase
```rust
// src/application/use_cases/user/register.rs
pub struct RegisterUserUseCase {
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl RegisterUserUseCase {
    pub async fn execute(
        &self,
        request: CreateUserRequest,
    ) -> ApplicationResult<UserDto>
}
```

**責務**:
- Email/Username 重複チェック
- パスワードハッシュ化
- User Entity 作成
- Repository 保存
- UserRegistered イベント発行

##### GetUserByIdUseCase
```rust
// src/application/use_cases/user/get_by_id.rs
pub struct GetUserByIdUseCase {
    user_repo: Arc<dyn UserRepository>,
}

impl GetUserByIdUseCase {
    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> ApplicationResult<Option<UserDto>>
}
```

##### UpdateUserUseCase
```rust
// src/application/use_cases/user/update.rs
pub struct UpdateUserUseCase {
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl UpdateUserUseCase {
    pub async fn execute(
        &self,
        user_id: UserId,
        request: UpdateUserRequest,
    ) -> ApplicationResult<UserDto>
}
```

##### SuspendUserUseCase
```rust
// src/application/use_cases/user/suspend.rs
pub struct SuspendUserUseCase {
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl SuspendUserUseCase {
    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> ApplicationResult<()>
}
```

#### 3. Post Use Cases（4個）

##### CreatePostUseCase
```rust
// src/application/use_cases/post/create.rs
pub struct CreatePostUseCase {
    post_repo: Arc<dyn PostRepository>,
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl CreatePostUseCase {
    pub async fn execute(
        &self,
        author_id: UserId,
        request: CreatePostRequest,
    ) -> ApplicationResult<PostDto>
}
```

##### PublishPostUseCase
```rust
// src/application/use_cases/post/publish.rs
pub struct PublishPostUseCase {
    post_repo: Arc<dyn PostRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl PublishPostUseCase {
    pub async fn execute(
        &self,
        post_id: PostId,
    ) -> ApplicationResult<PostDto>
}
```

##### UpdatePostUseCase
```rust
// src/application/use_cases/post/update.rs
pub struct UpdatePostUseCase {
    post_repo: Arc<dyn PostRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl UpdatePostUseCase {
    pub async fn execute(
        &self,
        post_id: PostId,
        request: UpdatePostRequest,
    ) -> ApplicationResult<PostDto>
}
```

##### ListPostsUseCase
```rust
// src/application/use_cases/post/list.rs
pub struct ListPostsUseCase {
    post_repo: Arc<dyn PostRepository>,
}

impl ListPostsUseCase {
    pub async fn execute(
        &self,
        request: PaginationRequest,
    ) -> ApplicationResult<PaginationResponse<PostListDto>>
}
```

### 🧪 テスト戦略

#### 単体テスト（Use Case レベル）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;

    mock! {
        pub UserRepo {}
        #[async_trait]
        impl UserRepository for UserRepo {
            async fn save(&self, user: User) -> DomainResult<()>;
            async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>>;
            async fn find_by_email(&self, email: &Email) -> DomainResult<Option<User>>;
            async fn delete(&self, id: &UserId) -> DomainResult<()>;
            async fn exists_by_email(&self, email: &Email) -> DomainResult<bool>;
        }
    }

    #[tokio::test]
    async fn test_register_user_success() {
        // Arrange
        let mut mock_repo = MockUserRepo::new();
        mock_repo
            .expect_exists_by_email()
            .returning(|_| Ok(false));
        mock_repo
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        let use_case = RegisterUserUseCase::new(
            Arc::new(mock_repo),
            Arc::new(MockEventPublisher::new()),
            Arc::new(MockPasswordHasher::new()),
        );

        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "testuser");
        assert_eq!(user_dto.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_register_user_duplicate_email() {
        // Arrange
        let mut mock_repo = MockUserRepo::new();
        mock_repo
            .expect_exists_by_email()
            .returning(|_| Ok(true));

        let use_case = RegisterUserUseCase::new(
            Arc::new(mock_repo),
            Arc::new(MockEventPublisher::new()),
            Arc::new(MockPasswordHasher::new()),
        );

        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "duplicate@example.com".to_string(),
            password: "password123".to_string(),
        };

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::DuplicateEmail(_)
        ));
    }
}
```

### 📏 Week 8-9 完了基準

- [ ] 6個以上の DTO 実装完了
- [ ] 8個以上の Use Case 実装完了
- [ ] 各 Use Case に最低 2個の単体テスト
- [ ] すべてのテストがパス
- [ ] Clippy 警告ゼロ
- [ ] ドキュメントコメント完備

---

## 📅 Week 10-11: Repository 実装 + CQRS（2週間）

### 🎯 目標

- **Repository 実装**: 5個（User/Post/Comment/Tag/Category）
- **CQRS Queries**: 3-5個（読み取り専用クエリ）
- **Unit of Work**: 1個（トランザクション管理）
- **統合テスト**: 30個以上

### 📋 タスク詳細

#### 1. Repository 実装（5個）

##### DieselUserRepository
```rust
// src/infrastructure/database/repositories/diesel_user_repository.rs
pub struct DieselUserRepository {
    pool: DbPool,
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn save(&self, user: User) -> DomainResult<()> {
        let conn = self.pool.get().await?;
        let model = UserModel::from(user);
        
        diesel::insert_into(users::table)
            .values(&model)
            .on_conflict(users::id)
            .do_update()
            .set(&model)
            .execute(&conn)
            .await?;
        
        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>> {
        let conn = self.pool.get().await?;
        
        let result = users::table
            .filter(users::id.eq(id.as_uuid()))
            .first::<UserModel>(&conn)
            .await
            .optional()?;
        
        Ok(result.map(User::from))
    }

    async fn find_by_email(&self, email: &Email) -> DomainResult<Option<User>> {
        let conn = self.pool.get().await?;
        
        let result = users::table
            .filter(users::email.eq(email.as_str()))
            .first::<UserModel>(&conn)
            .await
            .optional()?;
        
        Ok(result.map(User::from))
    }

    async fn delete(&self, id: &UserId) -> DomainResult<()> {
        let conn = self.pool.get().await?;
        
        diesel::delete(users::table.filter(users::id.eq(id.as_uuid())))
            .execute(&conn)
            .await?;
        
        Ok(())
    }

    async fn exists_by_email(&self, email: &Email) -> DomainResult<bool> {
        let conn = self.pool.get().await?;
        
        let count: i64 = users::table
            .filter(users::email.eq(email.as_str()))
            .count()
            .get_result(&conn)
            .await?;
        
        Ok(count > 0)
    }
}
```

**実装方針**:
- Domain Entity ↔ DB Model 変換: `From` trait 実装
- エラーハンドリング: Diesel エラー → `InfrastructureError` → `DomainError`
- 接続プール: `deadpool-diesel` 使用
- トランザクション: Unit of Work パターン

##### DieselPostRepository
```rust
// src/infrastructure/database/repositories/diesel_post_repository.rs
pub struct DieselPostRepository {
    pool: DbPool,
}

#[async_trait]
impl PostRepository for DieselPostRepository {
    async fn save(&self, post: Post) -> DomainResult<()> { /* ... */ }
    async fn find_by_id(&self, id: &PostId) -> DomainResult<Option<Post>> { /* ... */ }
    async fn find_by_slug(&self, slug: &Slug) -> DomainResult<Option<Post>> { /* ... */ }
    async fn find_by_author(&self, author_id: &UserId) -> DomainResult<Vec<Post>> { /* ... */ }
    async fn delete(&self, id: &PostId) -> DomainResult<()> { /* ... */ }
    async fn list(
        &self,
        page: u32,
        per_page: u32,
    ) -> DomainResult<(Vec<Post>, u64)> { /* ... */ }
    async fn count(&self) -> DomainResult<u64> { /* ... */ }
}
```

##### その他 Repository（Comment/Tag/Category）

同様のパターンで実装。詳細は `RESTRUCTURE_EXAMPLES.md` を参照。

#### 2. CQRS Queries（3-5個）

##### ListUsersQuery
```rust
// src/application/queries/list_users.rs
pub struct ListUsersQuery {
    pool: DbPool,
}

impl ListUsersQuery {
    pub async fn execute(
        &self,
        page: u32,
        per_page: u32,
        filter: Option<UserFilter>,
    ) -> ApplicationResult<PaginationResponse<UserListDto>> {
        let conn = self.pool.get().await?;
        
        let mut query = users::table.into_boxed();
        
        if let Some(filter) = filter {
            if let Some(username) = filter.username {
                query = query.filter(users::username.like(format!("%{}%", username)));
            }
            if let Some(is_active) = filter.is_active {
                query = query.filter(users::is_active.eq(is_active));
            }
        }
        
        let total = query.count().get_result::<i64>(&conn).await? as u64;
        
        let users = query
            .limit(per_page as i64)
            .offset((page * per_page) as i64)
            .order(users::created_at.desc())
            .load::<UserModel>(&conn)
            .await?;
        
        let items = users.into_iter().map(UserListDto::from).collect();
        
        Ok(PaginationResponse {
            items,
            total,
            page,
            per_page,
        })
    }
}
```

##### ListPostsQuery
```rust
// src/application/queries/list_posts.rs
pub struct ListPostsQuery {
    pool: DbPool,
}

impl ListPostsQuery {
    pub async fn execute(
        &self,
        page: u32,
        per_page: u32,
        status: Option<PostStatus>,
        author_id: Option<UserId>,
    ) -> ApplicationResult<PaginationResponse<PostListDto>> { /* ... */ }
}
```

##### SearchPostsQuery
```rust
// src/application/queries/search_posts.rs
pub struct SearchPostsQuery {
    search_service: Arc<dyn SearchService>,
}

impl SearchPostsQuery {
    pub async fn execute(
        &self,
        keyword: String,
        limit: u32,
    ) -> ApplicationResult<Vec<PostListDto>> {
        let post_ids = self.search_service.search_posts(&keyword, limit).await?;
        
        // post_ids から PostListDto を取得
        // ...
        
        Ok(posts)
    }
}
```

#### 3. Unit of Work 実装

```rust
// src/infrastructure/database/unit_of_work.rs
pub struct DieselUnitOfWork {
    conn: PooledConnection<ConnectionManager<PgConnection>>,
}

impl DieselUnitOfWork {
    pub async fn new(pool: &DbPool) -> Result<Self, InfrastructureError> {
        let conn = pool.get().await?;
        Ok(Self { conn })
    }

    pub async fn begin(&mut self) -> Result<(), InfrastructureError> {
        self.conn.begin_test_transaction().await?;
        Ok(())
    }

    pub async fn commit(&mut self) -> Result<(), InfrastructureError> {
        // Diesel の transaction は自動コミット
        Ok(())
    }

    pub async fn rollback(&mut self) -> Result<(), InfrastructureError> {
        // Diesel の transaction は Drop で自動ロールバック
        Ok(())
    }

    pub async fn savepoint(&mut self, name: &str) -> Result<(), InfrastructureError> {
        diesel::sql_query(format!("SAVEPOINT {}", name))
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn release_savepoint(&mut self, name: &str) -> Result<(), InfrastructureError> {
        diesel::sql_query(format!("RELEASE SAVEPOINT {}", name))
            .execute(&self.conn)
            .await?;
        Ok(())
    }
}
```

### 🧪 統合テスト戦略

```rust
// tests/integration/user_use_cases.rs
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::clients::Cli;
    use testcontainers::images::postgres::Postgres;

    #[tokio::test]
    async fn test_register_and_get_user() {
        // Arrange: PostgreSQL コンテナ起動
        let docker = Cli::default();
        let postgres = docker.run(Postgres::default());
        let connection_string = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            postgres.get_host_port_ipv4(5432)
        );

        let pool = create_pool(&connection_string).await;
        run_migrations(&pool).await;

        let user_repo = Arc::new(DieselUserRepository::new(pool.clone()));
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let password_hasher = Arc::new(BcryptPasswordHasher::new());

        let register_use_case = RegisterUserUseCase::new(
            user_repo.clone(),
            event_publisher.clone(),
            password_hasher,
        );
        let get_use_case = GetUserByIdUseCase::new(user_repo);

        // Act: ユーザー登録
        let request = CreateUserRequest {
            username: "integration_test".to_string(),
            email: "integration@example.com".to_string(),
            password: "password123".to_string(),
        };
        let registered_user = register_use_case.execute(request).await.unwrap();

        // Act: ユーザー取得
        let user_id = UserId::from_str(&registered_user.id).unwrap();
        let fetched_user = get_use_case.execute(user_id).await.unwrap();

        // Assert
        assert!(fetched_user.is_some());
        let user = fetched_user.unwrap();
        assert_eq!(user.username, "integration_test");
        assert_eq!(user.email, "integration@example.com");
    }
}
```

### 📏 Week 10-11 完了基準

- [ ] 5個の Repository 実装完了
- [ ] 3個以上の CQRS Query 実装完了
- [ ] Unit of Work 実装完了
- [ ] 統合テスト 30個以上実装
- [ ] すべてのテストがパス（370個以上）
- [ ] Clippy 警告ゼロ
- [ ] パフォーマンステスト実施（Phase 2 との比較）

---

## 🔍 技術選定

### DTOs
- **Serialization**: `serde` + `serde_json`
- **Validation**: `validator` crate
- **Date/Time**: `chrono`

### Use Cases
- **DI Container**: 手動 DI（`Arc<dyn Trait>`）
- **Async**: `tokio` + `async-trait`
- **Transaction**: Unit of Work パターン

### Repository
- **ORM**: `diesel` + `diesel-async`
- **Connection Pool**: `deadpool-diesel`
- **Migration**: `diesel_migrations`

### Testing
- **Unit Test**: `mockall`
- **Integration Test**: `testcontainers`
- **Assertions**: `assert_matches`, `pretty_assertions`

---

## 📊 進捗管理

### Daily Standups（毎朝10分）

- 昨日やったこと
- 今日やること
- ブロッカー

### Weekly Reviews（毎週金曜日）

- 完了タスク数 / 予定タスク数
- テストカバレッジ
- Clippy 警告数
- 次週の計画調整

### リスク管理

| リスク | 発生確率 | 影響度 | 対策 |
|--------|---------|--------|------|
| Diesel async 対応遅延 | 中 | 高 | 事前検証・サンプル実装 |
| テスト環境構築遅延 | 低 | 中 | testcontainers 事前確認 |
| Use Case 設計変更 | 低 | 中 | 週次レビューで早期検出 |

---

## 📚 参考リソース

### 必読ドキュメント

- `RESTRUCTURE_PLAN.md` - 全体再編計画
- `RESTRUCTURE_EXAMPLES.md` - 実装例集
- `PHASE2_COMPLETION_REPORT.md` - Phase 2 完了報告
- `.github/copilot-instructions.md` - AI 開発者向け指示

### 外部リソース

- [Diesel Documentation](https://diesel.rs/)
- [async-trait Documentation](https://docs.rs/async-trait/)
- [mockall Documentation](https://docs.rs/mockall/)
- [testcontainers-rs Documentation](https://docs.rs/testcontainers/)

---

**作成日**: 2025年10月18日  
**次回レビュー**: Phase 3 Week 8 完了時（2025年11月1日予定）
