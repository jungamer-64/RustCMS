//! アプリケーション層 (Application Layer) - 監査済み構造
//!
//! Commands + Queries + DTOs を統合した CQRS パターンを採用します。
//!
//! ## 構造（監査推奨）
//! - **user.rs**: User CQRS統合（Commands + Queries + DTOs）
//! - **post.rs**: Post CQRS統合（Commands + Queries + DTOs）
//! - **comment.rs**: Comment CQRS統合（Commands + Queries + DTOs）
//! - **category.rs**: Category CQRS統合（Commands + Queries + DTOs）
//! - **dto/**: 共通DTOモジュール（pagination等）
//! - **ports/**: インターフェース定義（Repository, Service等）
//!
//! ## 設計原則
//! - Entity + Value Objects 統合パターン（domain層）
//! - Commands + Queries + DTOs 統合パターン（application層）
//! - 500行未満は単一ファイル推奨
//! - Repository Port への依存性注入

use std::sync::Arc;

// ============================================================================
// Phase 3 完成版: CQRS統合構造（監査済み）
// ============================================================================

/// DTOs - Data Transfer Objects（共通モジュール）
pub mod dto;
pub use dto::*;

/// Ports - インターフェース定義（Repository, Service等）
pub mod ports;
pub use ports::{cache, events, repositories};
// Phase 9: search removed
// pub use ports::search;

// Re-export commonly used types (feature-gated)
#[cfg(feature = "restructure_domain")]
pub use ports::repositories::{
    CategoryRepository, CommentRepository, PostRepository, RepositoryError, TagRepository,
    UserRepository,
};

pub use ports::{CacheService, EventPublisher};

// ============================================================================
// CQRS統合モジュール（Commands + Queries + DTOs）
// ============================================================================

/// User CQRS統合（Commands + Queries + DTOs）
#[cfg(feature = "restructure_domain")]
pub mod user;

/// Post CQRS統合（Commands + Queries + DTOs）
#[cfg(feature = "restructure_domain")]
pub mod post;

/// Comment CQRS統合（Commands + Queries + DTOs）
#[cfg(feature = "restructure_domain")]
pub mod comment;

/// Category CQRS統合（Commands + Queries + DTOs）
#[cfg(feature = "restructure_domain")]
pub mod category;

/// Queries（CQRSクエリ層）
#[cfg(feature = "restructure_domain")]
pub mod queries;

// ============================================================================
// レガシー構造（既存コードとの並行稼働）Phase 6-B: Feature flag 保護
// ============================================================================

#[cfg(not(feature = "restructure_domain"))]
pub mod handlers {
    pub use crate::handlers::*;
}

#[cfg(not(feature = "restructure_domain"))]
pub use handlers::*;

pub mod use_cases;
pub use use_cases::*;

pub mod services {
    // Re-exports for service-like modules (eg: limiter, auth glue) can go here.
}

/// Application prelude: commonly used handler/service types for migrating
/// call sites to `crate::application`.
#[cfg(not(feature = "restructure_domain"))]
pub mod prelude {
    pub use super::handlers::*;
}

// ============================================================================
// AppContainer - Use Case DI Container (Phase 5-4 実装)
// ============================================================================

/// Application Service Container
///
/// アプリケーション層の Use Case を集約し、依存性注入を行うコンテナ。
/// AppState へのラッパーとして機能し、各 Use Case へのアクセスを提供する。
///
/// # 責務
/// - Use Case のファクトリメソッド提供
/// - リポジトリ・サービスの注入
/// - トランザクション境界の管理
///
/// # 設計方針
/// - AppState が既に全サービスを保有しているため、AppContainer は軽量
/// - feature flag で段階的に Use Case を追加
/// - Phase 5-4 で最小実装、Phase 5-5 で拡張
#[cfg(not(feature = "restructure_domain"))]
#[derive(Clone)]
pub struct AppContainer {
    /// AppState への参照（全サービスを含む）
    state: Arc<crate::app::AppState>,
}

#[cfg(not(feature = "restructure_domain"))]
impl AppContainer {
    /// 新しい AppContainer を作成
    ///
    /// # 引数
    /// - `state`: 初期化済みの AppState
    ///
    /// # 例
    /// ```rust,ignore
    /// let state = Arc::new(app_state);
    /// let container = AppContainer::new(state);
    /// ```
    pub fn new(state: Arc<crate::app::AppState>) -> Self {
        Self { state }
    }

    /// AppState への参照を取得（内部使用）
    pub fn state(&self) -> &Arc<crate::app::AppState> {
        &self.state
    }

    // ========================================================================
    // Use Case Accessors (feature-gated)
    // Phase 5-4: 既存の Use Case 実装と互換性を保つ
    // ========================================================================

    /// User 作成 Use Case（既存実装への互換レイヤー）
    #[cfg(feature = "database")]
    pub fn create_user(
        &self,
    ) -> Arc<
        crate::application::use_cases::CreateUserUseCase<
            crate::infrastructure::repositories::DieselUserRepository,
        >,
    > {
        let repo = crate::infrastructure::repositories::DieselUserRepository::new(
            self.state.database.clone(),
        );
        Arc::new(crate::application::use_cases::CreateUserUseCase::new(
            Arc::new(repo),
        ))
    }

    /// User 取得 Use Case（既存実装への互換レイヤー）
    #[cfg(feature = "database")]
    pub fn get_user_by_id(
        &self,
    ) -> Arc<
        crate::application::use_cases::GetUserByIdUseCase<
            crate::infrastructure::repositories::DieselUserRepository,
        >,
    > {
        let repo = crate::infrastructure::repositories::DieselUserRepository::new(
            self.state.database.clone(),
        );
        Arc::new(crate::application::use_cases::GetUserByIdUseCase::new(
            Arc::new(repo),
        ))
    }

    /// User 更新 Use Case（既存実装への互換レイヤー）
    #[cfg(feature = "database")]
    pub fn update_user(
        &self,
    ) -> Arc<
        crate::application::use_cases::UpdateUserUseCase<
            crate::infrastructure::repositories::DieselUserRepository,
        >,
    > {
        let repo = crate::infrastructure::repositories::DieselUserRepository::new(
            self.state.database.clone(),
        );
        Arc::new(crate::application::use_cases::UpdateUserUseCase::new(
            Arc::new(repo),
        ))
    }
}

// ============================================================================
// 注記: Phase 5-4 では既存の Use Case 実装を AppContainer 経由で提供
// Phase 5-5 で新しい Use Case を段階的に追加予定
// ============================================================================
