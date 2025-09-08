---
name: Phase 4 Removal (Auth Flat Fields)
about: Execute 3.0.0 removal of legacy-auth-flat + flattened auth fields
labels: cleanup, breaking-change
---

# Phase 4 Removal (Auth Flat Fields)

## Scope

Remove deprecated flattened auth fields & `legacy-auth-flat` feature per Phase 4 migration plan.

## Tasks

- [ ] Remove feature `legacy-auth-flat` from Cargo.toml
- [ ] Delete `LoginResponse` struct & cfg-gated code
- [ ] Remove deprecated flattened fields from `AuthSuccessResponse`
- [ ] Update OpenAPI generation (eliminate conditional schema injection)
- [ ] Refactor tests to rely solely on `response.tokens.*`
- [ ] Delete any snapshot fixtures referencing flattened fields (regenerate if needed)
- [ ] Run `cargo test --all` (ensure green)
- [ ] Run `grep -R "\.access_token" -n src/ | grep -v tokens` and confirm only references are within struct definitions or tests explicitly covering deprecation removal
- [ ] Regenerate docs (`cargo run --bin dump_openapi` or docs pipeline)
- [ ] Update CHANGELOG: Move Planned items into 3.0.0 Breaking Changes
- [ ] Add migration note to README (confirm Phase 4 section removed or updated)

## Validation

Describe verification steps & paste summarized diffs for critical files.

## Rollback Plan

If issues arise, restore feature & flattened fields from previous minor tag (2.x) and re-publish patch hotfix tag; communicate via release notes.

## Additional Notes

Link related discussion / PRs / external SDK update tracking here.
