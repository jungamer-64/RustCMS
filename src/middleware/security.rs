use crate::middleware::common::{BoxServiceFuture, forward_poll_ready};
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, Method},
    response::Response,
};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::{Layer, Service};

// HSTS max-age in seconds (1 year)
const HSTS_MAX_AGE: u64 = 31_536_000;

/// CSRF protection service for preventing Cross-Site Request Forgery attacks
#[derive(Clone)]
pub struct CsrfService {
    /// Secret key for HMAC token generation
    _secret_key: Arc<[u8; 32]>,
    /// Active CSRF tokens (in production, use Redis or database)
    active_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl CsrfService {
    /// Create new CSRF protection service
    pub fn new() -> Self {
        // Generate secure random key for CSRF token generation
        let mut key = [0u8; 32];
        rand::rng().fill(&mut key);

        Self {
            _secret_key: Arc::new(key),
            active_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Generate a new CSRF token using secure random generation
    pub async fn generate_token(&self) -> String {
        // Generate random token using cryptographically secure random
        let mut token_bytes = [0u8; 32];
        rand::rng().fill(&mut token_bytes);

        // Convert to base64 for safe transmission
        let token = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            token_bytes,
        );

        // Store token for validation (防止 token 重复利用攻击)
        self.active_tokens.write().await.insert(token.clone());

        token
    }

    /// Validate CSRF token and consume it (one-time use)
    pub async fn validate_and_consume_token(&self, token: &str) -> bool {
        // Basic format validation
        if token.is_empty() || token.len() < 32 {
            return false;
        }

        // Check if token exists and remove it (one-time use security)
        let mut tokens = self.active_tokens.write().await;
        tokens.remove(token)
    }

    /// Clean expired tokens (in production, implement with TTL)
    pub async fn cleanup_expired_tokens(&self) {
        // Simple cleanup - in production, use proper TTL management
        let mut tokens = self.active_tokens.write().await;
        if tokens.len() > 10000 {
            tokens.clear(); // Reset when too many tokens accumulate
        }
    }
}

impl Default for CsrfService {
    fn default() -> Self {
        Self::new()
    }
}

/// Security headers middleware for enterprise security compliance
#[derive(Clone)]
pub struct SecurityHeadersLayer;

impl SecurityHeadersLayer {
    #[must_use]
    pub const fn new() -> Self {
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
            let hsts_val = format!("max-age={HSTS_MAX_AGE}; includeSubDomains; preload");
            add_security_header(
                headers,
                HeaderName::from_static("strict-transport-security"),
                &hsts_val,
            );

            // Content Security Policy (CSP)
            let csp = build_csp_for_path(&path);
            add_security_header(
                headers,
                HeaderName::from_static("content-security-policy"),
                &csp,
            );

            // Server identification
            // Avoid leaking server details; omit explicit Server header (some platforms add it anyway)

            Ok(response)
        })
    }
}

fn build_csp_for_path(path: &str) -> String {
    if path.starts_with("/api/docs") {
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://unpkg.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'".to_string()
    } else {
        "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' https://unpkg.com https://cdn.jsdelivr.net; style-src 'self' https://fonts.googleapis.com https://unpkg.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'".to_string()
    }
}

fn add_security_header(headers: &mut HeaderMap, name: HeaderName, value: &str) {
    if let Ok(header_value) = HeaderValue::from_str(value) {
        headers.insert(name, header_value);
    }
}

/// HTML escaping utility to prevent XSS attacks
/// 提供 HTML 转义功能以防止 XSS 攻击
#[must_use]
pub fn escape_html(input: &str) -> String {
    // Basic HTML entity escaping for security
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

/// Sanitize user input to prevent various injection attacks
/// 清理用户输入以防止注入攻击
#[must_use]
pub fn sanitize_input(input: &str) -> String {
    // Remove potentially dangerous characters and sequences
    let sanitized: String = input
        .replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .replace("javascript:", "")
        .replace("data:", "")
        .replace("vbscript:", "")
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace()) // Remove control characters except whitespace
        .collect();

    // Apply HTML escaping
    escape_html(&sanitized)
}

/// URL encoding utility for safe URL construction
/// 安全的 URL 构建工具
#[must_use]
pub fn encode_url_component(input: &str) -> String {
    urlencoding::encode(input).to_string()
}

/// Check if endpoint requires CSRF protection
/// API endpoints with proper authentication (Bearer tokens) are exempt from CSRF
#[must_use]
pub fn is_csrf_protected_endpoint(method: &Method, path: &str) -> bool {
    // Only protect state-changing operations
    if !matches!(
        method,
        &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
    ) {
        return false;
    }

    // Exclude API endpoints with proper token authentication from CSRF
    // These are protected by token-based auth which is inherently CSRF-resistant
    if path.starts_with("/api/v1/") && !path.contains("/auth/") {
        return false;
    }

    // Protect form-based endpoints and admin operations
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_csrf_token_generation_and_validation() {
        let csrf_service = CsrfService::new();

        // Generate token
        let token = csrf_service.generate_token().await;
        assert!(!token.is_empty());

        // Validate token (should succeed)
        assert!(csrf_service.validate_and_consume_token(&token).await);

        // Validate same token again (should fail - one-time use)
        assert!(!csrf_service.validate_and_consume_token(&token).await);
    }

    #[test]
    fn test_html_escaping() {
        let dangerous_input = "<script>alert('xss')</script>";
        let escaped = escape_html(dangerous_input);
        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;"
        );
    }

    #[test]
    fn test_input_sanitization() {
        let dangerous_input = "<script>alert('xss')</script><img onerror='alert(1)' src='x'>";
        let sanitized = sanitize_input(dangerous_input);

        // Should not contain executable script tags
        assert!(!sanitized.contains("<script"));
        assert!(!sanitized.contains("javascript:"));
    }

    #[test]
    fn test_url_encoding() {
        let input = "hello world & special chars";
        let encoded = encode_url_component(input);
        assert_eq!(encoded, "hello%20world%20%26%20special%20chars");
    }

    #[test]
    fn test_csrf_protection_logic() {
        use axum::http::Method;

        // State-changing operations on forms should be protected
        assert!(is_csrf_protected_endpoint(&Method::POST, "/admin/posts"));
        assert!(is_csrf_protected_endpoint(&Method::PUT, "/forms/contact"));

        // API endpoints with token auth should not require CSRF
        assert!(!is_csrf_protected_endpoint(&Method::POST, "/api/v1/posts"));
        assert!(!is_csrf_protected_endpoint(
            &Method::DELETE,
            "/api/v1/users/123"
        ));

        // Auth endpoints still need CSRF for cookie-based flows
        assert!(is_csrf_protected_endpoint(
            &Method::POST,
            "/api/v1/auth/login"
        ));

        // GET requests don't need CSRF
        assert!(!is_csrf_protected_endpoint(&Method::GET, "/admin/posts"));
    }
}
