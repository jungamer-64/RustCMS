# Phase 5: Repository 実装層（Infrastructure/Database）

**期間**: 2025-01-17 開始  
**目標**: Domain-Driven DDD パターンに準拠した Diesel リポジトリ実装  
**成功基準**: 95+ 統合テスト合格、0 脆弱性（Codacy trivy）、全 feature セットでビルド成功

---

## 📋 概要

### 背景
- **Phase 1-2**: Domain Layer 完成（5 entities × 109 tests = 2,963 LOC）
- **Phase 3-4**: Application Ports + Event System （合計 106 tests 追加）
- **Phase 5 今回**: Infrastructure/Database → 具体的な Diesel リポジトリ実装

### 現況
- ✅ Port/Trait 定義済み: `src/application/ports/repositories.rs` (342行, 21tests)
- ⚠️ リポジトリ実装: `src/infrastructure/repositories/` にスケルトン存在
  - `diesel_user_repository.rs`: 185行（部分実装）
  - `diesel_post_repository.rs`: 存在但し未確認
  - `diesel_tag_repository.rs`: 存在但し未確認
  - `diesel_comment_repository.rs`: 存在但し未確認
  - `diesel_category_repository.rs`: 存在但し未確認

---

## 🎯 Task Breakdown

### Task 1: 計画策定と現況調査 ✅ 完了

**完了内容**:
- [ ] 既存コード現況確認
- [ ] Domain Entity との対応マッピング
- [ ] ポート/アダプタパターン確認

**出力**:
- このドキュメント
- Todo リスト（7 tasks）

---

### Task 2: DieselUserRepository 完成度向上

**目的**: `diesel_user_repository.rs` (現在 185行) → Phase 2 User entity に完全準拠

**実装内容**:
1. **Port トレイト準拠**
   - `async fn save(&self, user: User) -> Result<(), RepositoryError>`
   - `async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>`
   - `async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>`
   - `async fn delete(&self, id: UserId) -> Result<(), RepositoryError>`
   - `async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepositoryError>`

2. **エラーハンドリング強化** (3-layer hierarchy)
   - DomainError → RepositoryError への正確なマッピング
   - Diesel error → InfrastructureError への変換
   - HTTP response への最終マッピング

3. **ビジネスルール適用**
   - Email の一意性チェック（重複時 `RepositoryError::Duplicate`）
   - User の不変条件（invariants）保護
   - Soft delete vs Hard delete の使い分け

4. **テスト実装** (16個以上)
   - save: 新規/更新/重複 Email
   - find_by_id: 存在/非存在
   - find_by_email: キャッシュ効果検証
   - delete: 存在/非存在
   - list_all: ページネーション、フィルター
   - Edge cases: 境界値、Unicode、並行処理

**成功基準**: 16/16 tests passing ✅

---

### Task 3: DieselPostRepository 実装

**目的**: Post エンティティ（708行, 19 tests in Phase 2）用リポジトリ

**実装内容**:
1. **Post Value Objects への対応**
   - PostId (Uuid)
   - Slug (String, unique constraint)
   - Published (bool)

2. **メソッド実装**
   ```rust
   async fn save(&self, post: Post) -> Result<(), RepositoryError>
   async fn find_by_id(&self, id: PostId) -> Result<Option<Post>, RepositoryError>
   async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, RepositoryError>
   async fn delete(&self, id: PostId) -> Result<(), RepositoryError>
   async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError>
   async fn find_by_author(&self, author_id: UserId) -> Result<Vec<Post>, RepositoryError>
   ```

3. **ビジネスロジック**
   - Slug の一意性チェック（Post publish 時に検証）
   - Draft ↔ Published 状態遷移の監視
   - Author 削除時の cascade ルール

4. **テスト** (18個以上)
   - Slug 一意性
   - Published 状態遷移
   - Author フィルター
   - ページネーション

**成功基準**: 18/18 tests passing ✅

---

