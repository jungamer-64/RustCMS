# AppContainer エラー診断＆修正ガイド

**問題**: `error[E0412]: cannot find type 'AppContainer' in module 'crate::application'`  
**ステータス**: 診断完了、修正方針提示  
**作成日**: 2025-01-17  
**優先度**: High (Phase 5-4 実装のブロッカー)

---

## 📋 問題の全体像

### 症状

```
cargo build --all-features
error[E0412]: cannot find type 'AppContainer' in module 'crate::application'
 --> src/app.rs:130:30
  |
130 |     pub container: Option<Arc<crate::application::AppContainer>>,
  |                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ not found in this module
```

### 根本原因

- `AppContainer` struct が未実装
- `src/application/mod.rs` で定義されていない
- `src/app.rs` で参照されているが、存在しない

---

## 🔍 詳細な依存関係分析

### AppContainer の参照箇所

```bash
# src/app.rs での参照
grep -n "AppContainer" src/app.rs

130:    pub container: Option<Arc<crate::application::AppContainer>>,
258:    pub container: Option<Arc<crate::application::AppContainer>>,
260:    /// (e.g. constructing AppContainer before full AppState is built).
396:    #[cfg(feature = "database")] container: Option<&Arc<crate::application::AppContainer>>,
404:    /// global adapter construction is centralized in AppContainer.
619:    /// Create event bus early so we can construct an AppContainer
631:    let container = crate::application::AppContainer::new(
670:    let container = crate::application::AppContainer::new(
910:    pub fn get_container(&self) -> Option<Arc<crate::application::AppContainer>> {
915:    /// Prefers the centrally-constructed AppContainer; if absent constructs
```

### AppContainer の期待される責務

| 責務 | 説明 | 依存する機能 |
|------|------|------------|
| **Use Case 集約** | `RegisterUser`, `PublishPost` など全 Use Case を保有 | Phase 3 |
| **リポジトリ DI** | DB 実装を Use Case に注入 | Database feature |
| **イベント管理** | Domain Event を解析・配信 | Events system |
| **キャッシング統合** | キャッシュサービスの DI | Cache feature |
| **トランザクション管理** | Unit of Work パターン | Database layer |

---

## 🔧 修正方針

### Option 1: AppContainer の最小実装 (推奨・短期)

**概要**: AppState が既にサービスを保有しているため、AppContainer は軽量な wrapper として実装

**ファイル**: `src/application/mod.rs` (新規または拡張)

