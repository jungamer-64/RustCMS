# Phase 3 Week 11 完了報告 - CQRS & Unit of Work

**作成日**: 2025年10月18日  
**ステータス**: ✅ 66% 完了（CQRS ✅ + Unit of Work ✅）  
**次のステップ**: 統合テスト（testcontainers）

---

## 📊 Executive Summary

Phase 3 Week 11 では、**CQRS パターン（Command Query Responsibility Segregation）**と**Unit of Work パターン**の実装を完了しました。これにより、読み取り操作と書き込み操作の分離、およびトランザクション境界の適切な管理が可能になりました。

### 主要成果

| カテゴリ | 実装内容 | コード行数 | テスト数 | ステータス |
|---------|---------|-----------|---------|-----------|
| **CQRS Queries** | Pagination + User/Post Queries | 978行 | 20 tests | ✅ 100% |
| **Unit of Work** | Transaction Management | 327行 | 5 tests | ✅ 100% |
| **統合テスト** | testcontainers + Repository Tests | - | - | 🔜 0% |
| **合計** | - | **1,305行** | **25 tests** | **66%** |

### 超過達成項目

- ✅ CQRS Queries: 3個実装（ListUsersQuery, ListPostsQuery, SearchPostsQuery）
- ✅ Unit of Work: 5個のメソッド（execute_in_transaction, with_savepoint, execute_two/three）
- ✅ Pagination Infrastructure: 完全な Builder パターン実装

---

## 🎯 Phase 3 Week 11 目標

### 当初の目標

1. **CQRS 実装** - 読み取り専用クエリの分離
2. **Unit of Work 実装** - トランザクション管理
3. **統合テスト** - testcontainers で PostgreSQL 起動

### 達成状況

- ✅ **CQRS 実装**: 100% 完了
- ✅ **Unit of Work 実装**: 100% 完了
- 🔜 **統合テスト**: 次のタスク（Phase 3 完了のための最終ステップ）

---

## 📝 実装詳細

### 1. CQRS Queries 実装（978行, 20 tests）

#### 1.1 Pagination Infrastructure (267行, 12 tests)

**ファイル**: `src/application/queries/pagination.rs`

**目的**: 全クエリで共通利用可能なページネーション機能を提供

**主要コンポーネント**:

```rust
// ページネーションパラメータ（1-100のlimit制限）
pub struct PaginationParams {
    limit: i64,   // 1-100にクランプ
    offset: i64,  // 負の値を防止
}

// ページネーション結果（メタデータ付き）
pub struct PaginationResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
```

**Builder パターン API**:

```rust
// 最初のページ
PaginationParams::first_page(20);

// ページ番号指定
PaginationParams::page(page_number, page_size);

// 次/前のページ
params.next_page();
params.prev_page();
```

**テストカバレッジ**: 12個
- Limit clamping（上限・下限）
- Offset validation（負の値防止）
- Page calculation（ページ番号 → offset 変換）
- Navigation（next/prev page）
- Result mapping（has_next_page, has_prev_page）

#### 1.2 User Queries (277行, 4 tests)

**ファイル**: `src/application/queries/user_queries.rs`

**目的**: ユーザー一覧取得と検索機能

**主要コンポーネント**:

```rust
pub struct ListUsersQuery {
    user_repo: Arc<dyn UserRepository>,
}

pub struct UserFilter {
    pub is_active: Option<bool>,           // アクティブ状態フィルタ
    pub username_contains: Option<String>, // ユーザー名検索
    pub email_contains: Option<String>,    // メール検索
}

pub struct UserSort {
    pub field: UserSortField,              // ソートフィールド
    pub direction: SortDirection,          // 昇順/降順
}

pub enum UserSortField {
    CreatedAt,
    UpdatedAt,
    Username,
    Email,
}
```

**使用例**:

