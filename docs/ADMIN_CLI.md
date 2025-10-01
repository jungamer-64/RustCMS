# Admin CLI — developer notes

This document explains the refactored layout for the admin CLI binary and how to run tests locally.

Location
- CLI entrypoint: `src/bin/admin.rs` (thin dispatcher)
- Admin submodules: `src/bin/admin/` containing:
  - `cli.rs` — clap definitions for `Cli`, `Commands`, and action enums
  - `backend.rs` — `AdminBackend` trait and `AppState` implementation
  - `util.rs` — helpers for password prompting/generation and user lookup
  - `handlers/` — subcommands split by domain (user, system, content, analytics, security)

Goals
- Secure password handling using `secrecy::SecretString` and `rpassword` for prompts
- Typed CLI with clap v4 for robust parsing and validation
- Testable handlers via `AdminBackend` trait so tests can inject mocks

How to run locally

Run unit tests and integration tests:

```bash
cargo test
```

Run clippy and formatter checks:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
```

Run only the admin binary's tests (if present):

```bash
cargo test --bin cms-admin
```

Notes & next steps
- `content` handler remains a NotImplemented stub and needs porting from the old monolith if required.
- The `AdminBackend` trait is intentionally narrow; expand methods if new admin commands need database operations.

If you want, I can also add a short README snippet in the repository root linking to this document.
