# CHANGELOG

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

### Planned (Breaking Change Notice for 3.0.0)

The following items are scheduled for removal in the next major release (3.0.0). They remain available in 2.x with deprecation warnings to provide a clear migration window:

- Cargo feature `legacy-auth-flat` (will be removed; use nested `tokens` object only)
- Flattened auth response fields: `access_token`, `refresh_token`, `biscuit_token`, `expires_in`, `session_id`, `token`
- Legacy `LoginResponse` schema (currently gated behind `legacy-auth-flat`)

Migration Guidance (repeat from docs for visibility):

1. Replace any direct `response.access_token` style references with `response.tokens.access_token`
2. Remove reliance on `token` alias
3. Regenerate client SDKs after upgrading; stale cached OpenAPI artifacts may retain removed fields

Risk Mitigation:

- Early notice (this entry + `docs/AUTH_MIGRATION_V2.md`)
- Warnings at compile-time via `#[deprecated]` attributes
- CI recommendation: add a job that builds with `--no-default-features --features auth,database` to ensure no accidental flat-field dependencies linger

Tracking Reference: see `docs/AUTH_MIGRATION_V2.md` Phase 4 plan and `deduplicated_logic_report.csv` (`phase4_plan_doc`).

### Deprecations (Active in 2.x)

These items are deprecated and will be removed in 3.0.0:

- Flattened auth response fields (`access_token`, `refresh_token`, `biscuit_token`, `expires_in`, `session_id`, `token`) – use `response.tokens.*`
- `LoginResponse` schema (available only when `legacy-auth-flat` feature is enabled)
- Cargo feature `legacy-auth-flat` itself (migration aid only)

Monitoring Guidance:

- Run: `grep -R "\.access_token" -n src/ | grep -v auth_response.rs` to detect lingering direct field usage
- Consider adding a non-fatal CI job that reports count of deprecated field accesses outside their defining module

Removal Checklist lives in: `docs/AUTH_MIGRATION_V2.md` (Phase 4)

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
