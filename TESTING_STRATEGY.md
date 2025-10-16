# RustCMS テスト戦略

## 🎯 テスト目標

### カバレッジ目標

| レイヤー | カバレッジ目標 | 優先度 | 理由 |
|---------|--------------|-------|-----|
| Domain Layer | **100%** | 🔴 Critical | ビジネスロジックの完全性保証 |
| Application Layer | **95%** | 🔴 Critical | Use Case の正確性保証 |
| Infrastructure Layer | **80%** | 🟡 High | 外部依存の基本動作保証 |
| Presentation Layer | **90%** | 🟡 High | API 契約の遵守保証 |
| Overall | **≥85%** | 🟡 High | プロジェクト全体の品質担保 |

### パフォーマンス目標

- **ユニットテスト実行時間**: < 30秒 (全テストスイート)
- **統合テスト実行時間**: < 5分 (Docker コンテナ起動含む)
- **E2E テスト実行時間**: < 10分 (主要シナリオ)
- **並列実行**: 最大CPU数まで並列化

---

## 🏗️ レイヤー別テスト戦略

### 1. Domain Layer (ドメイン層)

#### 対象

- Value Objects (`src/domain/value_objects/`)
- Entities (`src/domain/entities/`)
- Domain Services (`src/domain/services/`)
- Domain Events (`src/domain/events/`)

#### テストアプローチ

**原則**: **100% ピュアユニットテスト** (外部依存なし)

1. **Value Objects**
   - ✅ 正常な値の生成とバリデーション
   - ✅ 不正な値の拒否とエラーメッセージ
   - ✅ Equality / Comparison の正確性
   - ✅ Serialization / Deserialization

2. **Entities**
   - ✅ Factory メソッドによる不変条件の保証
   - ✅ ビジネスメソッドの副作用とイベント発行
   - ✅ 無効な状態遷移の防止
   - ✅ Property-based Testing (proptest)

3. **Domain Services**
   - ✅ ビジネスルールの正確性
   - ✅ エンティティ間の整合性保証
   - ✅ エラーケースの網羅

#### ツール

- **mockall**: Repository Port のモック化
- **proptest**: ランダムテストによる網羅性向上
- **rstest**: パラメタライズドテスト

#### 実装例

```rust
// tests/domain/value_objects/email_test.rs
use proptest::prelude::*;

#[test]
fn test_valid_email() {
    let email = Email::new("user@example.com".to_string()).unwrap();
    assert_eq!(email.as_str(), "user@example.com");
}

#[test]
fn test_invalid_email_no_at_sign() {
    let result = Email::new("invalid".to_string());
    assert!(matches!(result, Err(DomainError::InvalidEmail(_))));
}

proptest! {
    #[test]
    fn test_email_roundtrip(s in "[a-z]{1,10}@[a-z]{3,10}\\.(com|org)") {
        let email = Email::new(s.clone()).unwrap();
        assert_eq!(email.as_str(), &s);
    }
}
```

---

### 2. Application Layer (アプリケーション層)

#### 対象

- Use Cases (`src/application/use_cases/`)
- Commands / Queries (`src/application/commands/`, `src/application/queries/`)
- DTOs (`src/application/dto/`)
- Application Services (`src/application/services/`)

#### テストアプローチ

**原則**: **モックベースのユニットテスト** (Repository/Service は mock)

1. **Use Cases**
   - ✅ 正常系: Repository への正しい呼び出し順序
   - ✅ 異常系: エラーハンドリングとロールバック
   - ✅ トランザクション境界の検証
   - ✅ イベント発行の検証

2. **Commands / Queries**
   - ✅ CQRS 分離の遵守
   - ✅ 読み取り専用操作の副作用なし
   - ✅ 書き込み操作のイベント発行

#### ツール

- **mockall**: Repository/Service Port のモック
- **tokio::test**: 非同期テストランタイム
- **insta**: スナップショットテスト (DTO 出力検証)

#### 実装例

```rust
// tests/application/use_cases/register_user_test.rs
use mockall::predicate::*;

#[tokio::test]
async fn test_register_user_success() {
    // Arrange
    let mut mock_repo = MockUserRepository::new();
    mock_repo
        .expect_find_by_email()
        .with(eq(Email::new("test@example.com".into()).unwrap()))
        .returning(|_| Ok(None));
    
    mock_repo
        .expect_create()
        .withf(|user| user.email().as_str() == "test@example.com")
        .returning(|user| Ok(user));

    let mut mock_event_bus = MockEventPublisher::new();
    mock_event_bus
        .expect_publish()
        .with(always())
        .returning(|_| Ok(()));

    let use_case = RegisterUserUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_event_bus),
    );

    // Act
    let request = RegisterUserRequest {
        email: "test@example.com".into(),
        username: "testuser".into(),
        password: "SecurePassword123!".into(),
    };
    let result = use_case.execute(request).await;

    // Assert
    assert!(result.is_ok());
    let user_dto = result.unwrap();
    assert_eq!(user_dto.email, "test@example.com");
}

#[tokio::test]
async fn test_register_user_duplicate_email() {
    let mut mock_repo = MockUserRepository::new();
    mock_repo
        .expect_find_by_email()
        .returning(|_| Ok(Some(User::new(/* ... */))));
    
    let use_case = RegisterUserUseCase::new(Arc::new(mock_repo), /* ... */);
    let result = use_case.execute(/* ... */).await;

    assert!(matches!(result, Err(ApplicationError::Conflict(_))));
}
```

