//! Rate Limiting Middleware (Phase 5.2 - 新AppState対応版)
//!
//! This middleware provides IP-based rate limiting using the unified AppState structure.
//! All configuration flows from `Config.security.*` via `AppState`.
//! Behaviour: if limit exceeded -> 429 + Retry-After (window seconds).
//!
//! Phase 5.2 での変更点:
//! - 新しい `Arc<AppState>` 構造に対応
//! - キャッシュベースのレート制限 (Phase 5.3 で完全実装)

use crate::web::middleware::common::{BoxServiceFuture, forward_poll_ready};
use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::net::IpAddr;
use tower::{Layer, Service};
use tracing::warn;

use crate::infrastructure::app_state::AppState;
#[cfg(feature = "monitoring")]
use metrics::{counter, gauge};
use std::sync::Arc;

#[derive(Clone)]
pub struct RateLimitLayer;

impl RateLimitLayer {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for RateLimitLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimitService { service }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    service: S,
}

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        forward_poll_ready(&mut self.service, cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let mut service = self.service.clone();

        // Extract IP early
        let ip = extract_ip_from_request(&request).unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

        // Pull state extension
        let state_opt = request.extensions().get::<Arc<AppState>>().cloned();

        // TODO Phase 5.3: キャッシュベースのレート制限を実装
        // let allowed = state_opt.as_ref().map_or(true, |s| s.check_rate_limit(&ip));

        // 暫定: 常に許可 (Phase 5.3 で実装)
        let allowed = true;

        if !allowed {
            warn!("Rate limit would block IP: {}", ip);
        }

        #[cfg(feature = "monitoring")]
        {
            if allowed {
                counter!("ip_rate_limit_allowed_total").increment(1);
            } else {
                counter!("ip_rate_limit_blocked_total").increment(1);
            }
        }

        if !allowed {
            let retry_after = 60; // TODO Phase 5.3: 設定から取得
            let response = (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_after.to_string())],
                "Rate limit exceeded. Please try again later.",
            )
                .into_response();
            return Box::pin(async move { Ok(response) });
        }

        Box::pin(async move { service.call(request).await })
    }
}

#[inline]
#[allow(clippy::cast_precision_loss)]
#[cfg(feature = "monitoring")]
const fn usize_to_f64(n: usize) -> f64 {
    n as f64
}

fn extract_ip_from_request<B>(request: &Request<B>) -> Option<IpAddr> {
    if let Some(forwarded) = request.headers().get("X-Forwarded-For")
        && let Ok(forwarded_str) = forwarded.to_str()
        && let Some(ip_str) = forwarded_str.split(',').next()
        && let Ok(ip) = ip_str.trim().parse()
    {
        return Some(ip);
    }
    if let Some(real_ip) = request.headers().get("X-Real-IP")
        && let Ok(ip_str) = real_ip.to_str()
        && let Ok(ip) = ip_str.parse()
    {
        return Some(ip);
    }
    None
}
