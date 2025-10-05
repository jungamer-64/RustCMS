# CHANGELOG

## Unreleased

### Added

- **Event-Driven Architecture**: Implemented comprehensive event system for decoupling handlers from search indexing and cache invalidation
  - Created `events.rs` with `AppEvent` enum supporting User, Post, Comment, Category, and Tag events
  - Implemented `listeners.rs` with dedicated search indexing and cache invalidation listeners
  - Added 15 `emit_*` helper methods to `AppState` for fire-and-forget event emission
  - Integrated event bus into application lifecycle with automatic listener spawning

### Changed

- **Handler Migration to Events**: Migrated handlers to use event system instead of direct service calls
  - `auth.rs`: `register` handler now emits `UserCreated` event
  - `users.rs`: `change_user_role` handler now emits `UserUpdated` event
  - `admin.rs`: `create_post` and `delete_post` now emit `PostCreated` and `PostDeleted` events
  - Handlers no longer directly call `search.index_*` or `invalidate_*_caches` methods

### Removed

- **Technical Debt Cleanup**: Removed unused `handlers_new` file (1841 lines)
  - Legacy monolithic handler file no longer in use
  - All functionality already migrated to modular handler files

- **Architecture Principles**: Event system follows key design principles
  - **Fire-and-Forget**: Event emission never fails primary operations
  - **Database as Source of Truth**: Listeners always fetch fresh data from database
  - **Resilient Error Handling**: Listener failures are logged but don't crash the system
  - **Lag Tolerance**: Listeners detect and handle channel overflow gracefully

### Tests

- **Unit Tests**: Added 13 unit tests for event bus functionality (`event_system_tests.rs`)
- **Integration Tests**: Implemented comprehensive integration test suite (`event_integration_tests.rs`)
  - Created mock services (`MockDatabase`, `MockSearchService`, `MockCacheService`)
  - Test for listener error resilience (search fails but cache continues)
  - Test for channel overflow handling with lag detection
  - Test for database-as-source-of-truth principle verification
  - Test for multiple independent listeners receiving same events

### Documentation

- **Architecture Documentation**: Created comprehensive `ARCHITECTURE.md` (460 lines)
  - Motivation and design principles
  - Component descriptions and usage guide
  - Testing strategy and performance considerations
  - Troubleshooting and best practices

### Technical Details

- Event bus uses `tokio::sync::broadcast` channel with configurable capacity
- Listeners run as background tasks spawned during application startup
- Event payloads are lightweight (IDs and essential metadata only)
- Search indexing and cache invalidation now fully decoupled from business logic

## 3.0.0 (Complete Biscuit Migration)

### Breaking Changes (v3.0.0)

- **Removed all legacy authentication features**: Complete migration to Biscuit-only authentication
- **Removed feature flags**: `legacy-auth-flat`, `legacy-admin-token`, `auth-flat-fields`
- **Removed flattened auth response fields**: `access_token`, `refresh_token`, `biscuit_token`, `expires_in`, `session_id`, `token`
  - Use `response.tokens.*` instead (e.g., `response.tokens.access_token`)
- **Removed legacy environment variables**: `ACCESS_TOKEN_TTL_SECS`, `REFRESH_TOKEN_TTL_SECS`
  - Use `AUTH_ACCESS_TOKEN_TTL_SECS` and `AUTH_REFRESH_TOKEN_TTL_SECS` instead
- **Removed admin token authentication**: All admin operations now use Biscuit authentication with SuperAdmin role

### Changed (v3.0.0)

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

### Breaking Changes (v2.0.0)

- Removed all JWT-based authentication logic; Biscuit tokens are now the sole auth mechanism.
- Eliminated `AuthError::Jwt` variant.
- Removed legacy `simple-cms` example project.

### Added (v2.0.0)

- Configurable `access_token_ttl_secs` and `refresh_token_ttl_secs` in `AuthConfig`.
- Biscuit access & refresh token rotation with in-memory session map (versioned refresh tokens).
- New test: `biscuit_token_flow_tests` validating refresh rotation & old token invalidation.

### Changed (v2.0.0)

- `AuthService::new` now derives keys from config/env (Biscuit only) and persists/generates as needed.
- Updated documentation (`README.md`, `README_PRODUCTION.md`) to reflect Biscuit-only auth.

### Removed (v2.0.0)

- All JWT environment variables, config fields, and code paths.


### Internal

- Consolidated token parsing/verification into `verify_biscuit_generic`.
- Pruned unused parser expiration field and simplified error surface.
