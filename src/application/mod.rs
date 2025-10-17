//! アプリケーション層 (Application Layer)
//!
//! ユースケースの実装とポート定義を担うレイヤーです。
//! - Use Cases: ビジネスユースケースの実装
//! - Ports: インターフェース定義（Repository, Service等）
//! - DTOs: Data Transfer Objects
//! - Commands/Queries: CQRS パターン
//! - AppContainer: Use Case 集約・DI コンテナ

use std::sync::Arc;

// ============================================================================
// Phase 3: 新しいアプリケーション層構造（監査済み）
// ============================================================================

pub mod dto;
pub mod ports;

// Phase 3 Week 11: CQRS Queries
#[cfg(feature = "restructure_domain")]
pub mod queries;

// ============================================================================
// レガシー構造（既存コードとの並行稼働）
// ============================================================================

// Note: handlers::* is re-exported first to avoid ambiguity with ports::search
pub mod handlers {
    pub use crate::handlers::*;
}

pub use handlers::*;

// Ports - intentionally after handlers to avoid search module conflict
// Only re-export specific items to avoid glob conflicts
#[allow(unused_imports)] // Used conditionally
pub use ports::{CacheService, EventPublisher, PostRepository, UserRepository};
pub use ports::{cache, events, repositories};

pub mod use_cases;
pub use use_cases::*;

pub mod services {
    // Re-exports for service-like modules (eg: limiter, auth glue) can go here.
}

/// Application prelude: commonly used handler/service types for migrating
/// call sites to `crate::application`.
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
#[derive(Clone)]
pub struct AppContainer {
    /// AppState への参照（全サービスを含む）
    state: Arc<crate::app::AppState>,
}

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
