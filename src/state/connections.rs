//! 高性能接続プール管理
//! 
//! - HTTP接続の再利用
//! - 適応的プールサイズ調整
//! - 接続健全性チェック

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use smallvec::SmallVec;
use dashmap::DashMap;
use tokio::sync::Semaphore;
use super::FastHasher;

/// 接続統計
#[derive(Debug, Default, Clone)]
pub struct ConnectionStats {
    pub active_connections: u32,
    pub total_created: u64,
    pub total_reused: u64,
    pub total_errors: u64,
    pub average_connection_time: Duration,
}

/// 接続プール
#[derive(Debug)]
pub struct ConnectionPool {
    /// アクティブ接続数制限
    semaphore: Arc<Semaphore>,
    
    /// 接続統計
    stats: RwLock<ConnectionStats>,
    
    /// 接続履歴（パフォーマンス分析用）
    connection_history: DashMap<String, SmallVec<[ConnectionEvent; 16]>, FastHasher>,
    
    /// 設定
    #[allow(dead_code)]
    config: PoolConfig,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_connection_age: Duration,
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            max_connection_age: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
struct ConnectionEvent {
    timestamp: Instant,
    event_type: ConnectionEventType,
    #[allow(dead_code)]
    duration: Option<Duration>,
}

#[derive(Debug, Clone)]
enum ConnectionEventType {
    Created,
    Reused,
    Closed,
    Error,
    #[allow(dead_code)]
    HealthCheck,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    pub fn with_config(config: PoolConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_connections as usize)),
            stats: RwLock::new(ConnectionStats::default()),
            connection_history: DashMap::with_hasher(FastHasher::default()),
            config,
        }
    }

    /// 接続許可を取得（制限あり）
    pub async fn acquire_connection(&self) -> Result<ConnectionGuard, ConnectionError> {
        let permit = self.semaphore
            .acquire()
            .await
            .map_err(|_| ConnectionError::PoolClosed)?;

        let start_time = Instant::now();
        
        // 接続作成をシミュレート（実際にはHTTPクライアントなど）
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        // 統計更新
        {
            let mut stats = self.stats.write();
            stats.active_connections += 1;
            stats.total_created += 1;
        }

        // イベント記録
        self.record_event(&connection_id, ConnectionEventType::Created, Some(start_time.elapsed()));

        Ok(ConnectionGuard {
            _permit: permit,
            connection_id,
            pool: self,
            created_at: start_time,
        })
    }

    /// 接続再利用
    pub async fn reuse_connection(&self, connection_id: &str) -> Result<(), ConnectionError> {
        // 接続の健全性チェック
        if self.is_connection_healthy(connection_id).await {
            // 統計更新
            {
                let mut stats = self.stats.write();
                stats.total_reused += 1;
            }

            // イベント記録
            self.record_event(connection_id, ConnectionEventType::Reused, None);
            Ok(())
        } else {
            Err(ConnectionError::Unhealthy)
        }
    }

    /// 接続健全性チェック
    async fn is_connection_healthy(&self, connection_id: &str) -> bool {
        // 実際の実装では、pingやhealth checkを行う
        let history = self.connection_history.get(connection_id);
        
        if let Some(events) = history {
            // 最後のエラーから十分時間が経っているかチェック
            let last_error = events.iter()
                .filter(|e| matches!(e.event_type, ConnectionEventType::Error))
                .last();
                
            if let Some(error_event) = last_error {
                error_event.timestamp.elapsed() > Duration::from_secs(30)
            } else {
                true
            }
        } else {
            true
        }
    }

    /// エラー記録
    pub fn record_error(&self, connection_id: &str, error: &ConnectionError) {
        {
            let mut stats = self.stats.write();
            stats.total_errors += 1;
        }

        self.record_event(connection_id, ConnectionEventType::Error, None);
        tracing::warn!("Connection error for {}: {:?}", connection_id, error);
    }

    /// イベント記録
    fn record_event(&self, connection_id: &str, event_type: ConnectionEventType, duration: Option<Duration>) {
        let event = ConnectionEvent {
            timestamp: Instant::now(),
            event_type,
            duration,
        };

        self.connection_history
            .entry(connection_id.to_string())
            .or_insert_with(SmallVec::new)
            .push(event);
    }

    /// 統計取得
    pub fn stats(&self) -> ConnectionStats {
        self.stats.read().clone()
    }

    /// プールの健全性レポート
    pub fn health_report(&self) -> HealthReport {
        let stats = self.stats();
        let total_connections = stats.total_created + stats.total_reused;
        
        HealthReport {
            active_connections: stats.active_connections,
            total_connections,
            error_rate: if total_connections > 0 {
                stats.total_errors as f64 / total_connections as f64
            } else {
                0.0
            },
            reuse_rate: if stats.total_created > 0 {
                stats.total_reused as f64 / stats.total_created as f64
            } else {
                0.0
            },
            average_connection_time: stats.average_connection_time,
        }
    }

    /// 定期的なメンテナンス
    pub async fn maintenance(&self) {
        // 古い接続履歴をクリーンアップ
        let cutoff = Instant::now() - Duration::from_secs(3600); // 1時間前
        
        self.connection_history.retain(|_, events| {
            events.retain(|event| event.timestamp > cutoff);
            !events.is_empty()
        });

        tracing::debug!("Connection pool maintenance completed");
    }
}

