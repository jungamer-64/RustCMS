use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::{
    global,
    sdk::{trace::TracerProvider, Resource},
    KeyValue,
};
use opentelemetry_jaeger::new_agent_pipeline;
use std::time::Duration;

/// Initialize comprehensive telemetry for enterprise monitoring
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry tracer for distributed tracing
    let tracer = new_agent_pipeline()
        .with_service_name("enterprise-cms")
        .with_service_name(&format!("enterprise-cms-{}", env!("CARGO_PKG_VERSION")))
        .with_auto_split_batch(true)
        .with_max_packet_size(9216)
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Create OpenTelemetry layer
    let opentelemetry_layer = OpenTelemetryLayer::new(tracer);

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
                .json()  // Structured logging for production
        )
        .with(opentelemetry_layer)
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
