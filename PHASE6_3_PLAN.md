# Phase 6.3 実装計画 - Tag/Category Database Integration

**開始予定日**: 2025-10-18 (Phase 6.2b 完成後)  
**推定期間**: 5-7日  
**テスト目標**: 600+ tests passing

---

## 概要

Phase 6.3 では、Comment で確立したデータベース統合パターンを **Tag** と **Category** エンティティに適用します。

**Phase 6.2 から学習したパターン**:

```
Entity Definition (Domain)
    ↓
Database Schema (PostgreSQL)
    ↓
Diesel Schema Definition
    ↓
Database Helper Methods (6-8個) + Entity Reconstruction
    ↓
Repository Implementation (trait methods)
    ↓
Entity-to-Domain Mapping
```

---

## Step 1: データベーススキーマ定義

### 1.1 Tags テーブル

```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL UNIQUE,
    description TEXT NOT NULL,
    usage_count INT4 DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_tags_name ON tags(name);
CREATE INDEX idx_tags_usage_count ON tags(usage_count DESC);
```

**Diesel Schema**:

```rust
diesel::table! {
    tags (id) {
        id -> Uuid,
        name -> Varchar,
        description -> Text,
        usage_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
```

### 1.2 Categories テーブル

```sql
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    slug VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    parent_id UUID REFERENCES categories(id) ON DELETE SET NULL,
    post_count INT4 DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_categories_slug ON categories(slug);
CREATE INDEX idx_categories_parent_id ON categories(parent_id);
CREATE INDEX idx_categories_post_count ON categories(post_count DESC);
```

**Diesel Schema**:

```rust
diesel::table! {
    categories (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        post_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(categories -> categories (parent_id));
```

**アクション**: `src/database/schema.rs` にテーブル定義を追加

---

## Step 2: Tag CRUD Database Helpers

### 2.1 `create_tag(name, description)` 

- **機能**: タグを新規作成
- **検証**: name が既に存在しないか確認
- **戻り値**: (id, name, description, usage_count, created_at, updated_at)
- **エラー**: DuplicateTagName, ValidationError

### 2.2 `get_tag_by_id(tag_id)`

- **機能**: ID でタグを取得
- **戻り値**: 完全なタグデータタプル
- **エラー**: TagNotFound

### 2.3 `get_tag_by_name(name)`

- **機能**: name でタグを取得
- **戻り値**: 完全なタグデータタプル
- **エラー**: TagNotFound

### 2.4 `update_tag(tag_id, name, description, usage_count)`

- **機能**: タグメタデータを更新
- **検証**: name が既に存在しないか確認 (同じタグ除外)
- **戻り値**: 更新後のタグデータ
- **エラー**: TagNotFound, DuplicateTagName

### 2.5 `delete_tag(tag_id)`

- **機能**: タグを削除（物理削除）
- **戻り値**: success: boolean
- **エラー**: TagNotFound, TagInUse

### 2.6 `list_all_tags(page, limit)`

- **機能**: すべてのタグを取得
- **フィルタ**: なし
- **ソート**: `usage_count DESC` (人気順)
- **ページネーション**: ✅
- **戻り値**: Vec<Tuple>

### 2.7 `list_tags_in_use(page, limit)`

- **機能**: 1 回以上使用されているタグのみ
- **フィルタ**: `usage_count >= 1`
- **ソート**: `usage_count DESC`
- **ページネーション**: ✅
- **戻り値**: Vec<Tuple>

### 2.8 `increment_usage(tag_id, count)`

- **機能**: タグの使用数をインクリメント
- **使用場面**: ポストにタグを追加した時
- **戻り値**: 新しい usage_count
- **エラー**: TagNotFound

**ファイル**: `src/database/mod.rs` へ追加 (150-200行)

---

## Step 3: Tag Entity Reconstruction

### 3.1 `reconstruct_tag()` Helper

```rust
fn reconstruct_tag(
    id: Uuid,
    name: String,
    description: String,
    usage_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Tag
```

**実装**:

1. TagId(id) を作成
2. TagName(name) を検証・作成
3. Tag Value Object を作成
4. Tag Entity を構築

**ファイル**: `src/infrastructure/repositories/diesel_tag_repository.rs` に配置

---

## Step 4: Category CRUD Database Helpers

### 4.1 `create_category(name, slug, description, parent_id)`

- **機能**: カテゴリを新規作成
- **検証**: name, slug が既に存在しないか確認
- **親階層**: parent_id が存在するか確認
- **戻り値**: 完全なカテゴリデータタプル
- **エラー**: DuplicateCategoryName, DuplicateSlug, ParentNotFound

### 4.2 `get_category_by_id(category_id)`

- **機能**: ID でカテゴリを取得
- **戻り値**: 完全なカテゴリデータタプル
- **エラー**: CategoryNotFound

### 4.3 `get_category_by_slug(slug)`

- **機能**: slug でカテゴリを取得
- **戻り値**: 完全なカテゴリデータタプル
- **エラー**: CategoryNotFound

### 4.4 `update_category(category_id, name, slug, description, parent_id, post_count)`

- **機能**: カテゴリメタデータを更新
- **検証**: name, slug が既に存在しないか確認 (同じカテゴリ除外)
- **戻り値**: 更新後のカテゴリデータ
- **エラー**: CategoryNotFound, DuplicateName, DuplicateSlug, ParentNotFound

### 4.5 `delete_category(category_id)`