/// 接続ガード（RAIIパターン）
pub struct ConnectionGuard<'a> {
    _permit: tokio::sync::SemaphorePermit<'a>,
    connection_id: String,
    pool: &'a ConnectionPool,
    created_at: Instant,
}

impl<'a> ConnectionGuard<'a> {
    pub fn id(&self) -> &str {
        &self.connection_id
    }

    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl<'a> Drop for ConnectionGuard<'a> {
    fn drop(&mut self) {
        // 接続終了時の統計更新
        {
            let mut stats = self.pool.stats.write();
            stats.active_connections -= 1;
            
            // 平均接続時間を更新
            let duration = self.duration();
            let current_avg = stats.average_connection_time;
            let total = stats.total_created;
            
            if total > 1 {
                stats.average_connection_time = Duration::from_nanos(
                    (((current_avg.as_nanos() * (total - 1) as u128) + duration.as_nanos()) / total as u128) as u64
                );
            } else {
                stats.average_connection_time = duration;
            }
        }

        // イベント記録
        self.pool.record_event(&self.connection_id, ConnectionEventType::Closed, Some(self.duration()));
    }
}

/// 健全性レポート
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub active_connections: u32,
    pub total_connections: u64,
    pub error_rate: f64,
    pub reuse_rate: f64,
    pub average_connection_time: Duration,
}

impl HealthReport {
    pub fn is_healthy(&self) -> bool {
        self.error_rate < 0.05 && // 5%未満のエラー率
        self.average_connection_time < Duration::from_secs(5) // 5秒未満の平均接続時間
    }
}

/// 接続エラー
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Connection pool is closed")]
    PoolClosed,
    
    #[error("Connection timeout")]
    Timeout,
    
    #[error("Connection is unhealthy")]
    Unhealthy,
    
    #[error("Network error: {0}")]
    Network(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let pool = ConnectionPool::new();
        
        // 接続取得
        let _guard = pool.acquire_connection().await.unwrap();
        
        let stats = pool.stats();
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_created, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_limit() {
        let config = PoolConfig {
            max_connections: 2,
            ..Default::default()
        };
        let pool = ConnectionPool::with_config(config);
        
        // 2つの接続を取得
        let _guard1 = pool.acquire_connection().await.unwrap();
        let _guard2 = pool.acquire_connection().await.unwrap();
        
        // 3つ目は制限により待機状態になる
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            pool.acquire_connection()
        ).await;
        
        assert!(result.is_err()); // タイムアウトで失敗
    }

    #[tokio::test]
    async fn test_health_report() {
        let pool = ConnectionPool::new();
        let _guard = pool.acquire_connection().await.unwrap();
        
        let report = pool.health_report();
        assert_eq!(report.active_connections, 1);
        assert!(report.is_healthy());
    }
}
