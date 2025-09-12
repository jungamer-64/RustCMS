// telemetry temporarily minimized; re-expand later
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize comprehensive telemetry for enterprise monitoring
///
/// # Errors
///
/// 初期化過程（環境変数の解析やロガー構築）でエラーが発生した場合にエラーを返します。
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry tracer for distributed tracing
    // (Temporarily disabled Jaeger pipeline — reintroduce after dependency stabilization)

    // Environment filter for log levels
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,cms_backend=debug,axum=debug,tower=debug"));

    // Initialize tracing subscriber with multiple layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json(), // Structured logging for production
        )
        // telemetry exporter layer temporarily removed
        .init();

    // Initialize Prometheus metrics registry
    #[cfg(feature = "monitoring")]
    init_metrics();

    Ok(())
}

#[cfg(feature = "monitoring")]
const fn init_metrics() {
    // For now, keep metrics initialization minimal to avoid hard dependency on
    // optional monitoring crates during consolidation. This is a safe noop that
    // preserves the feature gate and can be expanded later to initialize a
    // Prometheus recorder + HTTP endpoint when we wire the optional crates.
}

#[cfg(feature = "monitoring")]
/// Gracefully shutdown telemetry systems
pub const fn shutdown_telemetry() {
    // No-op: opentelemetry global shutdown API surface changed between
    // versions; avoid calling unavailable helper to remain compatible.
    // If explicit tracer shutdown is required, expand here with the
    // appropriate opentelemetry SDK calls.
}