---

### 3. Infrastructure Layer (インフラ層)

#### 対象

- Repository 実装 (`src/infrastructure/database/`)
- Cache 実装 (`src/infrastructure/cache/`)
- Search 実装 (`src/infrastructure/search/`)
- Event Bus 実装 (`src/infrastructure/events/`)

#### テストアプローチ

**原則**: **統合テスト** (実際の外部依存を使用)

1. **Database Repositories**
   - ✅ **testcontainers** で PostgreSQL 起動
   - ✅ マイグレーション適用
   - ✅ CRUD 操作の正確性
   - ✅ トランザクション境界の検証
   - ✅ ロールバックの動作確認

2. **Cache Services**
   - ✅ **testcontainers** で Redis 起動
   - ✅ キャッシュヒット/ミス
   - ✅ TTL の正確性
   - ✅ キャッシュ無効化

3. **Search Services**
   - ✅ Tantivy インメモリインデックス
   - ✅ 全文検索の正確性
   - ✅ インデックス更新の反映

#### ツール

- **testcontainers**: Docker コンテナ管理
- **diesel_migrations**: DB マイグレーション
- **serial_test**: テスト順序制御 (必要に応じて)

#### 実装例

```rust
// tests/infrastructure/repositories/diesel_user_repository_test.rs
use testcontainers::{clients::Cli, images::postgres::Postgres};
use diesel::connection::Connection;

#[tokio::test]
async fn test_create_and_find_user() {
    // Arrange: Start PostgreSQL container
    let docker = Cli::default();
    let postgres = docker.run(Postgres::default());
    let connection_string = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/test",
        postgres.get_host_port_ipv4(5432)
    );

    let pool = create_pool(&connection_string);
    run_migrations(&pool);

    let repo = DieselUserRepository::new(pool.clone());

    // Act: Create user
    let user = User::new(
        UserId::new_v4(),
        Email::new("test@example.com".into()).unwrap(),
        Username::new("testuser".into()).unwrap(),
    );
    let created = repo.create(user.clone()).await.unwrap();

    // Assert: Find user
    let found = repo.find_by_id(created.id()).await.unwrap().unwrap();
    assert_eq!(found.email(), user.email());
}

#[tokio::test]
async fn test_transaction_rollback() {
    let docker = Cli::default();
    let postgres = docker.run(Postgres::default());
    let pool = create_pool(&format!(
        "postgres://postgres:postgres@127.0.0.1:{}/test",
        postgres.get_host_port_ipv4(5432)
    ));

    let repo = DieselUserRepository::new(pool.clone());
    let uow = DieselUnitOfWork::new(pool.clone());

    // Start transaction
    let tx = uow.begin().await.unwrap();

    // Create user in transaction
    let user = User::new(/* ... */);
    repo.create_in_transaction(&tx, user.clone()).await.unwrap();

    // Rollback
    tx.rollback().await.unwrap();

    // Assert: User should not exist
    let found = repo.find_by_id(user.id()).await.unwrap();
    assert!(found.is_none());
}
```

---

### 4. Presentation Layer (プレゼンテーション層)

#### 対象

- HTTP Handlers (`src/handlers/`)
- Middleware (`src/middleware/`)
- OpenAPI Schema (`src/openapi.rs`)

#### テストアプローチ

**原則**: **エンドツーエンドテスト** (実際の HTTP リクエスト)

1. **Handlers**
   - ✅ 正常系: 正しいステータスコードとレスポンス
   - ✅ 異常系: エラーハンドリングと適切なステータスコード
   - ✅ 認証/認可の検証
   - ✅ バリデーションエラーのテスト

2. **Middleware**
   - ✅ 認証トークンの検証
   - ✅ レート制限の動作
   - ✅ CORS ヘッダーの付与

#### ツール

- **axum-test-helpers**: Axum アプリケーションのテスト
- **reqwest**: HTTP クライアント
- **serde_json**: JSON アサーション

#### 実装例

