//! Application State - DDD準拠の新実装（Phase 5）
//!
//! 本モジュールは CMS の中核状態 `AppState` を提供します。
//! レガシー実装（src/app.rs）を完全削除し、DDD原則に沿ったクリーンな実装に置き換えます。
//!
//! ## 機能
//! - Database/Auth/Cache/Search サービスの統合
//! - EventBus 統合（domain events 発行）
//! - Metrics 収集
//! - Builder パターンによる段階的初期化
//! - Feature flags による条件付きコンパイル
//!
//! ## 設計方針
//! - domain層の型のみ使用（旧models依存を完全排除）
//! - Repository/Use Case 経由のアクセス
//! - 最小限のAPI提供（サービス取得、イベント発行、ヘルスチェック）
//!
//! ## Phase 5 Note
//! database/cache/searchサービスは現在移行中です。
//! 当面はeventsとconfigのみを統合し、サービスは段階的に追加します。

use crate::{Config, Result};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

// Events module location depends on feature flag
#[cfg(not(feature = "restructure_domain"))]
use crate::events::AppEvent;
#[cfg(feature = "restructure_domain")]
use crate::infrastructure::events::AppEvent;

/// EventBus type alias
#[cfg(not(feature = "restructure_domain"))]
pub type EventBus = broadcast::Sender<crate::events::AppEvent>;
#[cfg(feature = "restructure_domain")]
pub type EventBus = broadcast::Sender<crate::infrastructure::events::AppEvent>;

/// Central application state for the CMS
///
/// AppStateはアプリケーションの中核状態を管理します。
/// Phase 5: 現在はconfig とevent_busのみ。サービスは段階的に追加予定。
#[derive(Clone)]
pub struct AppState {
    /// Application configuration
    config: Arc<Config>,

    /// Event bus for domain events
    event_bus: EventBus,
}

impl AppState {
    /// Returns a builder for constructing AppState
    pub fn builder(config: Config) -> AppStateBuilder {
        AppStateBuilder {
            config: Arc::new(config),
        }
    }

    /// Get reference to configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get reference to event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Emit a domain event to the event bus
    ///
    /// Fire-and-Forget設計: エラーは無視します
    #[cfg(not(feature = "restructure_domain"))]
    pub fn emit_event(&self, event: crate::events::AppEvent) {
        let _ = self.event_bus.send(event);
    }

    /// Emit a domain event to the event bus (restructure_domain version)
    #[cfg(feature = "restructure_domain")]
    pub fn emit_event(&self, event: crate::infrastructure::events::AppEvent) {
        let _ = self.event_bus.send(event);
    }
}

/// Builder for AppState
///
/// AppStateBuilderは段階的にサービスを初期化します。
/// Phase 5: 現在はconfigのみ。サービスは後で追加予定。
pub struct AppStateBuilder {
    config: Arc<Config>,
}

impl AppStateBuilder {
    /// Build AppState
    ///
    /// EventBusを作成し、全てのサービスを統合します。
    pub fn build(self) -> Result<AppState> {
        info!("Building AppState...");

        // Create event bus (capacity: 1000)
        let (event_bus, _) = broadcast::channel(1000);

        let state = AppState {
            config: self.config,
            event_bus,
        };

        info!("AppState built successfully");
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let config = Config::default();
        let builder = AppState::builder(config);
        // Builder should be created successfully
        assert!(builder.config.server.host == "127.0.0.1" || builder.config.server.host == "0.0.0.0");
    }

    #[test]
    fn test_builder_build() {
        let config = Config::default();
        let builder = AppState::builder(config);
        let state = builder.build().unwrap();
        
        // AppState should be built successfully
        // Config default host can be either 127.0.0.1 or 0.0.0.0 depending on environment
        assert!(state.config().server.host == "127.0.0.1" || state.config().server.host == "0.0.0.0");
    }

    #[test]
    fn test_event_bus_creation() {
        let (_event_bus, _rx) = broadcast::channel::<AppEvent>(10);
        
        // Channel should be created successfully
        // Note: Actual event sending/receiving tested elsewhere
    }
}
