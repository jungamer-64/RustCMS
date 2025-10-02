use axum::{Router, routing::get};
use std::net::SocketAddr;

/// Print environment summary and recommended settings for local runs
pub fn print_env_summary() {
    use std::env;

    println!("Environment variables for CMS:");
    println!(
        "DATABASE_URL: {}",
        env::var("DATABASE_URL").unwrap_or_else(|_| "Not set".to_string())
    );
    println!(
        "DATABASE_NAME: {}",
        env::var("DATABASE_NAME").unwrap_or_else(|_| "Not set".to_string())
    );
    println!(
        "SERVER_HOST: {}",
        env::var("SERVER_HOST").unwrap_or_else(|_| "Not set".to_string())
    );
    println!(
        "SERVER_PORT: {}",
        env::var("SERVER_PORT").unwrap_or_else(|_| "Not set".to_string())
    );

    // 推奨設定を表示
    println!("\n推奨設定を表示");
    println!("DATABASE_URL=postgres://user:pass@localhost:5432/rust_cms");
    println!("DATABASE_NAME=rust_cms");
    println!("SERVER_HOST=127.0.0.1");
    println!("SERVER_PORT=3001");
    println!("BISCUIT_ROOT_KEY=base64_or_path_to_keydir");

    println!("\n✅ To run the CMS, use:");
    println!("cargo run --bin cms-server");
}

/// Render a health table from primitive components for CLI output.
/// Public so binaries and tests can reuse the same rendering logic.
pub fn render_health_table_components(
    overall_status: &str,
    db: (&str, f64, Option<&str>),
    cache: (&str, f64, Option<&str>),
    search: (&str, f64, Option<&str>),
    auth: (&str, f64, Option<&str>),
) -> comfy_table::Table {
    use comfy_table::{Cell, Table};

    let mut table = Table::new();
    table.set_header(vec!["Component", "Status", "Response (ms)", "Error"]);

    table.add_row(vec![
        Cell::new("Overall"),
        Cell::new(overall_status.to_string()),
        Cell::new("-"),
        Cell::new("-"),
    ]);

    table.add_row(vec![
        Cell::new("Database"),
        Cell::new(db.0.to_string()),
        Cell::new(format!("{:.2}", db.1)),
        Cell::new(db.2.unwrap_or_default()),
    ]);

    table.add_row(vec![
        Cell::new("Cache"),
        Cell::new(cache.0.to_string()),
        Cell::new(format!("{:.2}", cache.1)),
        Cell::new(cache.2.unwrap_or_default()),
    ]);

    table.add_row(vec![
        Cell::new("Search"),
        Cell::new(search.0.to_string()),
        Cell::new(format!("{:.2}", search.1)),
        Cell::new(search.2.unwrap_or_default()),
    ]);

    table.add_row(vec![
        Cell::new("Auth"),
        Cell::new(auth.0.to_string()),
        Cell::new(format!("{:.2}", auth.1)),
        Cell::new(auth.2.unwrap_or_default()),
    ]);

    table
}

/// Build a minimal docs-only Router that doesn't require `AppState`
fn docs_router() -> Router {
    use crate::handlers;
    Router::new()
        .route("/api/docs", get(handlers::docs_ui))
        .route("/api/docs/openapi.json", get(handlers::openapi_json))
}

/// Run a lightweight docs server on the given address.
///
/// # Errors
///
/// バインド時にソケットの確保に失敗した場合や、サーバー実行中に内部エラーが発生した場合にエラーを返します。
pub async fn run_docs_server(addr: SocketAddr) -> crate::Result<()> {
    let app = docs_router();

    println!("Docs server running on http://{addr} (endpoints: /api/docs, /api/docs/openapi.json)");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::AppError::IO)?;
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::AppError::Internal(format!("axum serve error: {e}")))?;
    Ok(())
}