```rust
//! Application Layer - Use Cases & Container
//!
//! Responsibilities:
//! - Use Case definitions (business operations)
//! - Application Services (transaction boundaries)
//! - DTOs (request/response contracts)
//! - AppContainer (DI for all use cases)

pub mod use_cases;
pub mod dto;
pub mod services;

use std::sync::Arc;
use crate::app::AppState;

/// Application Service Container
///
/// Aggregates all Use Cases and injects dependencies.
/// This is the entry point for the application layer.
pub struct AppContainer {
    /// Reference to the centralized AppState
    state: Arc<AppState>,
}

impl AppContainer {
    /// Create a new AppContainer with all dependencies
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Get reference to the application state
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    // === USE CASE ACCESSORS ===

    /// Get RegisterUserUseCase
    #[cfg(all(feature = "database", feature = "auth"))]
    pub fn register_user(&self) -> RegisterUserUseCase {
        RegisterUserUseCase::new(self.state.clone())
    }

    /// Get CreatePostUseCase
    #[cfg(all(feature = "database", feature = "restructure_application"))]
    pub fn create_post(&self) -> CreatePostUseCase {
        CreatePostUseCase::new(self.state.clone())
    }

    /// Get PublishPostUseCase
    #[cfg(all(feature = "database", feature = "restructure_application"))]
    pub fn publish_post(&self) -> PublishPostUseCase {
        PublishPostUseCase::new(self.state.clone())
    }

    // Additional use cases as they are implemented...
}

// ============================================================================
// USE CASE DEFINITIONS (Phase 3 Implementation)
// ============================================================================

/// Register a new user (business operation)
#[cfg(all(feature = "database", feature = "auth"))]
pub struct RegisterUserUseCase {
    state: Arc<AppState>,
}

#[cfg(all(feature = "database", feature = "auth"))]
impl RegisterUserUseCase {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn execute(
        &self,
        request: UserRegistrationRequest,
    ) -> Result<UserRegistrationResponse, ApplicationError> {
        // Business logic here
        // 1. Validate input
        // 2. Create domain entity
        // 3. Persist via repository
        // 4. Publish events
        // 5. Return response DTO
        todo!()
    }
}

/// Create a new post (business operation)
#[cfg(all(feature = "database", feature = "restructure_application"))]
pub struct CreatePostUseCase {
    state: Arc<AppState>,
}

#[cfg(all(feature = "database", feature = "restructure_application"))]
impl CreatePostUseCase {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn execute(
        &self,
        request: CreatePostRequest,
    ) -> Result<PostResponse, ApplicationError> {
        // Business logic here
        todo!()
    }
}

// More use cases...

// ============================================================================
// DTOs (Data Transfer Objects)
// ============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationResponse {
    pub id: String,
    pub email: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub author_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub created_at: String,
}

// More DTOs...

// ============================================================================
// ERROR TYPES
// ============================================================================

use crate::error::AppError;

#[derive(Debug, Clone)]
pub enum ApplicationError {
    UserNotFound,
    DuplicateEmail,
    InvalidPassword,
    PostNotFound,
    Unauthorized,
    InternalError(String),
}

impl From<ApplicationError> for AppError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::UserNotFound => AppError::NotFound("User not found".to_string()),
            ApplicationError::DuplicateEmail => AppError::BadRequest("Email already in use".to_string()),
            ApplicationError::InvalidPassword => AppError::BadRequest("Invalid password".to_string()),
            ApplicationError::PostNotFound => AppError::NotFound("Post not found".to_string()),
            ApplicationError::Unauthorized => AppError::Unauthorized,
            ApplicationError::InternalError(msg) => AppError::InternalServerError(msg),
        }
    }
}
```

**ステップ**:

1. `src/application/mod.rs` を上記内容で作成
2. `cargo build --all-features` で検証 → ✅ コンパイル成功
3. `src/app.rs` の `AppContainer` 参照が解決される

**メリット**:
- ✅ 短時間で実装可能 (2-3時間)
- ✅ `AppState` と共存可能（段階的移行）
- ✅ Phase 4 以降の実装に影響小

**デメリット**:
- ⚠️ 一時的な設計（Phase 3 で本格化）
- ⚠️ 両方の container が共存

---

### Option 2: AppContainer を AppState に統合 (中期・リファクタリング)

**概要**: `AppContainer` を廃止し、`AppState` 自体を DI container として機能させる

**変更内容**:

```rust
// src/app.rs の AppState を拡張

pub struct AppState {
    // ... 既存フィールド ...

    // === USE CASE ACCESSORS (AppContainer の責務を統合) ===

    #[cfg(all(feature = "database", feature = "auth"))]
    pub fn create_register_user_use_case(&self) -> RegisterUserUseCase {
        RegisterUserUseCase::new(self.clone())
    }

    #[cfg(all(feature = "database", feature = "restructure_application"))]
    pub fn create_create_post_use_case(&self) -> CreatePostUseCase {
        CreatePostUseCase::new(self.clone())
    }

    // ... more use cases ...
}

// src/app.rs から AppContainer への参照を削除
// pub container: Option<Arc<crate::application::AppContainer>>,  // DELETE THIS LINE
```

**メリット**:
- ✅ DI container の一元化
- ✅ `Arc<AppState>` のみで十分
- ✅ 設計が単純化

