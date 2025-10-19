# Phase 6-A: 依存元移行実行計画

**作成日**: 2025年10月19日  
**Phase**: Phase 6-A - 依存元を新構造に移行（削除準備）  
**目標**: src/models/ と src/repositories/ への依存を解消し、Phase 6-B での安全な削除を可能にする

---

## 📊 依存関係マップ（詳細）

### src/models/ への依存（20箇所）

```
【レガシー構造からの参照】
1. src/repositories/user_repository.rs
   - use crate::models::User;

2. src/utils/common_types.rs
   - use crate::models::{User, UserRole};
   - dto_from_model! マクロで User 変換

3. src/utils/paginate.rs
   - use crate::models::pagination::Paginated;

4. src/domain/mod.rs ⚠️ 重要
   - pub use crate::models::*;
   - Domain層がレガシーに依存している矛盾

【新構造からの参照】
5. src/web/handlers/users.rs
   - use crate::models::pagination::{Paginated, normalize_page_limit};
   - use crate::models::UpdateUserRequest;

6. src/web/handlers/posts.rs
   - use crate::models::pagination::{Paginated, normalize_page_limit};

7. src/web/handlers/api_keys.rs
   - crate::models::api_key::ApiKey::mask_raw(&raw)

【管理ツールからの参照】
8. src/bin/admin/cli.rs
   - use cms_backend::models::UserRole;

9. src/bin/admin/util.rs
   - use cms_backend::{Result, models::User};

10. src/bin/migrate.rs
    - cms_backend::models::CreateUserRequest

【Infrastructure層からの参照】
11. src/infrastructure/repositories/diesel_user_repository.rs
    - crate::models::User (返り値)

12. src/infrastructure/events/bus.rs
    - pub fn from_user(user: &crate::models::user::User)

13. src/infrastructure/database/mod.rs
    - pub use models::{...}

【その他】
14. src/app.rs
    - crate::models::User (複数箇所)

15. src/database/mod.rs
    - use crate::repositories::UserRepository;
```

---

## 🎯 Phase 6-A 実行ステップ

### Step 1: src/domain/mod.rs の pub use 削除 ⚠️ 最優先

**問題**: Domain層がレガシーmodels に依存している矛盾

**現状**:
```rust
// src/domain/mod.rs
pub use crate::models::*;  // ❌ レガシー依存
```

**対応**:
```rust
// src/domain/mod.rs
// pub use crate::models::*;  // 削除

// 新構造のみを公開
pub use user::{User, UserId, Email, Username};
pub use post::{Post, PostId, Slug, Title, Content};
pub use comment::{Comment, CommentId};
pub use category::{Category, CategoryId};
pub use tag::{Tag, TagId};
```

**影響範囲**: domain/mod.rs のみ（他への影響なし）

---

### Step 2: src/utils/ → src/common/ 移行

#### 移行対象ファイル

```
src/utils/
├── common_types.rs  → src/common/types.rs に統合
└── paginate.rs      → src/common/pagination.rs に統合
```

#### 2-1. common_types.rs 移行

**現状の問題**:
```rust
// src/utils/common_types.rs
use crate::models::{User, UserRole};  // ❌ レガシー依存
```

**移行先**: `src/common/types.rs`（既存ファイルに追加）

**移行内容**:
```rust
// src/common/types.rs に追加

// SessionId は既に存在するため、UserInfo のみ追加
use crate::domain::user::{User, UserRole};  // ✅ 新構造参照

/// Unified user information for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    // ... 既存フィールド
}

// dto_from_model! マクロを削除し、手動変換に置き換え
impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        // domain::User から変換
    }
}
```

#### 2-2. paginate.rs 移行

**現状の問題**:
```rust
// src/utils/paginate.rs
use crate::models::pagination::Paginated;  // ❌ レガシー依存
```

**移行先**: `src/common/pagination.rs`（新規作成）

**移行内容**:
```rust
// src/common/pagination.rs（新規）
use crate::application::queries::pagination::PaginationResult;  // ✅ 新構造参照

// fetch_paginated() などのヘルパー関数を移行
// Paginated<T> → PaginationResult<T> に置き換え
```

**依存更新**:
```rust
// src/web/handlers/users.rs
// BEFORE
use crate::models::pagination::{Paginated, normalize_page_limit};

// AFTER
use crate::common::pagination::PaginationResult;
use crate::application::queries::pagination::normalize_page_limit;
```

---

### Step 3: src/handlers/ の依存解消

#### 対象ファイル

```
src/handlers/
├── users.rs     - models::UpdateUserRequest 参照
├── posts.rs     - models::pagination 参照
└── api_keys.rs  - models::api_key 参照
```

#### 3-1. users.rs 更新

```rust
// src/handlers/users.rs
// BEFORE
use crate::models::UpdateUserRequest;
use crate::models::pagination::{Paginated, normalize_page_limit};

// AFTER
use crate::application::dto::user::UpdateUserRequest;
use crate::common::pagination::PaginationResult;
```

#### 3-2. posts.rs 更新

```rust
// src/handlers/posts.rs
// BEFORE
use crate::models::pagination::{Paginated, normalize_page_limit};

// AFTER
use crate::common::pagination::PaginationResult;
```

#### 3-3. api_keys.rs 更新

```rust
// src/handlers/api_keys.rs
// BEFORE
use crate::models::api_key::ApiKey;

// AFTER
use crate::infrastructure::database::models::ApiKey;
// または domain に ApiKey エンティティ作成
```

---

### Step 4: infrastructure/ の依存解消

#### 4-1. diesel_user_repository.rs 更新

