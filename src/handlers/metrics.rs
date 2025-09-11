//! Metrics handler (Prometheus exposition format)
use crate::{AppState, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse};

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
pub async fn metrics(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let m = state.get_metrics().await; // snapshot
    // Basic text format (Prometheus 0.0.4)
    let mut out = String::with_capacity(512);
    out.push_str("# HELP cms_total_requests Total number of HTTP requests handled.\n");
    out.push_str("# TYPE cms_total_requests counter\n");
    out.push_str(&format!("cms_total_requests {}\n", m.total_requests));

    out.push_str("# HELP cms_active_connections Active connections (gauge).\n");
    out.push_str("# TYPE cms_active_connections gauge\n");
    out.push_str(&format!("cms_active_connections {}\n", m.active_connections));

    out.push_str("# HELP cms_cache_hits Total cache hits (combined layers).\n");
    out.push_str("# TYPE cms_cache_hits counter\n");
    out.push_str(&format!("cms_cache_hits {}\n", m.cache_hits));

    out.push_str("# HELP cms_cache_misses Total cache misses (combined layers).\n");
    out.push_str("# TYPE cms_cache_misses counter\n");
    out.push_str(&format!("cms_cache_misses {}\n", m.cache_misses));

    out.push_str("# HELP cms_search_queries Number of search queries executed.\n");
    out.push_str("# TYPE cms_search_queries counter\n");
    out.push_str(&format!("cms_search_queries {}\n", m.search_queries));

    out.push_str(
        "# HELP cms_search_avg_response_time_ms Rolling average search response time (ms).\n",
    );
    out.push_str("# TYPE cms_search_avg_response_time_ms gauge\n");
    out.push_str(&format!("cms_search_avg_response_time_ms {}\n", m.search_avg_response_time_ms));

    out.push_str("# HELP cms_auth_attempts Authentication attempts.\n");
    out.push_str("# TYPE cms_auth_attempts counter\n");
    out.push_str(&format!("cms_auth_attempts {}\n", m.auth_attempts));

    out.push_str("# HELP cms_auth_successes Successful authentication attempts.\n");
    out.push_str("# TYPE cms_auth_successes counter\n");
    out.push_str(&format!("cms_auth_successes {}\n", m.auth_successes));

    out.push_str("# HELP cms_auth_failures Failed authentication attempts.\n");
    out.push_str("# TYPE cms_auth_failures counter\n");
    out.push_str(&format!("cms_auth_failures {}\n", m.auth_failures));

    out.push_str("# HELP cms_db_queries Database queries executed.\n");
    out.push_str("# TYPE cms_db_queries counter\n");
    out.push_str(&format!("cms_db_queries {}\n", m.db_queries));

    out.push_str("# HELP cms_db_avg_response_time_ms Rolling average DB query time (ms).\n");
    out.push_str("# TYPE cms_db_avg_response_time_ms gauge\n");
    out.push_str(&format!("cms_db_avg_response_time_ms {}\n", m.db_avg_response_time_ms));

    out.push_str("# HELP cms_errors_total Total errors encountered.\n");
    out.push_str("# TYPE cms_errors_total counter\n");
    out.push_str(&format!("cms_errors_total {}\n", m.errors_total));

    out.push_str("# HELP cms_errors_auth Auth related errors.\n");
    out.push_str("# TYPE cms_errors_auth counter\n");
    out.push_str(&format!("cms_errors_auth {}\n", m.errors_auth));

    out.push_str("# HELP cms_errors_db DB related errors.\n");
    out.push_str("# TYPE cms_errors_db counter\n");
    out.push_str(&format!("cms_errors_db {}\n", m.errors_db));

    out.push_str("# HELP cms_errors_cache Cache related errors.\n");
    out.push_str("# TYPE cms_errors_cache counter\n");
    out.push_str(&format!("cms_errors_cache {}\n", m.errors_cache));

    out.push_str("# HELP cms_errors_search Search related errors.\n");
    out.push_str("# TYPE cms_errors_search counter\n");
    out.push_str(&format!("cms_errors_search {}\n", m.errors_search));

    out.push_str("# HELP cms_active_sessions Current active auth sessions.\n");
    out.push_str("# TYPE cms_active_sessions gauge\n");
    out.push_str(&format!("cms_active_sessions {}\n", m.active_sessions));

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        out,
    ))
}
