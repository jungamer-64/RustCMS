// telemetry temporarily minimized; re-expand later
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
#[cfg(feature = "monitoring")]
use opentelemetry::global;

/// Initialize comprehensive telemetry for enterprise monitoring
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
    init_metrics()?;

    Ok(())
}

#[cfg(feature = "monitoring")]
fn init_metrics() -> Result<(), Box<dyn std::error::Error>> {
    // For now, keep metrics initialization minimal to avoid hard dependency on
    // optional monitoring crates during consolidation. This is a safe noop that
    // preserves the feature gate and can be expanded later to initialize a
    // Prometheus recorder + HTTP endpoint when we wire the optional crates.
    Ok(())
}

#[cfg(feature = "monitoring")]
/// Gracefully shutdown telemetry systems
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
