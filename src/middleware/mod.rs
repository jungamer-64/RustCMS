pub mod common;
pub mod compression;
pub mod deprecation; // Phase 5-4: RFC 8594 Deprecation headers for API v1
pub mod logging;
pub mod rate_limit_backend; // pluggable API key rate limiting backends
pub mod request_id;
pub mod security; // shared middleware helpers