### Task 4: DieselCommentRepository 実装

**目的**: Comment エンティティ（539行, 16 tests in Phase 2）用リポジトリ

**実装内容**:
1. **Comment の階層構造対応**
   - CommentId
   - PostId (外部キー)
   - UserId (author)
   - ParentCommentId (threading)

2. **メソッド実装**
   ```rust
   async fn save(&self, comment: Comment) -> Result<(), RepositoryError>
   async fn find_by_id(&self, id: CommentId) -> Result<Option<Comment>, RepositoryError>
   async fn find_by_post(&self, post_id: PostId, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError>
   async fn find_by_author(&self, author_id: UserId) -> Result<Vec<Comment>, RepositoryError>
   async fn delete(&self, id: CommentId) -> Result<(), RepositoryError>
   ```

3. **スレッド管理**
   - 親コメント削除時の子コメント処理（delete vs orphan）
   - 深さ制限（例: 3階層まで）

4. **テスト** (16個以上)
   - Threading 構造
   - Parent 削除時の挙動
   - Post フィルター

**成功基準**: 16/16 tests passing ✅

---

### Task 5: DieselTagRepository 実装

**目的**: Tag エンティティ（585行, 22 tests in Phase 2）用リポジトリ

**実装内容**:
1. **Tag Value Objects**
   - TagId (Uuid)
   - TagName (String, unique)
   - UsageCounter

2. **メソッド実装**
   ```rust
   async fn save(&self, tag: Tag) -> Result<(), RepositoryError>
   async fn find_by_id(&self, id: TagId) -> Result<Option<Tag>, RepositoryError>
   async fn find_by_name(&self, name: &TagName) -> Result<Option<Tag>, RepositoryError>
   async fn delete(&self, id: TagId) -> Result<(), RepositoryError>
   async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Tag>, RepositoryError>
   async fn list_in_use(&self) -> Result<Vec<Tag>, RepositoryError>  // usage_count > 0
   ```

3. **ビジネスルール**
   - Name の一意性チェック
   - Post association tracking via usage_counter
   - Delete cascade (posts_tags bridge table)

4. **テスト** (20個以上)
   - Name 一意性
   - Usage counter 更新
   - In-use フィルター

**成功基準**: 20/20 tests passing ✅

---

### Task 6: DieselCategoryRepository 実装

**目的**: Category エンティティ（651行, 31 tests in Phase 2）用リポジトリ

**実装内容**:
1. **Category Value Objects**
   - CategoryId (Uuid)
   - CategorySlug (String, unique)
   - PostCount (tracked counter)

2. **メソッド実装**
   ```rust
   async fn save(&self, category: Category) -> Result<(), RepositoryError>
   async fn find_by_id(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError>
   async fn find_by_slug(&self, slug: &CategorySlug) -> Result<Option<Category>, RepositoryError>
   async fn delete(&self, id: CategoryId) -> Result<(), RepositoryError>
   async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Category>, RepositoryError>
   async fn list_active(&self) -> Result<Vec<Category>, RepositoryError>  // post_count > 0
   ```

3. **ビジネスルール**
   - Slug の一意性チェック
   - Post association tracking via post_count
   - Active filter (post_count > 0)
   - Soft delete (deleted_at timestamp)

4. **テスト** (24個以上)
   - Slug 一意性
   - Post count tracking
   - Active フィルター
   - Soft delete

**成功基準**: 24/24 tests passing ✅

---

### Task 7: 統合テスト & Codacy 検査

**目的**: 全 5 リポジトリの連携動作確認と品質検証

**実装内容**:
1. **統合テスト**
   ```bash
   cargo test --lib infrastructure::repositories --quiet
   # 期待値: 95+ tests passing
   ```

2. **機能テスト**
   - Cross-repository 操作（例: User 削除 → Post cascade）
   - トランザクション整合性
   - 並行処理（multiple connections）

