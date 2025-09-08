//! Middleware layer using the unified `AppState.rate_limiter`.
//!
//! This replaces the previous ad-hoc in-module fixed window implementation.
//! All configuration now flows from `Config.security.*` via `AppState`.
//! Behaviour: if limit exceeded -> 429 + Retry-After (window seconds).

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::net::IpAddr;
use tower::{Layer, Service};
use crate::middleware::common::{BoxServiceFuture, forward_poll_ready};

use crate::AppState;
#[cfg(feature = "monitoring")]
use metrics::{counter, gauge};

#[derive(Clone)]
pub struct RateLimitLayer;

impl RateLimitLayer {
    pub fn new() -> Self { Self }
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
        // Access AppState (must be set via .with_state on the router)
        // We can't pull state directly here; instead embed a closure capturing service clone.
        let mut service = self.service.clone();

        // Extract IP early
        let ip = extract_ip_from_request(&request).unwrap_or(IpAddr::from([127,0,0,1]));

        // Pull state extension
        let state_opt = request.extensions().get::<AppState>().cloned();
        let allowed = state_opt
            .as_ref()
            .map(|s| s.allow_ip(&ip))
            .unwrap_or(true); // if no state, fail open

        #[cfg(feature = "monitoring")]
        {
            if allowed { counter!("ip_rate_limit_allowed_total").increment(1); } else { counter!("ip_rate_limit_blocked_total").increment(1); }
            if let Some(st) = state_opt.as_ref() { gauge!("ip_rate_limit_tracked_keys").set(st.rate_limiter.tracked_len() as f64); }
        }

        if !allowed {
            let retry_after = state_opt
                .as_ref()
                .map(|s| s.rate_limiter.window_secs())
                .unwrap_or(60);
            let response = (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_after.to_string())],
                "Rate limit exceeded. Please try again later.",
            ).into_response();
            return Box::pin(async move { Ok(response) });
        }

        Box::pin(async move { service.call(request).await })
    }
}

fn extract_ip_from_request<B>(request: &Request<B>) -> Option<IpAddr> {
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip_str) = forwarded_str.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse() { return Some(ip); }
            }
        }
    }
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() { if let Ok(ip) = ip_str.parse() { return Some(ip); } }
    }
    None
}