```rust
// アクティブユーザーのみ、作成日降順
let filter = UserFilter::active_only();
let sort = UserSort::default(); // CreatedAt DESC
let pagination = PaginationParams::first_page(20);

let result = list_users_query.execute(filter, Some(sort), pagination).await?;
```

**Phase 3 実装方針**:
- In-memory filtering（簡素化）
- Phase 4 で SQL WHERE clauses に最適化予定

**テストカバレッジ**: 4個
- Filter builders（all/active_only）
- Sort defaults
- Pagination integration

#### 1.3 Post Queries (434行, 4 tests)

**ファイル**: `src/application/queries/post_queries.rs`

**目的**: 投稿一覧取得と全文検索

**主要コンポーネント**:

```rust
pub struct ListPostsQuery {
    post_repo: Arc<dyn PostRepository>,
}

pub struct PostFilter {
    pub status: Option<PostStatus>,              // 公開/下書き
    pub author_id: Option<UserId>,               // 著者フィルタ
    pub created_after: Option<DateTime<Utc>>,    // 作成日範囲
    pub created_before: Option<DateTime<Utc>>,
    pub published_after: Option<DateTime<Utc>>,  // 公開日範囲
    pub published_before: Option<DateTime<Utc>>,
    pub slug_contains: Option<String>,           // スラッグ検索
}

pub struct SearchPostsQuery {
    post_repo: Arc<dyn PostRepository>,
}
```

**Builder パターン**:

```rust
// 公開済み投稿のみ、特定著者、日付範囲指定
let filter = PostFilter::published_only()
    .with_author(author_id)
    .created_between(start_date, end_date)
    .with_slug("rust");
```

**全文検索 (SearchPostsQuery)**:

```rust
// Phase 3: Simple substring matching
let results = search_posts_query.search("Rust programming", pagination).await?;

// Phase 4: Tantivy integration（予定）
// - Full-text indexing
// - Ranking algorithm
// - Fuzzy matching
```

**テストカバレッジ**: 4個
- Filter builders（published_only/drafts_only）
- Sort defaults
- Search placeholder functionality

---

### 2. Unit of Work パターン実装（327行, 5 tests）

#### 2.1 DieselUnitOfWork

**ファイル**: `src/infrastructure/database/unit_of_work.rs`

**目的**: トランザクション境界の管理と自動コミット/ロールバック

**設計原則**:

1. **クロージャベース API** - 自動リソースクリーンアップ
2. **Async Wrapping** - Diesel 同期 API を `tokio::task::spawn_blocking` でラップ
3. **Error Propagation** - `RepositoryError` への自動変換

**主要メソッド**:

```rust
pub struct DieselUnitOfWork {
    pool: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl DieselUnitOfWork {
    // 単一トランザクション
    pub async fn execute_in_transaction<F, R>(
        &self,
        f: F,
    ) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut PgConnection) -> Result<R, RepositoryError> + Send + 'static,
        R: Send + 'static,
    {
        // spawn_blocking で Diesel 同期 API をラップ
        // 成功時: 自動コミット
        // 失敗時: 自動ロールバック
    }

    // ネストトランザクション（セーブポイント）
    pub fn with_savepoint<F, R>(
        conn: &mut PgConnection,
        f: F,
    ) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut PgConnection) -> Result<R, RepositoryError>,
    {
        // Diesel の build_transaction() API を使用
    }

    // 複数操作の同時実行
    pub async fn execute_two_in_transaction<F1, F2, R1, R2>(
        &self,
        f1: F1,
        f2: F2,
    ) -> Result<(R1, R2), RepositoryError>
    {
        // 2つの操作を単一トランザクション内で実行
    }

    pub async fn execute_three_in_transaction<F1, F2, F3, R1, R2, R3>(
        &self,
        f1: F1,
        f2: F2,
        f3: F3,
    ) -> Result<(R1, R2, R3), RepositoryError>
    {
        // 3つの操作を単一トランザクション内で実行
    }
}
```

