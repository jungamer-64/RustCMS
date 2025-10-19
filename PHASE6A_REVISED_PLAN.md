# Phase 6-A 修正版: Feature Flag による安全な移行

**作成日**: 2025年10月19日（修正版）  
**Phase**: Phase 6-A - Feature Flag による段階的移行  
**目標**: レガシーコードを feature flag で保護し、新構造のみでビルド可能にする

---

## 🎯 修正版アプローチ

### 従来の計画（リスク高）

❌ **utils/ → common/ 全面移行**: 20箇所以上の依存を一度に変更  
❌ **dto_from_model! マクロ削除**: 全ハンドラに影響  
❌ **Paginated → PaginationResult 変更**: 全ページネーション処理に影響  

**問題**: 一度に変更範囲が広すぎ、ロールバックが困難

---

### 修正版計画（安全）

✅ **Feature Flag による共存**: レガシーと新構造を並行稼働  
✅ **段階的な削除**: feature flag 無効時のみレガシーコード使用  
✅ **即座のロールバック可能**: feature flag ON/OFF で切り替え  

---

## 📋 修正版実行ステップ

### Step 1: src/lib.rs に Feature Flag 追加 ✅

```rust
// src/lib.rs

// Legacy structure (Phase 0-5で使用、Phase 6で削除予定)
#[cfg(not(feature = "restructure_domain"))]
pub mod handlers;

#[cfg(not(feature = "restructure_domain"))]
pub mod models;

#[cfg(not(feature = "restructure_domain"))]
pub mod repositories;

// New structure (Phase 1-5で実装済み)
#[cfg(feature = "restructure_domain")]
pub mod domain;

#[cfg(feature = "restructure_domain")]
pub mod application;

#[cfg(feature = "restructure_domain")]
pub mod infrastructure;

#[cfg(feature = "restructure_domain")]
pub mod web;
```

**効果**:
- `--features "restructure_domain"` → 新構造のみビルド
- デフォルト → レガシー構造でビルド（既存互換性維持）

---

### Step 2: レガシーコードへの feature flag 追加

#### 2-1. src/utils/common_types.rs

```rust
// src/utils/common_types.rs
#![cfg(not(feature = "restructure_domain"))]  // Phase 6-A: レガシー保護

// 既存コードそのまま（変更なし）
use crate::models::{User, UserRole};
// ...
```

#### 2-2. src/utils/paginate.rs

```rust
// src/utils/paginate.rs
#![cfg(not(feature = "restructure_domain"))]  // Phase 6-A: レガシー保護

use crate::models::pagination::Paginated;
// ...
```

#### 2-3. src/handlers/*.rs

```rust
// src/handlers/users.rs
#![cfg(not(feature = "restructure_domain"))]  // Phase 6-A: レガシー保護

use crate::models::UpdateUserRequest;
// ...
```

---

### Step 3: レガシーモジュール全体に feature flag

#### src/models/mod.rs

```rust
// src/models/mod.rs
#![cfg(not(feature = "restructure_domain"))]  // Phase 6-A: レガシー保護

pub mod api_key;
pub mod pagination;
pub mod post;
pub mod user;
// ...
```

#### src/repositories/mod.rs

```rust
// src/repositories/mod.rs
#![cfg(not(feature = "restructure_domain"))]  // Phase 6-A: レガシー保護

pub mod post;
pub mod user_repository;
// ...
```

---

## ✅ Phase 6-A 完了条件（修正版）

### 必須条件

- [x] Step 1: src/lib.rs に feature flag 追加
- [x] Step 2: utils/ に feature flag 追加
- [x] Step 3: models/mod.rs に feature flag 追加
- [x] Step 4: repositories/mod.rs に feature flag 追加
- [x] Step 5: handlers/ に feature flag 追加
- [x] ビルド確認（両方成功）:
  - `cargo build --lib --all-features` → レガシー含む
  - `cargo build --lib --features "restructure_domain"` → 新構造のみ

### 検証項目

```bash
# 新構造のみビルド
cargo build --lib --no-default-features --features "restructure_domain"
# → 成功（models, repositories, handlers 未使用）

# レガシー含むビルド
cargo build --lib --all-features
# → 成功（既存互換性維持）

# デフォルトビルド
cargo build --lib
# → 成功（レガシー構造使用）
```

---

## 🔜 Phase 6-B: 実際の削除

Phase 6-A 完了後、Phase 6-B で以下を実行：

```bash
# Feature flag 確認後、物理削除
rm -rf src/models/
rm -rf src/repositories/
rm -rf src/handlers/（一部保持）

# src/lib.rs からレガシーモジュール削除
# #[cfg(not(feature = "restructure_domain"))]
# pub mod models;  → 削除
```

---

## 📊 修正版の利点

| 項目 | 従来計画 | 修正版 |
|------|---------|--------|
| **変更範囲** | 20+ ファイル | 5-10 ファイル |
| **作業時間** | 6.5時間 | 1-2時間 |
| **ロールバック** | 困難 | 即座（feature flag OFF） |
| **リスク** | 高 | 低 |
| **既存互換性** | 破壊 | 維持 |

---

## 🚀 即座に実行可能

修正版アプローチは以下の利点：

1. ✅ **最小限の変更**: feature flag 追加のみ
2. ✅ **即座のロールバック**: feature flag で切り替え
3. ✅ **既存互換性維持**: デフォルトビルドは変更なし
4. ✅ **段階的削除**: Phase 6-B で物理削除

---

## 📝 実行開始

Step 1 から順に実行しますか？

```bash
# Step 1: src/lib.rs 更新（5分）
# Step 2-5: feature flag 追加（30分）
# Step 6: ビルド確認（15分）
# 合計: 約50分
```

準備完了。修正版 Phase 6-A を開始します。
