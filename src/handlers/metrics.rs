//! Metrics handler (Prometheus exposition format)
use crate::{AppState, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::fmt::Write;

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
    writeln!(
        &mut out,
        "# HELP cms_total_requests Total number of HTTP requests handled."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_total_requests counter").unwrap();
    writeln!(&mut out, "cms_total_requests {}", m.total_requests).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_active_connections Active connections (gauge)."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_active_connections gauge").unwrap();
    writeln!(&mut out, "cms_active_connections {}", m.active_connections).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_cache_hits Total cache hits (combined layers)."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_cache_hits counter").unwrap();
    writeln!(&mut out, "cms_cache_hits {}", m.cache_hits).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_cache_misses Total cache misses (combined layers)."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_cache_misses counter").unwrap();
    writeln!(&mut out, "cms_cache_misses {}", m.cache_misses).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_search_queries Number of search queries executed."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_search_queries counter").unwrap();
    writeln!(&mut out, "cms_search_queries {}", m.search_queries).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_search_avg_response_time_ms Rolling average search response time (ms)."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_search_avg_response_time_ms gauge").unwrap();
    writeln!(
        &mut out,
        "cms_search_avg_response_time_ms {}",
        m.search_avg_response_time_ms
    )
    .unwrap();

    writeln!(
        &mut out,
        "# HELP cms_auth_attempts Authentication attempts."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_auth_attempts counter").unwrap();
    writeln!(&mut out, "cms_auth_attempts {}", m.auth_attempts).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_auth_successes Successful authentication attempts."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_auth_successes counter").unwrap();
    writeln!(&mut out, "cms_auth_successes {}", m.auth_successes).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_auth_failures Failed authentication attempts."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_auth_failures counter").unwrap();
    writeln!(&mut out, "cms_auth_failures {}", m.auth_failures).unwrap();

    writeln!(&mut out, "# HELP cms_db_queries Database queries executed.").unwrap();
    writeln!(&mut out, "# TYPE cms_db_queries counter").unwrap();
    writeln!(&mut out, "cms_db_queries {}", m.db_queries).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_db_avg_response_time_ms Rolling average DB query time (ms)."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_db_avg_response_time_ms gauge").unwrap();
    writeln!(
        &mut out,
        "cms_db_avg_response_time_ms {}",
        m.db_avg_response_time_ms
    )
    .unwrap();

    writeln!(
        &mut out,
        "# HELP cms_errors_total Total errors encountered."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_errors_total counter").unwrap();
    writeln!(&mut out, "cms_errors_total {}", m.errors_total).unwrap();

    writeln!(&mut out, "# HELP cms_errors_auth Auth related errors.").unwrap();
    writeln!(&mut out, "# TYPE cms_errors_auth counter").unwrap();
    writeln!(&mut out, "cms_errors_auth {}", m.errors_auth).unwrap();

    writeln!(&mut out, "# HELP cms_errors_db DB related errors.").unwrap();
    writeln!(&mut out, "# TYPE cms_errors_db counter").unwrap();
    writeln!(&mut out, "cms_errors_db {}", m.errors_db).unwrap();

    writeln!(&mut out, "# HELP cms_errors_cache Cache related errors.").unwrap();
    writeln!(&mut out, "# TYPE cms_errors_cache counter").unwrap();
    writeln!(&mut out, "cms_errors_cache {}", m.errors_cache).unwrap();

    writeln!(&mut out, "# HELP cms_errors_search Search related errors.").unwrap();
    writeln!(&mut out, "# TYPE cms_errors_search counter").unwrap();
    writeln!(&mut out, "cms_errors_search {}", m.errors_search).unwrap();

    writeln!(
        &mut out,
        "# HELP cms_active_sessions Current active auth sessions."
    )
    .unwrap();
    writeln!(&mut out, "# TYPE cms_active_sessions gauge").unwrap();
    writeln!(&mut out, "cms_active_sessions {}", m.active_sessions).unwrap();

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        out,
    ))
}
