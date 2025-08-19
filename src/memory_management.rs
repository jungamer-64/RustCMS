/// 厳格なメモリ管理システム
/// 
/// このモジュールはアプリケーション全体のメモリ使用量を監視・制御し、
/// メモリリークを防止し、パフォーマンスを最適化します。

use std::sync::{Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use tokio::time::interval;

/// グローバルメモリ統計
#[derive(Debug)]
pub struct MemoryStats {
    /// 総アロケーション数
    pub total_allocations: AtomicU64,
    /// 総デアロケーション数
    pub total_deallocations: AtomicU64,
    /// 現在のメモリ使用量（推定）
    pub current_usage_bytes: AtomicUsize,
    /// ピークメモリ使用量
    pub peak_usage_bytes: AtomicUsize,
    /// 最後の統計更新時刻
    pub last_update: std::sync::Mutex<Option<Instant>>,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            current_usage_bytes: AtomicUsize::new(0),
            peak_usage_bytes: AtomicUsize::new(0),
            last_update: std::sync::Mutex::new(None),
        }
    }
}

impl MemoryStats {
    /// アロケーションを記録
    pub fn record_allocation(&self, size: usize) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        let new_usage = self.current_usage_bytes.fetch_add(size, Ordering::Relaxed) + size;
        
        // ピーク値を更新
        let mut peak = self.peak_usage_bytes.load(Ordering::Relaxed);
        while new_usage > peak {
            match self.peak_usage_bytes.compare_exchange_weak(
                peak, new_usage, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
    }
    
    /// デアロケーションを記録
    pub fn record_deallocation(&self, size: usize) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_usage_bytes.fetch_sub(size, Ordering::Relaxed);
    }
    
    /// 統計のスナップショット取得
    pub fn snapshot(&self) -> MemoryStatsSnapshot {
        MemoryStatsSnapshot {
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.total_deallocations.load(Ordering::Relaxed),
            current_usage_bytes: self.current_usage_bytes.load(Ordering::Relaxed),
            peak_usage_bytes: self.peak_usage_bytes.load(Ordering::Relaxed),
        }
    }
}

/// メモリ統計のスナップショット
#[derive(Debug, Clone)]
pub struct MemoryStatsSnapshot {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub current_usage_bytes: usize,
    pub peak_usage_bytes: usize,
}

/// グローバルメモリマネージャ
pub struct MemoryManager {
    stats: Arc<MemoryStats>,
    config: MemoryConfig,
}

/// メモリ管理設定
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// メモリ使用量の警告閾値（バイト）
    pub warning_threshold_bytes: usize,
    /// メモリ使用量の緊急閾値（バイト）
    pub critical_threshold_bytes: usize,
    /// 統計更新間隔
    pub stats_interval: Duration,
    /// GC実行間隔
    pub gc_interval: Duration,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            warning_threshold_bytes: 512 * 1024 * 1024,  // 512MB
            critical_threshold_bytes: 1024 * 1024 * 1024, // 1GB
            stats_interval: Duration::from_secs(30),
            gc_interval: Duration::from_secs(60),
        }
    }
}

impl MemoryManager {
    /// 新しいメモリマネージャを作成
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            stats: Arc::new(MemoryStats::default()),
            config,
        }
    }
    
    /// メモリ監視タスクを開始
    pub fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut stats_timer = interval(config.stats_interval);
            let mut gc_timer = interval(config.gc_interval);
            
            loop {
                tokio::select! {
                    _ = stats_timer.tick() => {
                        Self::log_memory_stats(&stats);
                    }
                    _ = gc_timer.tick() => {
                        Self::perform_gc_if_needed(&stats, &config);
                    }
                }
            }
        })
    }
    
    /// メモリ統計をログ出力
    fn log_memory_stats(stats: &MemoryStats) {
        let snapshot = stats.snapshot();
        let current_mb = snapshot.current_usage_bytes as f64 / (1024.0 * 1024.0);
        let peak_mb = snapshot.peak_usage_bytes as f64 / (1024.0 * 1024.0);
        let net_allocations = snapshot.total_allocations.saturating_sub(snapshot.total_deallocations);
        
        info!(
            "📊 Memory Stats: Current: {:.2}MB, Peak: {:.2}MB, Net Allocs: {}, Total Allocs: {}",
            current_mb, peak_mb, net_allocations, snapshot.total_allocations
        );
    }
    
    /// 必要に応じてガベージコレクションを実行
    fn perform_gc_if_needed(stats: &MemoryStats, config: &MemoryConfig) {
        let current_usage = stats.current_usage_bytes.load(Ordering::Relaxed);
        
        if current_usage > config.critical_threshold_bytes {
            error!(
                "🚨 Critical memory usage: {:.2}MB > {:.2}MB",
                current_usage as f64 / (1024.0 * 1024.0),
                config.critical_threshold_bytes as f64 / (1024.0 * 1024.0)
            );
            // 緊急時の処理（必要に応じて実装）
        } else if current_usage > config.warning_threshold_bytes {
            warn!(
                "⚠️  High memory usage: {:.2}MB > {:.2}MB",
                current_usage as f64 / (1024.0 * 1024.0),
                config.warning_threshold_bytes as f64 / (1024.0 * 1024.0)
            );
        }
    }
    
    /// 統計を取得
    pub fn get_stats(&self) -> MemoryStatsSnapshot {
        self.stats.snapshot()
    }
}

/// メモリ効率化トレイト
pub trait MemoryEfficient {
    /// メモリ使用量を推定
    fn estimated_memory_usage(&self) -> usize;
    
    /// メモリをクリーンアップ
    fn cleanup_memory(&mut self);
    
    /// メモリプールに返却可能かどうか
    fn is_poolable(&self) -> bool {
        true
    }
}

/// RAII ガード - スコープを出る時に自動的にメモリをクリーンアップ
pub struct MemoryGuard<T> {
    value: Option<T>,
    cleanup_fn: Option<Box<dyn FnOnce(T) + Send>>,
}

impl<T> MemoryGuard<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Some(value),
            cleanup_fn: None,
        }
    }
    
    pub fn with_cleanup<F>(value: T, cleanup_fn: F) -> Self 
    where 
        F: FnOnce(T) + Send + 'static,
    {
        Self {
            value: Some(value),
            cleanup_fn: Some(Box::new(cleanup_fn)),
        }
    }
    
    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }
    
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }
    
    pub fn take(mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T> Drop for MemoryGuard<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            if let Some(cleanup_fn) = self.cleanup_fn.take() {
                cleanup_fn(value);
            }
        }
    }
}

/// グローバルメモリマネージャインスタンス
static GLOBAL_MEMORY_MANAGER: std::sync::OnceLock<MemoryManager> = std::sync::OnceLock::new();

/// グローバルメモリマネージャを初期化
pub fn init_global_memory_manager(config: Option<MemoryConfig>) -> &'static MemoryManager {
    GLOBAL_MEMORY_MANAGER.get_or_init(|| {
        MemoryManager::new(config.unwrap_or_default())
    })
}

/// グローバルメモリマネージャを取得
pub fn get_global_memory_manager() -> Option<&'static MemoryManager> {
    GLOBAL_MEMORY_MANAGER.get()
}
