use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Response,
};
use tower::{Layer, Service};
use crate::middleware::common::{BoxServiceFuture, forward_poll_ready};

/// Security headers middleware for enterprise security compliance
#[derive(Clone)]
pub struct SecurityHeadersLayer;

impl SecurityHeadersLayer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SecurityHeadersLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, service: S) -> Self::Service {
        SecurityHeadersService { service }
    }
}

#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    service: S,
}

impl<S, B> Service<Request<B>> for SecurityHeadersService<S>
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
        // Capture path before moving request into inner service so we can tailor CSP for docs
        let path = request.uri().path().to_string();
        let mut service = self.service.clone();

        Box::pin(async move {
            let mut response = service.call(request).await?;

            let headers = response.headers_mut();

            // Security headers for enterprise compliance
            add_security_header(
                headers,
                HeaderName::from_static("x-content-type-options"),
                "nosniff",
            );
            add_security_header(headers, HeaderName::from_static("x-frame-options"), "DENY");
            // 'X-XSS-Protection' is obsolete on modern browsers; prefer strong CSP instead
            add_security_header(
                headers,
                HeaderName::from_static("referrer-policy"),
                "strict-origin-when-cross-origin",
            );
            add_security_header(
                headers,
                HeaderName::from_static("permissions-policy"),
                "geolocation=(), microphone=(), camera=()",
            );
            // Additional hardening
            add_security_header(
                headers,
                HeaderName::from_static("cross-origin-opener-policy"),
                "same-origin",
            );
            add_security_header(
                headers,
                HeaderName::from_static("x-permitted-cross-domain-policies"),
                "none",
            );

            // Strict Transport Security (HSTS) for HTTPS
            add_security_header(
                headers,
                HeaderName::from_static("strict-transport-security"),
                "max-age=31536000; includeSubDomains; preload",
            );

            // Content Security Policy (CSP)
            // Use a stricter CSP by default. Allow inline only for docs UI endpoints.
            let is_docs = path.starts_with("/api/docs");
            let csp = if is_docs {
                // Swagger UI requires some inline styles/scripts; relax only for docs routes
                "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://unpkg.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'"
            } else {
                // No 'unsafe-inline' for app endpoints; add extra directives
                "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' https://unpkg.com https://cdn.jsdelivr.net; style-src 'self' https://fonts.googleapis.com https://unpkg.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'"
            };
            add_security_header(
                headers,
                HeaderName::from_static("content-security-policy"),
                csp,
            );

            // Server identification
            // Avoid leaking server details; omit explicit Server header (some platforms add it anyway)

            Ok(response)
        })
    }
}

fn add_security_header(headers: &mut HeaderMap, name: HeaderName, value: &str) {
    if let Ok(header_value) = HeaderValue::from_str(value) {
        headers.insert(name, header_value);
    }
}
