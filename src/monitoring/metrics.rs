use metrics::{counter, gauge, histogram};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct PerformanceMonitor;

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn record_request(&self, method: &str, path: &str, status: u16, duration: Duration) {
        let labels = [
            ("method", method.to_string()),
            ("path", sanitize_path(path)),
            ("status", status.to_string()),
        ];

        counter!("http_requests_total").increment(1);
        histogram!("http_request_duration_seconds", &labels).record(duration.as_secs_f64());

        // Record status code specific metrics
        match status {
            200..=299 => counter!("http_requests_success_total").increment(1),
            400..=499 => counter!("http_requests_client_error_total").increment(1),
            500..=599 => counter!("http_requests_server_error_total").increment(1),
            _ => {}
        }
    }

    pub fn record_database_query(&self, operation: &str, table: &str, duration: Duration) {
        let labels = [
            ("operation", operation.to_string()),
            ("table", table.to_string()),
        ];

        counter!("database_queries_total", &labels).increment(1);
        histogram!("database_query_duration_seconds", &labels).record(duration.as_secs_f64());
    }

    pub fn record_cache_operation(&self, operation: &str, hit: bool, duration: Duration) {
        let labels = [
            ("operation", operation.to_string()),
            ("result", if hit { "hit" } else { "miss" }.to_string()),
        ];

        counter!("cache_operations_total", &labels).increment(1);
        histogram!("cache_operation_duration_seconds", &labels).record(duration.as_secs_f64());
    }

    pub fn record_active_connections(&self, count: usize) {
        gauge!("active_connections").set(count as f64);
    }

    pub fn record_memory_usage(&self, bytes: u64) {
        gauge!("memory_usage_bytes").set(bytes as f64);
    }

    pub fn increment_rate_limit_violations(&self, client_ip: &str) {
        let labels = [("client_ip", client_ip.to_string())];
        counter!("rate_limit_violations_total", &labels).increment(1);
    }
}

pub struct Timer {
    start: Instant,
    name: String,
    labels: Vec<(String, String)>,
}

impl Timer {
    pub fn new(name: String, labels: Vec<(String, String)>) -> Self {
        Self {
            start: Instant::now(),
            name,
            labels,
        }
    }

    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        let label_refs: Vec<(&str, &str)> = self
            .labels
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        histogram!(&self.name, &label_refs).record(duration.as_secs_f64());
        duration
    }
}

pub fn start_timer(name: &str, labels: Vec<(String, String)>) -> Timer {
    Timer::new(name.to_string(), labels)
}

#[derive(Serialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f32,
    pub active_connections: usize,
    pub total_requests: u64,
    pub error_rate: f32,
    pub average_response_time_ms: f32,
}

impl SystemMetrics {
    pub async fn collect() -> Self {
        // In a real implementation, you would collect these metrics
        // from your monitoring system or system APIs
        Self {
            uptime_seconds: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            active_connections: 0,
            total_requests: 0,
            error_rate: 0.0,
            average_response_time_ms: 0.0,
        }
    }
}

fn sanitize_path(path: &str) -> String {
    // Replace dynamic path segments with placeholders for better metric grouping
    let path = path.to_string();

    // Replace UUIDs
    let uuid_regex =
        regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    let path = uuid_regex.replace_all(&path, ":id");

    // Replace numeric IDs
    let id_regex = regex::Regex::new(r"/\d+").unwrap();
    let path = id_regex.replace_all(&path, "/:id");

    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path() {
        assert_eq!(sanitize_path("/api/posts/123"), "/api/posts/:id");
        assert_eq!(
            sanitize_path("/api/posts/550e8400-e29b-41d4-a716-446655440000"),
            "/api/posts/:id"
        );
        assert_eq!(sanitize_path("/api/posts"), "/api/posts");
    }
}