**使用例**:

```rust
// Use Case での使用例
pub struct PublishPostUseCase {
    uow: Arc<DieselUnitOfWork>,
    post_repo: Arc<dyn PostRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl PublishPostUseCase {
    pub async fn execute(&self, post_id: PostId) -> Result<(), RepositoryError> {
        self.uow.execute_in_transaction(|conn| {
            // 1. 投稿を取得
            let mut post = self.post_repo.find_by_id_with_connection(conn, post_id)?
                .ok_or(RepositoryError::NotFound("Post not found".to_string()))?;

            // 2. 投稿を公開
            post.publish()?;

            // 3. 著者の統計を更新
            let author = self.user_repo.find_by_id_with_connection(conn, post.author_id())?
                .ok_or(RepositoryError::NotFound("Author not found".to_string()))?;

            // 4. 保存（両方失敗したら自動ロールバック）
            self.post_repo.save_with_connection(conn, post)?;
            self.user_repo.save_with_connection(conn, author)?;

            Ok(())
        }).await
    }
}
```

**テストカバレッジ**: 5個
- Creation test
- Commit success
- Rollback on error
- Two operations in transaction
- Three operations in transaction

#### 2.2 RepositoryError 拡張

**ファイル**: `src/application/ports/repositories.rs`

**追加実装**:

```rust
// Diesel Error からの自動変換
#[cfg(feature = "database")]
impl From<diesel::result::Error> for RepositoryError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            DieselError::NotFound => RepositoryError::NotFound("Record not found".to_string()),
            DieselError::DatabaseError(kind, info) => {
                RepositoryError::DatabaseError(format!("{kind:?}: {info}"))
            }
            DieselError::QueryBuilderError(msg) => {
                RepositoryError::DatabaseError(format!("Query builder error: {msg}"))
            }
            DieselError::DeserializationError(e) => {
                RepositoryError::ConversionError(format!("Deserialization error: {e}"))
            }
            DieselError::SerializationError(e) => {
                RepositoryError::ConversionError(format!("Serialization error: {e}"))
            }
            _ => RepositoryError::Unknown(format!("Diesel error: {err}")),
        }
    }
}

// Connection Pool Error からの変換
#[cfg(feature = "database")]
impl From<diesel::r2d2::PoolError> for RepositoryError {
    fn from(err: diesel::r2d2::PoolError) -> Self {
        RepositoryError::DatabaseError(format!("Connection pool error: {err}"))
    }
}
```

**重要な変更**:
- `RepositoryError` から `PartialEq` を削除（Diesel Error との互換性のため）
- Unit of Work 内でのエラー変換が自動化

---

## 🏗️ アーキテクチャパターン確立

### 1. CQRS パターン（Command Query Responsibility Segregation）

**原則**:
- **Commands**: 状態を変更する操作（Write）
- **Queries**: 状態を読み取る操作（Read）

**実装方針**:

```text
┌─────────────────────────────────────────────────┐
│          Application Layer (CQRS)               │
├─────────────────────────────────────────────────┤
│                                                 │
│  Commands (Write)          Queries (Read)      │
│  ├─ RegisterUser           ├─ ListUsersQuery   │
│  ├─ CreatePost             ├─ ListPostsQuery   │
│  ├─ PublishPost            └─ SearchPostsQuery │
│  └─ CreateComment                               │
│                                                 │
└─────────────────────────────────────────────────┘
         ↓                           ↓
┌─────────────────────┐   ┌─────────────────────┐
│   Repository        │   │   Repository        │
│   (Write Model)     │   │   (Read Model)      │
└─────────────────────┘   └─────────────────────┘
```

**Phase 3 実装**:
- Queries は In-memory filtering（簡素化）
- Commands は Repository Port 経由でDB更新

**Phase 4 最適化予定**:
- Queries: SQL WHERE clauses（パフォーマンス向上）
- SearchPostsQuery: Tantivy 統合（全文検索エンジン）

