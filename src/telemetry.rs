//! テレメトリ初期化（簡易版）
//!
//! 依存関係の整備が完了するまで、ログ出力と環境フィルタに絞った構成です。
//! 将来的にトレーシングのエクスポータやメトリクスHTTPエンドポイントを再導入します。 （詳細は `docs/FEATURES_JA.md` の monitoring セクションを参照）
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use std::fs::OpenOptions; // Add this import for file output

/// テレメトリを初期化します。
///
/// `verbose`が`true`の場合、ログレベルを`debug`に設定します（`RUST_LOG`が未設定の場合）。
/// `LOG_FORMAT`環境変数で`json`または`text`形式を指定できます。
///
/// # Errors
///
/// 初期化過程（環境変数の解析やロガー構築）でエラーが発生した場合にエラーを返します。
pub fn init_telemetry(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let has_env = std::env::var_os("RUST_LOG").is_some();
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());

    // Determine default log filter string based on verbose flag and APP_ENV
    let default_filter_str = if verbose || app_env.eq_ignore_ascii_case("development") {
        // More verbose for development or when --verbose flag is used
        "debug,cms_backend=debug,sqlx=warn,tower=info,hyper=info"
    } else {
        // Default for production or when not verbose
        "info,cms_backend=info,tower=warn,sqlx=warn"
    };

    let env_filter = if has_env {
        // RUST_LOG is explicitly set, use it.
        EnvFilter::try_from_default_env()?
    } else {
        // RUST_LOG is not set, use the determined default.
        EnvFilter::new(default_filter_str)
    };

    // Determine default log format based on APP_ENV
    let default_format = if app_env.eq_ignore_ascii_case("development") { "text" } else { "json" };
    let format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| default_format.to_string());
    let is_json = format.eq_ignore_ascii_case("json"); // This line was duplicated, fixed below

    let log_output = std::env::var("LOG_OUTPUT").unwrap_or_else(|_| "stdout".to_string());

    // Build the common base layer config
    let base_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE);

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if log_output.eq_ignore_ascii_case("stdout") || log_output.eq_ignore_ascii_case("stderr") {
        // Default behavior: use the default writer (stdout/stderr depending on level)
        let layer = base_layer;
        if is_json {
            subscriber.with(layer.json()).try_init()?;
        } else {
            subscriber.with(layer.pretty()).try_init()?;
        }
    } else {
        // Assume it's a file path; create/open the file and provide a closure that clones it for each writer
        let file = OpenOptions::new().append(true).create(true).open(&log_output)?;
        // `with_writer` expects a factory/closure that returns a writer. We clone the file handle per call.
        let layer = base_layer.with_writer(move || file.try_clone().expect("failed to clone log file handle")).with_ansi(false);
        if is_json {
            subscriber.with(layer.json()).try_init()?;
        } else {
            subscriber.with(layer.pretty()).try_init()?;
        }
    }

    // Initialize Prometheus metrics registry
    #[cfg(feature = "monitoring")]
    init_metrics();

    Ok(())
}

/// Initialize telemetry with optional verbose mode.
///
/// # Deprecated
/// Use `init_telemetry(verbose)` instead.
#[deprecated(since = "2.1.0", note = "Please use `init_telemetry(verbose)` instead")]
pub fn init_telemetry_with_verbose(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    init_telemetry(verbose)
}

#[cfg(feature = "monitoring")]
fn init_metrics() {
    // For now, keep metrics initialization minimal to avoid hard dependency on
    // optional monitoring crates during consolidation. This is a safe noop that
    // preserves the feature gate and can be expanded later to initialize a
    // Prometheus recorder + HTTP endpoint when we wire the optional crates.
}

#[cfg(feature = "monitoring")]
/// Gracefully shutdown telemetry systems
pub fn shutdown_telemetry() {
    // No-op: opentelemetry global shutdown API surface changed between
    // versions; avoid calling unavailable helper to remain compatible.
    // If explicit tracer shutdown is required, expand here with the
    // appropriate opentelemetry SDK calls.
}