# CHANGELOG

## 3.0.0 (Complete Biscuit Migration)

### Breaking Changes

- **Removed all legacy authentication features**: Complete migration to Biscuit-only authentication
- **Removed feature flags**: `legacy-auth-flat`, `legacy-admin-token`, `auth-flat-fields`
- **Removed flattened auth response fields**: `access_token`, `refresh_token`, `biscuit_token`, `expires_in`, `session_id`, `token`
  - Use `response.tokens.*` instead (e.g., `response.tokens.access_token`)
- **Removed legacy environment variables**: `ACCESS_TOKEN_TTL_SECS`, `REFRESH_TOKEN_TTL_SECS`
  - Use `AUTH_ACCESS_TOKEN_TTL_SECS` and `AUTH_REFRESH_TOKEN_TTL_SECS` instead
- **Removed admin token authentication**: All admin operations now use Biscuit authentication with SuperAdmin role

### Changed

- `AuthSuccessResponse` now only contains `success`, `tokens`, and `user` fields
- All authentication uses unified Biscuit token system exclusively
- Updated OpenAPI documentation to reflect Biscuit-only authentication

### Migration Guide

For applications upgrading from 2.x:

1. Update all API response handling to use `response.tokens.*` instead of flat fields
2. Replace admin token authentication with Biscuit tokens + SuperAdmin role
3. Update environment variables to use new `AUTH_*` prefixed names
4. Regenerate API client SDKs from updated OpenAPI specification

### Improvements

- Simplified authentication codebase with single unified system
- Improved security through consistent Biscuit-based permissions
- Reduced technical debt by removing deprecated code paths

## Unreleased

- Add session rotation test for Biscuit tokens (access/refresh) (#feature/gen-biscuit-backups)
- Unified rate limiting system
- Introduced `limiter::FixedWindowLimiter` managed via `AppState` (config-driven `security.rate_limit_*`).
- Replaced ad-hoc IP middleware HashMap implementation with `RateLimitLayer` delegating to shared limiter.
- Added monitoring metrics (feature `monitoring`): `ip_rate_limit_allowed_total`, `ip_rate_limit_blocked_total`, `ip_rate_limit_tracked_keys`.
- Removed legacy `state::auth_metrics::RateLimiter` (superseded).
- Added generic trait `GenericRateLimiter` and `RateLimitDecision` enum; implemented for IP limiter and API key failure limiter via adapter.
- Refactored API key middleware to use unified trait adapter (`ApiKeyFailureLimiterAdapter`).
- Added tests `rate_limiter_tests` for fixed window + adapter behavior.

## 2.0.0 (Biscuit Unification)

### Breaking Changes

- Removed all JWT-based authentication logic; Biscuit tokens are now the sole auth mechanism.
- Eliminated `AuthError::Jwt` variant.
- Removed legacy `simple-cms` example project.

### Added

- Configurable `access_token_ttl_secs` and `refresh_token_ttl_secs` in `AuthConfig`.
- Biscuit access & refresh token rotation with in-memory session map (versioned refresh tokens).
- New test: `biscuit_token_flow_tests` validating refresh rotation & old token invalidation.

### Changed

- `AuthService::new` now derives keys from config/env (Biscuit only) and persists/generates as needed.
- Updated documentation (`README.md`, `README_PRODUCTION.md`) to reflect Biscuit-only auth.

### Removed

- All JWT environment variables, config fields, and code paths.

### Internal

- Consolidated token parsing/verification into `verify_biscuit_generic`.
- Pruned unused parser expiration field and simplified error surface.
