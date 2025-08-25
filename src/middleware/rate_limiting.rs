use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower::{Layer, Service};

/// Rate limiting middleware for protecting against abuse
#[derive(Clone)]
pub struct RateLimitLayer {
    max_requests: u32,
    window: Duration,
    store: Arc<Mutex<HashMap<IpAddr, RateLimitInfo>>>,
}

#[derive(Debug, Clone)]
struct RateLimitInfo {
    requests: u32,
    window_start: Instant,
}

impl RateLimitLayer {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let mut store = self.store.lock().unwrap();
        let now = Instant::now();

        // Clean up expired entries
        store.retain(|_, info| now.duration_since(info.window_start) < self.window);

        match store.get_mut(&ip) {
            Some(info) => {
                if now.duration_since(info.window_start) >= self.window {
                    // Reset window
                    info.requests = 1;
                    info.window_start = now;
                    true
                } else if info.requests < self.max_requests {
                    info.requests += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                store.insert(
                    ip,
                    RateLimitInfo {
                        requests: 1,
                        window_start: now,
                    },
                );
                true
            }
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimitService {
            service,
            rate_limiter: self.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    service: S,
    rate_limiter: RateLimitLayer,
}

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        // Extract IP address from request
        let ip = extract_ip_from_request(&request).unwrap_or(IpAddr::from([127, 0, 0, 1]));

        if !self.rate_limiter.check_rate_limit(ip) {
            // Rate limit exceeded
            let response = (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", "60")],
                "Rate limit exceeded. Please try again later.",
            )
                .into_response();

            return Box::pin(async move { Ok(response) });
        }

        let mut service = self.service.clone();
        Box::pin(async move { service.call(request).await })
    }
}

fn extract_ip_from_request<B>(request: &Request<B>) -> Option<IpAddr> {
    // Try to get IP from X-Forwarded-For header first
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip_str) = forwarded_str.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return Some(ip);
            }
        }
    }

    // Fall back to connection info (not available in this context)
    None
}
