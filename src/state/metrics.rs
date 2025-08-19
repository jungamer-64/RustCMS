//! 高性能メトリクス収集・分析システム
//! 
//! - リアルタイムパフォーマンス監視
//! - ヒストグラム統計
//! - 適応的しきい値調整

use std::time::{Duration, Instant};
use parking_lot::RwLock;
use dashmap::DashMap;
use std::collections::VecDeque;
use super::FastHasher;

/// パフォーマンスメトリクス
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// リクエスト統計
    request_stats: RwLock<RequestStats>,
    
    /// レスポンス時間ヒストグラム
    response_times: RwLock<ResponseTimeHistogram>,
    
    /// エンドポイント別統計
    endpoint_stats: DashMap<String, EndpointMetrics, FastHasher>,
    
    /// システムリソース統計
    system_stats: RwLock<SystemStats>,
    
    /// アラート設定
    alert_thresholds: RwLock<AlertThresholds>,
    
    /// 認証メトリクス
    pub auth_metrics: super::auth_metrics::AuthMetrics,
}

/// リクエスト統計
#[derive(Debug, Clone)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub requests_per_second: f64,
    pub last_reset: Instant,
}

impl Default for RequestStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            requests_per_second: 0.0,
            last_reset: Instant::now(),
        }
    }
}

/// レスポンス時間ヒストグラム
#[derive(Debug)]
pub struct ResponseTimeHistogram {
    buckets: Vec<HistogramBucket>,
    total_samples: u64,
    sum: Duration,
}

#[derive(Debug, Clone)]
struct HistogramBucket {
    upper_bound: Duration,
    count: u64,
}

impl Default for ResponseTimeHistogram {
    fn default() -> Self {
        Self {
            buckets: vec![
                HistogramBucket { upper_bound: Duration::from_millis(1), count: 0 },
                HistogramBucket { upper_bound: Duration::from_millis(5), count: 0 },
                HistogramBucket { upper_bound: Duration::from_millis(10), count: 0 },
                HistogramBucket { upper_bound: Duration::from_millis(50), count: 0 },
                HistogramBucket { upper_bound: Duration::from_millis(100), count: 0 },
                HistogramBucket { upper_bound: Duration::from_millis(500), count: 0 },
                HistogramBucket { upper_bound: Duration::from_secs(1), count: 0 },
                HistogramBucket { upper_bound: Duration::from_secs(5), count: 0 },
                HistogramBucket { upper_bound: Duration::MAX, count: 0 },
            ],
            total_samples: 0,
            sum: Duration::ZERO,
        }
    }
}

impl ResponseTimeHistogram {
    fn observe(&mut self, duration: Duration) {
        self.total_samples += 1;
        self.sum += duration;
        
        for bucket in &mut self.buckets {
            if duration <= bucket.upper_bound {
                bucket.count += 1;
            }
        }
    }

    fn percentile(&self, p: f64) -> Duration {
        let target_count = (self.total_samples as f64 * p / 100.0) as u64;
        let mut cumulative = 0;
        
        for bucket in &self.buckets {
            cumulative += bucket.count;
            if cumulative >= target_count {
                return bucket.upper_bound;
            }
        }
        
        Duration::ZERO
    }

    fn average(&self) -> Duration {
        if self.total_samples > 0 {
            self.sum / self.total_samples as u32
        } else {
            Duration::ZERO
        }
    }
}

/// エンドポイント別メトリクス
#[derive(Debug)]
pub struct EndpointMetrics {
    pub request_count: u64,
    pub error_count: u64,
    pub response_times: VecDeque<Duration>,
    pub last_accessed: Instant,
    pub max_response_time: Duration,
    pub min_response_time: Duration,
}

impl Default for EndpointMetrics {
    fn default() -> Self {
        Self {
            request_count: 0,
            error_count: 0,
            response_times: VecDeque::new(),
            last_accessed: Instant::now(),
            max_response_time: Duration::ZERO,
            min_response_time: Duration::MAX,
        }
    }
}

const MAX_RESPONSE_TIME_SAMPLES: usize = 1000;

impl EndpointMetrics {
    pub fn new() -> Self {
        Self {
            request_count: 0,
            error_count: 0,
            response_times: VecDeque::with_capacity(MAX_RESPONSE_TIME_SAMPLES),
            last_accessed: Instant::now(),
            max_response_time: Duration::ZERO,
            min_response_time: Duration::MAX,
        }
    }

    pub fn record_request(&mut self, duration: Duration, is_error: bool) {
        self.request_count += 1;
        if is_error {
            self.error_count += 1;
        }
        
        self.last_accessed = Instant::now();
        
        // レスポンス時間統計
        if duration > self.max_response_time {
            self.max_response_time = duration;
        }
        if duration < self.min_response_time {
            self.min_response_time = duration;
        }
        
        // サンプル数制限
        if self.response_times.len() >= MAX_RESPONSE_TIME_SAMPLES {
            self.response_times.pop_front();
        }
        self.response_times.push_back(duration);
    }