```rust
// tests/handlers/users_test.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_register_user_endpoint() {
    // Arrange
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/v2/users")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"email":"test@example.com","username":"testuser","password":"SecurePass123!"}"#
        ))
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let user: UserDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(user.email, "test@example.com");
}

#[tokio::test]
async fn test_register_user_invalid_email() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/v2/users")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"email":"invalid","username":"testuser","password":"SecurePass123!"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_user_unauthorized() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/v2/users/123e4567-e89b-12d3-a456-426614174000")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

---

## 🔬 特殊テスト

### Property-Based Testing (proptest)

**対象**: Value Objects, エンティティの不変条件

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_username_length_invariant(s in "[a-z]{3,20}") {
        let username = Username::new(s.clone());
        prop_assert!(username.is_ok());
        prop_assert_eq!(username.unwrap().as_str().len(), s.len());
    }

    #[test]
    fn test_username_rejects_short(s in "[a-z]{1,2}") {
        let username = Username::new(s);
        prop_assert!(username.is_err());
    }
}
```

### Snapshot Testing (insta)

**対象**: DTO, OpenAPI スキーマ, エラーメッセージ

```rust
use insta::assert_json_snapshot;

#[test]
fn test_user_dto_serialization() {
    let dto = UserDto {
        id: "123e4567-e89b-12d3-a456-426614174000".parse().unwrap(),
        email: "test@example.com".into(),
        username: "testuser".into(),
        created_at: "2025-01-01T00:00:00Z".parse().unwrap(),
    };

    assert_json_snapshot!(dto);
}
```

### Mutation Testing (cargo-mutants)

**頻度**: CI で週1回実行

```bash
cargo mutants --workspace --timeout 60
```

**目標**: Mutation Score ≥ 80%

---

## 🚀 CI/CD テスト実行

### Pull Request (PR) ビルド

```yaml
# .github/workflows/test.yml
- name: Run unit tests
  run: cargo test --workspace --lib --no-fail-fast

- name: Run integration tests
  run: cargo test --workspace --test '*' --no-fail-fast
  env:
    DATABASE_URL: postgres://postgres:password@localhost:5432/test

- name: Run E2E tests
  run: cargo test --workspace --test e2e --no-fail-fast

- name: Generate coverage
  run: cargo tarpaulin --workspace --out Xml --output-dir coverage/

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
```

### Nightly ビルド

```yaml
# .github/workflows/nightly.yml
- name: Run mutation tests
  run: cargo mutants --workspace --timeout 60

- name: Run property-based tests (extended)
  run: PROPTEST_CASES=10000 cargo test --workspace

- name: Run benchmarks
  run: cargo bench --workspace
```

---

## 📊 テストカバレッジ測定

### ツール

- **cargo-tarpaulin**: カバレッジ測定
- **cargo-llvm-cov**: LLVM ベースのカバレッジ
- **codecov.io**: カバレッジレポート可視化

### コマンド

```bash
# HTML レポート生成
cargo tarpaulin --out Html --output-dir coverage/

# カバレッジが目標を満たしているか確認
cargo tarpaulin --fail-under 85
```

---

## 🛠️ テストヘルパー

### Test Fixtures

```rust
// tests/common/fixtures.rs
pub fn create_test_user() -> User {
    User::new(
        UserId::new_v4(),
        Email::new("test@example.com".into()).unwrap(),
        Username::new("testuser".into()).unwrap(),
    )
}

pub fn create_test_post(author_id: UserId) -> Post {
    Post::new(
        PostId::new_v4(),
        Title::new("Test Post".into()).unwrap(),
        Content::new("Test content".into()).unwrap(),
        author_id,
    )
}
```

### Test Builders

```rust
// tests/common/builders.rs
pub struct UserBuilder {
    email: Option<String>,
    username: Option<String>,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self { email: None, username: None }
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn build(self) -> User {
        User::new(
            UserId::new_v4(),
            Email::new(self.email.unwrap_or("default@example.com".into())).unwrap(),
            Username::new(self.username.unwrap_or("defaultuser".into())).unwrap(),
        )
    }
}
```

---

## 📋 テスト実行チェックリスト

### 開発者ローカル (毎コミット前)

- [ ] `cargo test --workspace` - すべてのテストがパス
- [ ] `cargo clippy -- -D warnings` - Clippy 警告なし
- [ ] `cargo fmt --check` - フォーマット確認

### Pull Request (CI)

- [ ] ユニットテスト (lib) - すべてパス
- [ ] 統合テスト - すべてパス
- [ ] E2E テスト - すべてパス
- [ ] カバレッジ ≥ 85%
- [ ] パフォーマンステスト (劣化 < 5%)

### Weekly (Nightly CI)

- [ ] Mutation テスト (Mutation Score ≥ 80%)
- [ ] Property-based テスト (10,000 ケース)
- [ ] ベンチマーク (継続的改善確認)

---

**作成日**: 2025年10月16日  
**最終更新**: 2025年10月16日  
**ステータス**: Phase 1 準備完了