### 2. Unit of Work パターン

**原則**:
- トランザクション境界の明示化
- 複数の Repository 操作を単一のトランザクションでラップ
- 自動的なコミット/ロールバック

**実装戦略**:

```text
┌─────────────────────────────────────────────────┐
│              Use Case Layer                     │
│                                                 │
│  pub async fn execute(&self) {                  │
│      self.uow.execute_in_transaction(|conn| {   │
│          // 複数のRepository操作                 │
│          user_repo.save(conn, user)?;           │
│          post_repo.save(conn, post)?;           │
│          Ok(())  // 成功 → コミット              │
│      }).await?;  // 失敗 → ロールバック          │
│  }                                              │
└─────────────────────────────────────────────────┘
         ↓
┌─────────────────────────────────────────────────┐
│         DieselUnitOfWork (Infrastructure)       │
│                                                 │
│  tokio::task::spawn_blocking(move || {          │
│      conn.transaction(|conn| f(conn))  ← Diesel │
│  }).await                                       │
└─────────────────────────────────────────────────┘
```

**Async Wrapping パターン**:
```rust
// Diesel (同期 API)
conn.transaction(|conn| { /* ... */ })

// ↓ spawn_blocking でラップ

// Unit of Work (非同期 API)
uow.execute_in_transaction(|conn| { /* ... */ }).await
```

### 3. Error Chain パターン

**エラー変換の自動化**:

```text
Diesel Error (diesel::result::Error)
    ↓ From<diesel::result::Error>
RepositoryError (application/ports)
    ↓ From<RepositoryError>
ApplicationError (application layer)
    ↓ From<ApplicationError>
AppError (HTTP layer)
```

**実装例**:

```rust
// Use Case 内
let user = user_repo.find_by_id(id).await?;
//                                      ↑
// RepositoryError → ApplicationError に自動変換（From trait）

// Handler 内
let result = use_case.execute(request).await?;
//                                           ↑
// ApplicationError → AppError → IntoResponse に自動変換
```

---

## 📊 テスト結果

### テスト統計

| カテゴリ | テスト数 | 成功 | 失敗 | 無視 |
|---------|---------|------|------|------|
| **Domain Layer** | 133 | 133 | 0 | 0 |
| **Application Layer** | 110 | 110 | 0 | 0 |
| **Infrastructure Layer** | 19 | 14 | 0 | 5 |
| **合計** | **262** | **257** | **0** | **5** |

**注意**: Infrastructure Layer の 5個のテストは実際の DB 接続が必要なため `#[ignore]` 属性付き

### 新規追加テスト（Phase 3 Week 11）

#### Pagination Tests (12個)

```rust
#[test] fn test_pagination_params_new()
#[test] fn test_pagination_params_clamp_limit_max()
#[test] fn test_pagination_params_clamp_limit_min()
#[test] fn test_pagination_params_clamp_offset()
#[test] fn test_pagination_params_first_page()
#[test] fn test_pagination_params_page()
#[test] fn test_pagination_params_next_page()
#[test] fn test_pagination_params_prev_page()
#[test] fn test_pagination_result_new()
#[test] fn test_pagination_result_has_next_page()
#[test] fn test_pagination_result_has_prev_page()
#[test] fn test_pagination_result_map()
```

#### User Queries Tests (4個)

```rust
#[test] fn test_user_filter_all()
#[test] fn test_user_filter_active_only()
#[test] fn test_user_sort_default()
#[test] fn test_user_filter_builder()
```

#### Post Queries Tests (4個)

```rust
#[test] fn test_post_filter_published_only()
#[test] fn test_post_filter_drafts_only()
#[test] fn test_post_sort_default()
#[test] fn test_post_filter_builder()
```

#### Unit of Work Tests (5個)