    pub fn error_rate(&self) -> f64 {
        if self.request_count > 0 {
            self.error_count as f64 / self.request_count as f64
        } else {
            0.0
        }
    }

    pub fn average_response_time(&self) -> Duration {
        if self.response_times.is_empty() {
            Duration::ZERO
        } else {
            let sum: Duration = self.response_times.iter().sum();
            sum / self.response_times.len() as u32
        }
    }
}

/// システム統計
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub active_connections: u32,
    pub database_connections: u32,
    pub cache_hit_rate: f64,
    pub last_updated: Instant,
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            memory_usage: 0,
            cpu_usage: 0.0,
            active_connections: 0,
            database_connections: 0,
            cache_hit_rate: 0.0,
            last_updated: Instant::now(),
        }
    }
}

/// アラートしきい値
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub max_response_time: Duration,
    pub max_error_rate: f64,
    pub max_memory_usage: u64,
    pub max_cpu_usage: f64,
    pub min_cache_hit_rate: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_response_time: Duration::from_secs(5),
            max_error_rate: 0.05, // 5%
            max_memory_usage: 1024 * 1024 * 1024, // 1GB
            max_cpu_usage: 80.0, // 80%
            min_cache_hit_rate: 0.8, // 80%
        }
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            request_stats: RwLock::new(RequestStats {
                last_reset: Instant::now(),
                ..Default::default()
            }),
            response_times: RwLock::new(ResponseTimeHistogram::default()),
            endpoint_stats: DashMap::with_hasher(FastHasher::default()),
            system_stats: RwLock::new(SystemStats {
                last_updated: Instant::now(),
                ..Default::default()
            }),
            alert_thresholds: RwLock::new(AlertThresholds::default()),
            auth_metrics: Default::default(),
        }
    }

    /// リクエスト記録
    pub fn record_request(&self, endpoint: &str, duration: Duration, is_error: bool) {
        // 全体統計更新
        {
            let mut stats = self.request_stats.write();
            stats.total_requests += 1;
            if is_error {
                stats.failed_requests += 1;
            } else {
                stats.successful_requests += 1;
            }
            
            // RPS計算（1分間のスライディングウィンドウ）
            let elapsed = stats.last_reset.elapsed();
            if elapsed >= Duration::from_secs(60) {
                stats.requests_per_second = stats.total_requests as f64 / elapsed.as_secs_f64();
                stats.last_reset = Instant::now();
                stats.total_requests = 0;
                stats.successful_requests = 0;
                stats.failed_requests = 0;
            }
        }

        // レスポンス時間ヒストグラム更新
        {
            let mut histogram = self.response_times.write();
            histogram.observe(duration);
        }

        // エンドポイント別統計更新
        self.endpoint_stats
            .entry(endpoint.to_string())
            .or_insert_with(EndpointMetrics::new)
            .record_request(duration, is_error);
    }

    /// システム統計更新
    pub fn update_system_stats(&self, stats: SystemStats) {
        *self.system_stats.write() = stats;
    }

    /// 全体統計取得
    pub fn get_overall_stats(&self) -> OverallStats {
        let request_stats = self.request_stats.read().clone();
        let response_times = self.response_times.read();
        let system_stats = self.system_stats.read().clone();

        OverallStats {
            request_stats,
            avg_response_time: response_times.average(),
            p50_response_time: response_times.percentile(50.0),
            p95_response_time: response_times.percentile(95.0),
            p99_response_time: response_times.percentile(99.0),
            system_stats,
        }
    }

    /// エンドポイント統計取得
    pub fn get_endpoint_stats(&self, endpoint: &str) -> Option<EndpointMetrics> {
        self.endpoint_stats.get(endpoint).map(|entry| {
            let mut metrics = EndpointMetrics::new();
            let original = entry.value();
            metrics.request_count = original.request_count;
            metrics.error_count = original.error_count;
            metrics.last_accessed = original.last_accessed;
            metrics.max_response_time = original.max_response_time;
            metrics.min_response_time = original.min_response_time;
            metrics.response_times = original.response_times.clone();
            metrics
        })
    }

    /// 全エンドポイント統計取得
    pub fn get_all_endpoint_stats(&self) -> Vec<(String, EndpointSummary)> {
        self.endpoint_stats
            .iter()
            .map(|entry| {
                let endpoint = entry.key().clone();
                let metrics = entry.value();
                let summary = EndpointSummary {
                    request_count: metrics.request_count,
                    error_rate: metrics.error_rate(),
                    avg_response_time: metrics.average_response_time(),
                    max_response_time: metrics.max_response_time,
                    min_response_time: if metrics.min_response_time == Duration::MAX {
                        Duration::ZERO
                    } else {
                        metrics.min_response_time
                    },
                    last_accessed: metrics.last_accessed,
                };
                (endpoint, summary)
            })
            .collect()
    }

    /// アラート検知
    pub fn check_alerts(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let thresholds = self.alert_thresholds.read();
        let overall_stats = self.get_overall_stats();

        // レスポンス時間アラート
        if overall_stats.p95_response_time > thresholds.max_response_time {
            alerts.push(Alert {
                alert_type: AlertType::HighResponseTime,
                message: format!(
                    "P95 response time ({:.2}ms) exceeds threshold ({:.2}ms)",
                    overall_stats.p95_response_time.as_millis(),
                    thresholds.max_response_time.as_millis()
                ),
                severity: AlertSeverity::Warning,
                timestamp: Instant::now(),
            });
        }

        // エラー率アラート
        let error_rate = if overall_stats.request_stats.total_requests > 0 {
            overall_stats.request_stats.failed_requests as f64 / overall_stats.request_stats.total_requests as f64
        } else {
            0.0
        };

        if error_rate > thresholds.max_error_rate {
            alerts.push(Alert {
                alert_type: AlertType::HighErrorRate,
                message: format!(
                    "Error rate ({:.2}%) exceeds threshold ({:.2}%)",
                    error_rate * 100.0,
                    thresholds.max_error_rate * 100.0
                ),
                severity: AlertSeverity::Critical,
                timestamp: Instant::now(),
            });
        }

        // システムリソースアラート
        if overall_stats.system_stats.memory_usage > thresholds.max_memory_usage {
            alerts.push(Alert {
                alert_type: AlertType::HighMemoryUsage,
                message: format!(
                    "Memory usage ({} bytes) exceeds threshold ({} bytes)",
                    overall_stats.system_stats.memory_usage,
                    thresholds.max_memory_usage
                ),
                severity: AlertSeverity::Warning,
                timestamp: Instant::now(),
            });
        }

        alerts
    }

    /// しきい値更新
    pub fn update_thresholds(&self, thresholds: AlertThresholds) {
        *self.alert_thresholds.write() = thresholds;
    }
}