- **機能**: カテゴリを削除（子カテゴリの parent_id を NULL に）
- **戻り値**: success: boolean
- **エラー**: CategoryNotFound

### 4.6 `list_all_categories(page, limit)`

- **機能**: すべてのカテゴリを取得
- **フィルタ**: parent_id IS NULL (トップレベルのみ)
- **ソート**: `post_count DESC` (投稿数順)
- **ページネーション**: ✅
- **戻り値**: Vec<Tuple>

### 4.7 `list_categories_by_parent(parent_id, page, limit)`

- **機能**: 特定の親カテゴリの子を取得
- **フィルタ**: `parent_id = $1`
- **ソート**: `post_count DESC`
- **ページネーション**: ✅
- **戻り値**: Vec<Tuple>

### 4.8 `increment_post_count(category_id, count)`

- **機能**: カテゴリの投稿数をインクリメント
- **使用場面**: ポストにカテゴリを追加した時
- **戻り値**: 新しい post_count
- **エラー**: CategoryNotFound

**ファイル**: `src/database/mod.rs` へ追加 (200-250行)

---

## Step 5: Repository Implementations

### 5.1 Tag Repository

```rust
#[async_trait]
pub trait TagRepository {
    async fn save(&self, tag: &Tag) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: TagId) -> Result<Option<Tag>, RepositoryError>;
    async fn find_by_name(&self, name: &TagName) -> Result<Option<Tag>, RepositoryError>;
    async fn delete(&self, id: TagId) -> Result<(), RepositoryError>;
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Tag>, RepositoryError>;
    async fn list_in_use(&self, limit: i64, offset: i64) -> Result<Vec<Tag>, RepositoryError>;
}
```

**実装**: `src/infrastructure/repositories/diesel_tag_repository.rs` (150-200行)

### 5.2 Category Repository

```rust
#[async_trait]
pub trait CategoryRepository {
    async fn save(&self, category: &Category) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError>;
    async fn find_by_slug(&self, slug: &Slug) -> Result<Option<Category>, RepositoryError>;
    async fn delete(&self, id: CategoryId) -> Result<(), RepositoryError>;
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Category>, RepositoryError>;
    async fn list_by_parent(&self, parent_id: CategoryId, limit: i64, offset: i64) -> Result<Vec<Category>, RepositoryError>;
}
```

**実装**: `src/infrastructure/repositories/diesel_category_repository.rs` (200-250行)

---

## Step 6: Entity Reconstruction

### 6.1 `reconstruct_category()` Helper

```rust
fn reconstruct_category(
    id: Uuid,
    name: String,
    slug: String,
    description: Option<String>,
    parent_id: Option<Uuid>,
    post_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Category
```

**実装**:

1. CategoryId(id) を作成
2. CategoryName(name) を検証・作成
3. Slug(slug) を検証・作成
4. Category Value Objects を作成
5. Category Entity を構築

**ファイル**: `src/infrastructure/repositories/diesel_category_repository.rs` に配置

---

## Step 7: Feature Gate & CI

### 7.1 Feature Gate

すべての Tag/Category コードに `#[cfg(feature = "restructure_domain")]` 適用

### 7.2 CI Matrix

既存の CI が自動的にカバー:
- `--all-features` ✅
- `--no-default-features --features restructure_domain` ✅
- `--features "database,auth,cache,search"` ✅

---

## テスト戦略

### 8.1 Domain Layer Tests (50-70 tests)

- **Tag Value Objects**: TagId, TagName (30 tests)
- **Category Value Objects**: CategoryId, CategoryName, Slug (40 tests)
- **Business Rules**: Slug uniqueness, parent hierarchy (20 tests)

### 8.2 Database Helper Tests (40-50 tests)

- **CRUD operations**: create, read, update, delete (20 tests)
- **Query operations**: list_all, list_in_use, list_by_parent (20 tests)
- **Pagination**: page boundaries, limits (10 tests)

### 8.3 Repository Tests (30-40 tests)

- **Entity reconstruction**: Tuple to Entity (15 tests)
- **Error handling**: Not found, duplicates (15 tests)

**テスト合計**: 120-160 新規テスト

**期待される累計**: 500 + 120-160 = 620-660 tests passing

---

## 実装順序

1. **Day 1**: スキーマ定義 + DB ヘルパー (Tag)
2. **Day 2**: Tag entity reconstruction + repository
3. **Day 3**: スキーマ定義 + DB ヘルパー (Category)
4. **Day 4**: Category entity reconstruction + repository
5. **Day 5**: ジョイナブル定義 + CI 検証
6. **Day 6-7**: 統合テスト + 最適化

---

## コミット計画

```
Phase 6.3.1: Add Tag/Category database schemas
Phase 6.3.2: Implement Tag CRUD helpers and entity reconstruction
Phase 6.3.3: Implement Tag repository with all methods
Phase 6.3.4: Implement Category CRUD helpers and entity reconstruction
Phase 6.3.5: Implement Category repository with all methods
Phase 6.3.6: Add joinable definitions and CI validation
```

---

## 成功基準

- ✅ 600+ テスト成功
- ✅ コンパイル時間 < 60秒
- ✅ 0 errors, 0 warnings
- ✅ すべての Tag/Category メソッド動作確認
- ✅ エンティティ復元パターン一貫性確認
- ✅ ページネーション正常動作

---

**準備完了**: Phase 6.3 開始準備中 🚀
