# CHANGELOG

## Unreleased

- Add session rotation test for Biscuit tokens (access/refresh) (#feature/gen-biscuit-backups)

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