/// 統計サマリー
#[derive(Debug, Clone)]
pub struct OverallStats {
    pub request_stats: RequestStats,
    pub avg_response_time: Duration,
    pub p50_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub system_stats: SystemStats,
}

#[derive(Debug, Clone)]
pub struct EndpointSummary {
    pub request_count: u64,
    pub error_rate: f64,
    pub avg_response_time: Duration,
    pub max_response_time: Duration,
    pub min_response_time: Duration,
    pub last_accessed: Instant,
}

/// アラート
#[derive(Debug, Clone)]
pub struct Alert {
    pub alert_type: AlertType,
    pub message: String,
    pub severity: AlertSeverity,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum AlertType {
    HighResponseTime,
    HighErrorRate,
    HighMemoryUsage,
    HighCpuUsage,
    LowCacheHitRate,
    DatabaseConnectionIssue,
}

#[derive(Debug, Clone)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_time_histogram() {
        let mut histogram = ResponseTimeHistogram::default();
        
        // サンプルデータ追加
        histogram.observe(Duration::from_millis(10));
        histogram.observe(Duration::from_millis(50));
        histogram.observe(Duration::from_millis(100));
        
        assert_eq!(histogram.total_samples, 3);
        assert!(histogram.average() > Duration::ZERO);
        assert!(histogram.percentile(50.0) >= Duration::from_millis(10));
    }

    #[test]
    fn test_endpoint_metrics() {
        let mut metrics = EndpointMetrics::new();
        
        metrics.record_request(Duration::from_millis(100), false);
        metrics.record_request(Duration::from_millis(200), true);
        
        assert_eq!(metrics.request_count, 2);
        assert_eq!(metrics.error_count, 1);
        assert_eq!(metrics.error_rate(), 0.5);
        assert_eq!(metrics.max_response_time, Duration::from_millis(200));
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        
        metrics.record_request("/api/test", Duration::from_millis(100), false);
        metrics.record_request("/api/test", Duration::from_millis(200), true);
        
        let endpoint_stats = metrics.get_endpoint_stats("/api/test").unwrap();
        assert_eq!(endpoint_stats.request_count, 2);
        assert_eq!(endpoint_stats.error_count, 1);
        
        let overall_stats = metrics.get_overall_stats();
        assert_eq!(overall_stats.request_stats.total_requests, 2);
        assert_eq!(overall_stats.request_stats.failed_requests, 1);
    }
}
