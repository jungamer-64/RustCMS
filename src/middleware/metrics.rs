use axum::{http::Request, middleware::Next, response::Response, body::Body};
use std::time::Instant;
use crate::AppState;

/// Metrics middleware: increments request count and records latency distribution (future-ready).
pub async fn metrics_middleware(state: AppState, req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    state.record_request().await;
    let resp = next.run(req).await;
    let _ = start.elapsed();
    resp
}
