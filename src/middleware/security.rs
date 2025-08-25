use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Response,
};
use tower::{Layer, Service};

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
            add_security_header(
                headers,
                HeaderName::from_static("x-xss-protection"),
                "1; mode=block",
            );
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

            // Strict Transport Security (HSTS) for HTTPS
            add_security_header(
                headers,
                HeaderName::from_static("strict-transport-security"),
                "max-age=31536000; includeSubDomains; preload",
            );

            // Content Security Policy (CSP)
            add_security_header(
                headers,
                HeaderName::from_static("content-security-policy"),
                "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://unpkg.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'"
            );

            // Server identification
            add_security_header(
                headers,
                HeaderName::from_static("server"),
                "Enterprise-CMS/2.0",
            );

            Ok(response)
        })
    }
}

fn add_security_header(headers: &mut HeaderMap, name: HeaderName, value: &str) {
    if let Ok(header_value) = HeaderValue::from_str(value) {
        headers.insert(name, header_value);
    }
}
