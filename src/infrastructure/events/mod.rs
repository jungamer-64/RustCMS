//! Infrastructure Events Module
//!
//! イベントシステムの実装を担うモジュールです。
//! Phase 3-4 で src/events.rs と src/listeners.rs から移行されました。
//!
//! ## 構成
//! - `bus.rs`: EventBus実装（元 src/events.rs）
//! - `listeners.rs`: イベントリスナー統合（元 src/listeners.rs）
//!
//! ## 設計原則
//! - EventPublisher port を実装
//! - Fire-and-Forget パターン
//! - Broadcast channel を使用した非同期イベント配信

pub mod bus;
pub mod listeners;

// Re-exports for convenience
pub use bus::{AppEvent, EventBus, create_event_bus};
pub use listeners::spawn_event_listeners;

/// Events prelude
pub mod prelude {
    pub use super::bus::{AppEvent, EventBus, create_event_bus};
    pub use super::listeners::spawn_event_listeners;
}