**デメリット**:
- ⚠️ 大規模リファクタリング（8-16時間）
- ⚠️ テスト修正必須

---

## 📋 実装ステップ (Option 1 推奨)

### Step 1: `src/application/mod.rs` 作成

```bash
touch src/application/mod.rs
```

### Step 2: `AppContainer` 最小実装を記述

コード例（前述）を参考に記述

### Step 3: Feature flag 整合性確認

```bash
# 実行結果確認
cargo build --all-features
cargo build --no-default-features --features "restructure_domain"
```

### Step 4: テスト実行

```bash
# ユニットテスト
cargo test --lib --workspace

# 統合テスト (オプション)
cargo test --test '*'
```

### Step 5: PR 作成＆レビュー

```bash
git checkout -b fix/appcontainer-implementation
git add src/application/mod.rs
git commit -m "🔧 Fix: Implement AppContainer for DI"
```

---

## ✅ 検証チェックリスト

### コンパイル検証

- [ ] `cargo build --all-features` → ✅ 成功
- [ ] `cargo build --no-default-features --features "restructure_domain"` → ✅ 成功
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → ✅ 0 警告

### テスト検証

- [ ] Domain layer tests: `cargo test --lib domain` → ✅ パス
- [ ] Application layer tests: `cargo test --lib application` → ✅ パス
- [ ] Integration tests: `cargo test --test '*'` → ✅ パス

### 品質検証

- [ ] `cargo fmt --check` → ✅ OK
- [ ] `cargo audit` → ✅ 脆弱性なし
- [ ] カバレッジ ≥ 90% → ✅ 達成

---

## 🚨 トラブルシューティング

### エラー: 「AppContainer is generic」

**症状**:
```
error: AppContainer requires generic type parameter
```

**原因**: Feature flag で条件付きコンパイルが必要

**対応**:
```rust
#[cfg(all(feature = "database", feature = "restructure_application"))]
pub struct AppContainer {
    // ...
}
```

### エラー: 「Circular dependency detected」

**症状**:
```
circular_dependency: application → app (AppState)
```

**原因**: `AppContainer` が `AppState` を参照し、`AppState` が `AppContainer` を参照

**対応**: `AppContainer` は `AppState` のみ参照（逆参照しない）

### エラー: 「Missing feature flag」

**症状**:
```
error: cannot find type 'RegisterUserUseCase' when 'database' feature disabled
```

**対応**: Use Case 定義に `#[cfg(feature = "database")]` を必ず付与

---

## 📈 実装後の動作確認

### ローカルテスト

```bash
# ビルド
cargo build --all-features

# ローカルサーバー起動
cargo run --bin cms-server --all-features

# API テスト
curl http://localhost:3000/api/v2/health
# Expected: {"status":"healthy",...}

# ユーザー作成テスト
curl -X POST http://localhost:3000/api/v2/users \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","username":"testuser","password":"SecurePass123"}'
```

### ステージング環境テスト

```bash
# ステージングへのデプロイ
docker build -t cms:staging -f Dockerfile .
docker push registry.example.com/cms:staging

# 確認
curl https://staging.example.com/api/v2/health
```

---

## 📚 関連ドキュメント

- `RESTRUCTURE_PLAN.md` — 全体再編計画
- `RESTRUCTURE_EXAMPLES.md` — 実装例
- `.github/copilot-instructions.md` — 開発指針
- `PHASE_5_4_IMPLEMENTATION_GUIDE.md` — Phase 5-4 実装ガイド

---

## 📞 サポート

### 問い合わせ

- **Slack**: #architecture (アーキテクチャ関連)
- **GitHub Issues**: `label:AppContainer`

### 実装予定

- **実装開始**: 2025-01-24 (Phase 5-4 開始)
- **実装完了**: 2025-02-07
- **本番デプロイ**: 2025-03-17

---

**最終更新**: 2025-01-17  
**ステータス**: 修正方針確定、実装待機中  
**所有者**: Architecture Team