```rust
#[test] fn test_unit_of_work_creation()
#[tokio::test] #[ignore] async fn test_execute_in_transaction_commit()
#[tokio::test] #[ignore] async fn test_execute_in_transaction_rollback()
#[tokio::test] #[ignore] async fn test_execute_two_in_transaction()
#[tokio::test] #[ignore] async fn test_execute_three_in_transaction()
```

---

## 📁 ファイル構成

### 新規作成ファイル

```text
src/application/queries/
├── mod.rs                     (38行)  - Query module structure
├── pagination.rs              (267行) - Pagination infrastructure
├── user_queries.rs            (277行) - User queries (List/Filter)
└── post_queries.rs            (434行) - Post queries (List/Search)

src/infrastructure/database/
└── unit_of_work.rs            (327行) - Transaction management
```

### 変更ファイル

```text
src/application/
├── mod.rs                     - queries module 追加
└── ports/repositories.rs      - RepositoryError 拡張

src/infrastructure/
├── mod.rs                     - database module feature flag 修正
└── database/mod.rs            - schema 再エクスポート
```

---

## 🔧 技術的課題と解決策

### 課題 1: PartialEq と Diesel Error の互換性

**問題**:
```rust
// RepositoryError が PartialEq を derive
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RepositoryError { /* ... */ }

// しかし Diesel Error は PartialEq を実装していない
impl From<diesel::result::Error> for RepositoryError {
    // ❌ コンパイルエラー
}
```

**解決策**:
```rust
// PartialEq を削除
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError { /* ... */ }

// テストで assert_eq! を使用している箇所を修正
// Before:
assert_eq!(result, Ok(0));

// After:
assert!(result.is_ok());
assert_eq!(result.unwrap(), 0);
```

### 課題 2: Module Database の重複定義

**問題**:
```rust
// src/infrastructure/mod.rs

// 新構造（restructure_domain）
#[cfg(all(feature = "restructure_domain", feature = "database"))]
pub mod database;

// 旧構造（レガシー）
#[cfg(all(not(feature = "restructure_application"), feature = "database"))]
pub mod database {
    pub use crate::database::*;
}

// ❌ Error: the name `database` is defined multiple times
```

**解決策**:
```rust
// feature flag を統一
#[cfg(all(feature = "restructure_domain", feature = "database"))]
pub mod database;

#[cfg(all(not(feature = "restructure_domain"), feature = "database"))]
pub mod database {
    pub use crate::database::*;
}
```

### 課題 3: Schema モジュールが見つからない

**問題**:
```rust
// src/infrastructure/database/repositories/comment_repository.rs
use crate::infrastructure::database::schema::comments;
//                                   ^^^^^^ could not find `schema` in `database`
```

**解決策**:
```rust
// src/infrastructure/database/mod.rs

// レガシー database モジュールの schema を再エクスポート
#[cfg(feature = "restructure_domain")]
pub use crate::database::schema;
```

---

## 🚀 次のステップ（Phase 3 Week 11 残り 34%）

### 優先度: High

#### 1. 統合テスト実装（testcontainers）

**目標**: Repository 実装の実際の PostgreSQL での動作確認

**タスク**:

```rust
// tests/integration/repositories/mod.rs

use testcontainers::clients::Cli;
use testcontainers::images::postgres::Postgres;

#[tokio::test]
async fn test_user_repository_crud() {
    // 1. PostgreSQL コンテナ起動
    let docker = Cli::default();
    let postgres = docker.run(Postgres::default());
    let connection_string = format!(
        "postgres://postgres@127.0.0.1:{}/postgres",
        postgres.get_host_port_ipv4(5432)
    );

    // 2. マイグレーション実行
    // 3. Repository 初期化
    // 4. CRUD テスト
    // 5. コンテナ自動削除
}
```

**実装内容**:
- ✅ Repository trait 準拠テスト（全メソッド実行確認）
- ✅ トランザクションロールバックテスト
- ✅ 並行アクセステスト（connection pool）
- ✅ エラーハンドリングテスト