```rust
// src/infrastructure/repositories/diesel_user_repository.rs
// BEFORE
fn get_user_by_email(&self, email: &str) -> BoxFuture<'_, crate::Result<crate::models::User>>

// AFTER
fn get_user_by_email(&self, email: &str) -> BoxFuture<'_, crate::Result<crate::domain::user::User>>
```

#### 4-2. events/bus.rs 更新

```rust
// src/infrastructure/events/bus.rs
// BEFORE
pub fn from_user(user: &crate::models::user::User) -> Self

// AFTER
pub fn from_user(user: &crate::domain::user::User) -> Self
```

---

### Step 5: bin/ の依存解消

#### 5-1. admin/cli.rs 更新

```rust
// src/bin/admin/cli.rs
// BEFORE
use cms_backend::models::UserRole;

// AFTER
use cms_backend::domain::user::UserRole;
```

#### 5-2. migrate.rs 更新

```rust
// src/bin/migrate.rs
// BEFORE
let admin_user = cms_backend::models::CreateUserRequest { ... };

// AFTER
let admin_user = cms_backend::application::dto::user::CreateUserRequest { ... };
```

---

### Step 6: src/app.rs の依存解消

```rust
// src/app.rs
// BEFORE
_user: crate::models::User,

// AFTER
_user: crate::domain::user::User,
```

---

## 📋 実行チェックリスト

### Step 1: domain/mod.rs 更新

- [ ] `pub use crate::models::*;` を削除
- [ ] 新構造の型のみを pub use で公開
- [ ] ビルド確認: `cargo build --lib --features "restructure_domain"`

### Step 2: utils/ → common/ 移行

- [ ] common_types.rs → common/types.rs に統合
  - [ ] UserInfo 構造体を移行
  - [ ] dto_from_model! マクロを削除
  - [ ] From<User> impl を手動実装
- [ ] paginate.rs → common/pagination.rs に移行
  - [ ] fetch_paginated() を移行
  - [ ] Paginated → PaginationResult に置き換え
- [ ] ビルド確認

### Step 3: handlers/ 更新

- [ ] users.rs の use 文更新
- [ ] posts.rs の use 文更新
- [ ] api_keys.rs の use 文更新
- [ ] ビルド確認

### Step 4: infrastructure/ 更新

- [ ] diesel_user_repository.rs の返り値型更新
- [ ] events/bus.rs の User 参照更新
- [ ] ビルド確認

### Step 5: bin/ 更新

- [ ] admin/cli.rs の use 文更新
- [ ] admin/util.rs の use 文更新
- [ ] migrate.rs の CreateUserRequest 更新
- [ ] ビルド確認

### Step 6: app.rs 更新

- [ ] User 型参照を domain::user::User に更新
- [ ] ビルド確認

### Step 7: 全体ビルド確認

- [ ] `cargo build --lib --all-features`
- [ ] `cargo test --lib --all-features`
- [ ] `cargo clippy --all-features -- -D warnings`

---

## 🚨 リスク評価

### 高リスク項目

1. **dto_from_model! マクロ削除**
   - 影響: common_types.rs の UserInfo 変換
   - 対策: 手動 impl From<User> で置き換え

2. **Paginated → PaginationResult 型変更**
   - 影響: 全ハンドラのページネーション処理
   - 対策: 段階的に更新、型エイリアスで一時的に互換性維持

### 中リスク項目

1. **infrastructure/ の User 型変更**
   - 影響: Repository 返り値型
   - 対策: domain::user::User に統一

2. **bin/ の依存更新**
   - 影響: 管理ツールのビルド
   - 対策: use 文のパス変更のみ

### 低リスク項目

1. **domain/mod.rs の pub use 削除**
   - 影響: domain 内部のみ
   - 対策: 明示的な pub use で型を公開

---

## 📊 完了条件

Phase 6-A 完了判定：

- [x] src/models/ への use crate::models:: 参照が0件
- [x] src/repositories/ への use crate::repositories:: 参照が0件（application/ports, infrastructure 除く）
- [x] 全feature flagsでビルド成功
- [x] 全テストパス（--all-features）
- [x] Phase 6-A 完了レポート作成

---

## 🔜 Phase 6-B への移行条件

Phase 6-A 完了後、以下を確認してから Phase 6-B へ：

1. ✅ すべての依存が新構造に移行済み
2. ✅ ビルド・テストが安定稼働
3. ✅ コードレビュー完了
4. ✅ Git コミット（Phase 6-A 完了時点）

Phase 6-B では：
- `rm -rf src/models/`
- `rm -rf src/repositories/`（旧版のみ）
- `src/lib.rs` のモジュール宣言削除

---

## 📝 推定作業時間

| ステップ | 作業時間 | 備考 |
|---------|---------|------|
| Step 1: domain/mod.rs | 30分 | リスク低 |
| Step 2: utils/ → common/ | 2時間 | マクロ削除が主な作業 |
| Step 3: handlers/ 更新 | 1時間 | use 文変更のみ |
| Step 4: infrastructure/ 更新 | 1時間 | 型変更 |
| Step 5: bin/ 更新 | 30分 | use 文変更のみ |
| Step 6: app.rs 更新 | 30分 | 型変更 |
| Step 7: ビルド・テスト | 1時間 | エラー修正含む |
| **合計** | **6.5時間** | 1日で完了可能 |

---

## 🎯 次のアクション

Step 1 から順に実行：

```bash
# Step 1: domain/mod.rs 更新
# src/domain/mod.rs を編集

# ビルド確認
cargo build --lib --features "restructure_domain"
```

準備完了。Step 1 から開始しますか？
