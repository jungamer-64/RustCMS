//! Cache Service Port (インターフェース定義)
//!
//! キャッシュサービスの Port/Adapter パターンによるインターフェース定義です。
//! Infrastructure層がこれらのtraitを実装します。
//!
//! ## 設計原則
//! - Redis と Memory キャッシュの両方をサポート
//! - Send + Sync制約でスレッド安全性を保証
//! - 非同期メソッド定義 (async_trait)

use async_trait::async_trait;
use std::time::Duration;

/// キャッシュサービス（Port/Interface）
///
/// Redis や Memory キャッシュへのアクセスを抽象化します。
/// Infrastructure層で具体的な実装（RedisCacheService等）を提供します。
#[async_trait]
pub trait CacheService: Send + Sync {
    /// キーに対応する値を取得
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;

    /// キーと値をキャッシュに保存（有効期限付き）
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError>;

    /// キーを削除
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn delete(&self, key: &str) -> Result<(), CacheError>;

    /// キーが存在するかチェック
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// キャッシュ全体をクリア（開発/テスト用）
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn clear(&self) -> Result<(), CacheError>;

    /// 複数キーを一括取得
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn multi_get(&self, keys: &[&str]) -> Result<Vec<Option<String>>, CacheError>;

    /// 複数キーを一括削除
    ///
    /// # Errors
    ///
    /// キャッシュアクセスエラーが発生した場合
    async fn multi_delete(&self, keys: &[&str]) -> Result<(), CacheError>;
}

/// キャッシュサービスのエラー型
#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Cache operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Unknown cache error: {0}")]
    Unknown(String),
}

// Phase 3: Infrastructure層での実装例
//
// ```rust
// pub struct RedisCacheService {
//     client: redis::Client,
// }
//
// #[async_trait]
// impl CacheService for RedisCacheService {
//     async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
//         // Redis実装
//     }
//     // ... 他のメソッド
// }
// ```