**予想工数**: 2-3日

#### 2. Use Case での Unit of Work 使用例追加

**目標**: Unit of Work の実際の使用方法をドキュメント化

**タスク**:

```rust
// src/application/use_cases/examples_unit_of_work.rs

/// 複数の Repository 操作を単一トランザクションで実行する例
pub struct PublishPostAndUpdateStatsUseCase {
    uow: Arc<DieselUnitOfWork>,
    post_repo: Arc<dyn PostRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl PublishPostAndUpdateStatsUseCase {
    pub async fn execute(&self, post_id: PostId) -> Result<(), RepositoryError> {
        self.uow.execute_in_transaction(|conn| {
            // 投稿を公開
            let mut post = /* ... */;
            post.publish()?;
            
            // 著者の統計を更新
            let mut author = /* ... */;
            author.increment_post_count();
            
            // 両方保存（どちらか失敗したらロールバック）
            self.post_repo.save_with_connection(conn, post)?;
            self.user_repo.save_with_connection(conn, author)?;
            
            Ok(())
        }).await
    }
}
```

**予想工数**: 1日

---

## 📈 Phase 3 全体進捗

### Week 8-9: DTO + Use Cases ✅ (100%)

- ✅ 4個の DTO Modules（640行, 16 tests）
- ✅ 10個の Use Cases（~2,500行, 43 tests）
- ✅ Application Layer: 90/90 tests passing

### Week 10: Repository 実装 ✅ (100%)

- ✅ 3個の Repository 実装（1,084行, 14 tests）
- ✅ Domain Entity 拡張（Post/Comment restore()）
- ✅ Infrastructure Layer: 14/14 tests passing

### Week 11: CQRS + Unit of Work ✅ (66%)

- ✅ CQRS Queries（978行, 20 tests）
- ✅ Unit of Work（327行, 5 tests）
- 🔜 統合テスト（testcontainers）

### Phase 3 合計進捗: 88% 完了

| Week | タスク | 進捗 |
|------|-------|------|
| Week 8-9 | DTO + Use Cases | ✅ 100% |
| Week 10 | Repository 実装 | ✅ 100% |
| Week 11 | CQRS + Unit of Work | 🚀 66% |
| **合計** | **Phase 3 全体** | **88%** |

---

## 📚 参考資料

### 実装したパターン

1. **CQRS Pattern**
   - Martin Fowler: [CQRS](https://martinfowler.com/bliki/CQRS.html)
   - Microsoft: [CQRS Pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs)

2. **Unit of Work Pattern**
   - Martin Fowler: [Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html)
   - Diesel Documentation: [Transactions](https://docs.diesel.rs/2.2.x/diesel/connection/trait.Connection.html#method.transaction)

3. **Repository Pattern**
   - Martin Fowler: [Repository](https://martinfowler.com/eaaCatalog/repository.html)
   - DDD: Eric Evans - Domain-Driven Design

### 関連ドキュメント

- `RESTRUCTURE_PLAN.md` - 構造再編計画
- `PHASE3_WEEK10_COMPLETION_REPORT.md` - Week 10 完了報告
- `.github/copilot-instructions.md` - AI 開発者向け指示

---

## ✅ 完了条件チェックリスト

- [x] CQRS Queries 実装完了（3個）
- [x] Pagination Infrastructure 実装完了
- [x] Unit of Work パターン実装完了
- [x] RepositoryError 拡張（Diesel Error 変換）
- [x] 全テストパス（257/262, 5 ignored）
- [x] ドキュメント作成
- [ ] 統合テスト実装（次のタスク）
- [ ] Phase 3 Week 11 完全完了（100%）

---

**次回アクション**: 統合テスト（testcontainers）の実装でPhase 3を完全完了させる

**担当者**: AI Development Team  
**レビュー日**: 2025年10月18日  
**承認**: ✅ Phase 3 Week 11 (66%) 完了
