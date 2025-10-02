//! Metrics handler (Prometheus exposition format)
use crate::{app::AppMetrics, AppState, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::fmt::Write;

/// Write request and connection metrics
fn write_request_metrics(out: &mut String, m: &AppMetrics) {
    writeln!(
        out,
        "# HELP cms_total_requests Total number of HTTP requests handled."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_total_requests counter").unwrap();
    writeln!(out, "cms_total_requests {}", m.total_requests).unwrap();

    writeln!(
        out,
        "# HELP cms_active_connections Active connections (gauge)."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_active_connections gauge").unwrap();
    writeln!(out, "cms_active_connections {}", m.active_connections).unwrap();
}

/// Write cache metrics
fn write_cache_metrics(out: &mut String, m: &AppMetrics) {
    writeln!(
        out,
        "# HELP cms_cache_hits Total cache hits (combined layers)."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_cache_hits counter").unwrap();
    writeln!(out, "cms_cache_hits {}", m.cache_hits).unwrap();

    writeln!(
        out,
        "# HELP cms_cache_misses Total cache misses (combined layers)."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_cache_misses counter").unwrap();
    writeln!(out, "cms_cache_misses {}", m.cache_misses).unwrap();
}

/// Write search metrics
fn write_search_metrics(out: &mut String, m: &AppMetrics) {
    writeln!(
        out,
        "# HELP cms_search_queries Number of search queries executed."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_search_queries counter").unwrap();
    writeln!(out, "cms_search_queries {}", m.search_queries).unwrap();

    writeln!(
        out,
        "# HELP cms_search_avg_response_time_ms Rolling average search response time (ms)."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_search_avg_response_time_ms gauge").unwrap();
    writeln!(
        out,
        "cms_search_avg_response_time_ms {}",
        m.search_avg_response_time_ms
    )
    .unwrap();
}

/// Write authentication metrics
fn write_auth_metrics(out: &mut String, m: &AppMetrics) {
    writeln!(
        out,
        "# HELP cms_auth_attempts Authentication attempts."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_auth_attempts counter").unwrap();
    writeln!(out, "cms_auth_attempts {}", m.auth_attempts).unwrap();

    writeln!(
        out,
        "# HELP cms_auth_successes Successful authentication attempts."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_auth_successes counter").unwrap();
    writeln!(out, "cms_auth_successes {}", m.auth_successes).unwrap();

    writeln!(
        out,
        "# HELP cms_auth_failures Failed authentication attempts."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_auth_failures counter").unwrap();
    writeln!(out, "cms_auth_failures {}", m.auth_failures).unwrap();

    writeln!(
        out,
        "# HELP cms_active_sessions Current active auth sessions."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_active_sessions gauge").unwrap();
    writeln!(out, "cms_active_sessions {}", m.active_sessions).unwrap();
}

/// Write database metrics
fn write_db_metrics(out: &mut String, m: &AppMetrics) {
    writeln!(out, "# HELP cms_db_queries Database queries executed.").unwrap();
    writeln!(out, "# TYPE cms_db_queries counter").unwrap();
    writeln!(out, "cms_db_queries {}", m.db_queries).unwrap();

    writeln!(
        out,
        "# HELP cms_db_avg_response_time_ms Rolling average DB query time (ms)."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_db_avg_response_time_ms gauge").unwrap();
    writeln!(
        out,
        "cms_db_avg_response_time_ms {}",
        m.db_avg_response_time_ms
    )
    .unwrap();
}

/// Write error metrics
fn write_error_metrics(out: &mut String, m: &AppMetrics) {
    writeln!(
        out,
        "# HELP cms_errors_total Total errors encountered."
    )
    .unwrap();
    writeln!(out, "# TYPE cms_errors_total counter").unwrap();
    writeln!(out, "cms_errors_total {}", m.errors_total).unwrap();

    writeln!(out, "# HELP cms_errors_auth Auth related errors.").unwrap();
    writeln!(out, "# TYPE cms_errors_auth counter").unwrap();
    writeln!(out, "cms_errors_auth {}", m.errors_auth).unwrap();

    writeln!(out, "# HELP cms_errors_db DB related errors.").unwrap();
    writeln!(out, "# TYPE cms_errors_db counter").unwrap();
    writeln!(out, "cms_errors_db {}", m.errors_db).unwrap();

    writeln!(out, "# HELP cms_errors_cache Cache related errors.").unwrap();
    writeln!(out, "# TYPE cms_errors_cache counter").unwrap();
    writeln!(out, "cms_errors_cache {}", m.errors_cache).unwrap();

    writeln!(out, "# HELP cms_errors_search Search related errors.").unwrap();
    writeln!(out, "# TYPE cms_errors_search counter").unwrap();
    writeln!(out, "cms_errors_search {}", m.errors_search).unwrap();
}

/// Expose application metrics in Prometheus text format.
/// NOTE: 軽量用途のため現時点ではカウンタ/ゲージのみ。ヒストグラム等が必要になれば `prometheus` crate 統合を検討。
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, description = "Prometheus metrics", content_type = "text/plain")
    )
)]
/// # Errors
///
/// メトリクススナップショットの取得やレスポンス生成中に内部エラーが発生した場合、エラーを返します。
pub async fn metrics(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let m = state.get_metrics().await; // snapshot
    // Basic text format (Prometheus 0.0.4)
    let mut out = String::with_capacity(512);
    
    write_request_metrics(&mut out, &m);
    write_cache_metrics(&mut out, &m);
    write_search_metrics(&mut out, &m);
    write_auth_metrics(&mut out, &m);
    write_db_metrics(&mut out, &m);
    write_error_metrics(&mut out, &m);

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        out,
    ))
}
