// src/application/ports/events.rs
//! Event Publisher Port (インターフェース定義)
//!
//! イベント発行サービスの Port/Adapter パターンによるインターフェース定義です。
//! Infrastructure層がこれらのtraitを実装します。
//!
//! ## 設計原則
//! - Domain Events の発行を抽象化
//! - Send + Sync制約でスレッド安全性を保証
//! - 非同期メソッド定義 (async_trait)
//!
//! ## 監査推奨
//! - このファイルは `application/ports/` に配置
//! - `domain/events.rs` でイベント型を定義
//! - `infrastructure/events/` で実装を提供

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Phase 2: Domain Events の再エクスポート
// NOTE: DomainEvent は domain::events で定義されているが、
// Phase 3-4 で実装予定のため、暫定的にプレースホルダーを使用
pub use domain::events::DomainEvent;

/// イベント発行サービス（Port/Interface）
///
/// Domain Events を Infrastructure層（EventBus等）に発行します。
/// Infrastructure層で具体的な実装（BroadcastEventPublisher等）を提供します。
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// 単一イベントを発行
    ///
    /// # Errors
    ///
    /// イベント発行エラーが発生した場合
    async fn publish(&self, event: DomainEvent) -> Result<(), EventError>;

    /// 複数イベントを一括発行
    ///
    /// # Errors
    ///
    /// イベント発行エラーが発生した場合
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), EventError>;
}

/// イベント発行のエラー型
#[derive(Debug, Clone, thiserror::Error)]
pub enum EventError {
    #[error("Event bus connection error: {0}")]
    ConnectionError(String),

    #[error("Event serialization error: {0}")]
    SerializationError(String),

    #[error("Event publish failed: {0}")]
    PublishFailed(String),

    #[error("Unknown event error: {0}")]
    Unknown(String),
}

/// 軽量イベントメタデータ（監視用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: i64, // Unix timestamp
    pub source: String,
}

// Phase 3-4: Infrastructure層での実装例
//
// ```rust
// pub struct BroadcastEventPublisher {
//     sender: tokio::sync::broadcast::Sender<AppEvent>,
// }
//
// #[async_trait]
// impl EventPublisher for BroadcastEventPublisher {
//     async fn publish(&self, event: DomainEvent) -> Result<(), EventError> {
//         let app_event = AppEvent::from(event);
//         self.sender.send(app_event).map_err(|e| EventError::PublishFailed(e.to_string()))?;
//         Ok(())
//     }
//
//     async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), EventError> {
//         for event in events {
//             self.publish(event).await?;
//         }
//         Ok(())
//     }
// }
// ```