3. **Codacy 脆弱性スキャン**
   ```bash
   mcp_codacy_codacy_cli_analyze --rootPath /path/to/repo --file src/infrastructure/repositories/ --tool trivy
   # 期待値: 0 critical vulnerabilities
   ```

4. **Performance 検査**
   - Query performance (複雑な JOIN)
   - Connection pool 効率
   - Index effectiveness

**成功基準**: 
- 95+ tests passing ✅
- 0 critical CVE ✅
- Build success with all feature combinations ✅

---

## 🛠️ 実装パターン（参考）

### Error Handling Pattern
```rust
impl DieselUserRepository {
    async fn save(&self, user: User) -> Result<(), RepositoryError> {
        match self.db.upsert_user(user).await {
            Ok(_) => Ok(()),
            Err(DatabaseError::UniqueViolation(col)) if col == "email" => {
                Err(RepositoryError::Duplicate("Email already exists".into()))
            }
            Err(DatabaseError::ValidationError(e)) => {
                Err(RepositoryError::ValidationError(e))
            }
            Err(e) => Err(RepositoryError::DatabaseError(e.to_string())),
        }
    }
}
```

### Test Pattern
```rust
#[tokio::test]
async fn test_save_duplicate_email_returns_conflict() {
    let repo = setup_repository().await;
    
    // Setup: Create first user
    let user1 = User::new(UserId::new(), Email::new("test@example.com").unwrap(), ...);
    repo.save(user1).await.unwrap();
    
    // Execute: Attempt to save duplicate
    let user2 = User::new(UserId::new(), Email::new("test@example.com").unwrap(), ...);
    let result = repo.save(user2).await;
    
    // Assert
    assert!(matches!(result, Err(RepositoryError::Duplicate(_))));
}
```

---

## 📊 進捗指標

| Task | 説明 | 状態 | テスト | 行数 |
|------|------|------|--------|------|
| 1    | 計画策定 | ✅ | - | - |
| 2    | DieselUserRepository | ⏳ | 0/16 | 185→250 |
| 3    | DieselPostRepository | ⏳ | 0/18 | 0→300 |
| 4    | DieselCommentRepository | ⏳ | 0/16 | 0→280 |
| 5    | DieselTagRepository | ⏳ | 0/20 | 0→260 |
| 6    | DieselCategoryRepository | ⏳ | 0/24 | 0→280 |
| 7    | 統合テスト & Codacy | ⏳ | 0/95 | - |
| **合計** | **Phase 5 完成** | ⏳ | **0/189** | **~1,600** |

---

## 🚀 実装推奨順序

1. **DieselUserRepository** (既存部分最小化で最速)
2. **DieselPostRepository** (Post の複雑性中程度)
3. **DieselCommentRepository** (threading 対応)
4. **DieselTagRepository** (counter tracking)
5. **DieselCategoryRepository** (最も複雑)
6. **統合テスト + Codacy**

---

## ✅ チェックリスト（実装前）

- [ ] すべてのエンティティクラスを確認
- [ ] Port トレイト定義を確認 (`src/application/ports/repositories.rs`)
- [ ] 既存 Diesel schema を確認
- [ ] Error type hierarchy を確認 (`src/common/error_types.rs`)
- [ ] Feature flag (`restructure_application`, `database`) を理解

---

## 📝 参考ファイル

| ファイル | 用途 | LOC |
|---------|------|-----|
| `src/domain/entities/*.rs` | Domain Entity definitions | 2,963 |
| `src/application/ports/repositories.rs` | Repository Port/Traits | 342 |
| `src/common/error_types.rs` | 3-layer error hierarchy | 233 |
| `src/infrastructure/repositories/` | **この実装対象** | ~1,600 |
| `.github/workflows/ci.yml` | CI matrix (feature flags) | - |
| `Cargo.toml` | Dependencies | 497 |

---

**次ステップ**: 「Task 2: DieselUserRepository 完成度向上」に着手します。準備完了です！
