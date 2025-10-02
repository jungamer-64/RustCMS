# Contributing to RustCMS

Thank you for your interest in contributing to RustCMS! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful and professional in all interactions.

## Getting Started

### Prerequisites

- Rust 1.75 or later (2024 edition)
- PostgreSQL 14 or later
- Redis 6 or later (optional, for caching)
- Git

### Forking and Cloning

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/RustCMS.git
   cd RustCMS
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/jungamer-64/RustCMS.git
   ```

## Development Environment

### Initial Setup

1. **Install dependencies:**
   ```bash
   cargo build
   ```

2. **Set up the database:**
   ```bash
   # Create database
   createdb cms_dev
   
   # Run migrations
   cargo run --bin cms-migrate
   ```

3. **Configure environment:**
   Create a `.env` file:
   ```env
   DATABASE_URL=postgres://localhost/cms_dev
   REDIS_URL=redis://localhost:6379
   RUST_LOG=debug
   ```

4. **Generate Biscuit keys:**
   ```bash
   cargo run --bin gen_biscuit_keys
   ```

### Running the Development Server

```bash
cargo run --bin cms-server
```

The server will start on `http://localhost:8080` by default.

## Project Structure

```
RustCMS/
├── src/
│   ├── auth/          # Authentication and authorization
│   ├── cache/         # Caching layer
│   ├── database/      # Database models and connections
│   ├── handlers/      # HTTP request handlers
│   ├── middleware/    # Middleware components
│   ├── models/        # Domain models
│   ├── repositories/  # Data access layer
│   ├── routes/        # Route definitions
│   ├── search/        # Full-text search
│   └── utils/         # Utility functions
├── tests/             # Integration tests
│   └── fixtures/      # Test data
├── examples/          # Usage examples
├── benches/           # Performance benchmarks
├── migrations/        # Database migrations
└── docs/              # Documentation
```

## Coding Standards

### Rust Style Guide

- Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `rustfmt` for formatting:
  ```bash
  cargo fmt
  ```
- Use `clippy` for linting:
  ```bash
  cargo clippy -- -D warnings
  ```

### Code Quality

- **Documentation:** All public APIs must have documentation comments
- **Error Handling:** Use `Result` types; avoid panicking in library code
- **Type Safety:** Prefer strongly-typed APIs over stringly-typed ones
- **Performance:** Consider performance implications; use benchmarks for critical paths

### Naming Conventions

- **Files:** Snake case (e.g., `user_repository.rs`)
- **Functions:** Snake case (e.g., `create_user`)
- **Types:** Pascal case (e.g., `UserRepository`)
- **Constants:** Screaming snake case (e.g., `MAX_UPLOAD_SIZE`)

### Comments

```rust
/// Creates a new user with the given credentials.
///
/// # Arguments
///
/// * `username` - The desired username
/// * `email` - The user's email address
/// * `password` - The plaintext password (will be hashed)
///
/// # Returns
///
/// Returns `Ok(User)` on success, or an error if the user cannot be created.
///
/// # Errors
///
/// This function will return an error if:
/// - The username is already taken
/// - The email is invalid
/// - Database connection fails
///
/// # Example
///
/// ```rust
/// let user = create_user("alice", "alice@example.com", "secure_pass")?;
/// ```
pub fn create_user(username: &str, email: &str, password: &str) -> Result<User, Error> {
    // Implementation
}
```

## Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

### Writing Tests

#### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("alice", "alice@example.com");
        assert_eq!(user.username, "alice");
    }
}
```

#### Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/user_integration_tests.rs
use cms_backend::*;

#[tokio::test]
async fn test_user_api() {
    // Test implementation
}
```

#### Using Test Fixtures

```rust
use std::fs;

#[test]
fn test_with_fixture() {
    let fixture = fs::read_to_string("tests/fixtures/sample_posts.json")
        .expect("Failed to read fixture");
    let posts: Vec<Post> = serde_json::from_str(&fixture).unwrap();
    assert_eq!(posts.len(), 5);
}
```

### Test Coverage

- Aim for at least 80% code coverage
- All public APIs must have tests
- Test both happy paths and error conditions
- Use mocks for external dependencies when appropriate

## Submitting Changes

### Branching Strategy

- `main` - Stable production code
- `develop` - Integration branch for features
- `feature/*` - Feature branches
- `fix/*` - Bug fix branches
- `refactor/*` - Refactoring branches

### Creating a Pull Request

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes and commit:**
   ```bash
   git add .
   git commit -m "feat: add user authentication"
   ```

3. **Follow commit message conventions:**
   ```
   <type>(<scope>): <subject>
   
   <body>
   
   <footer>
   ```
   
   Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

4. **Push to your fork:**
   ```bash
   git push origin feature/my-feature
   ```

5. **Create pull request on GitHub:**
   - Fill out the PR template
   - Link related issues
   - Add appropriate labels

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Documentation updated
- [ ] Changelog updated (for significant changes)
- [ ] No compiler warnings
- [ ] Clippy checks pass
- [ ] Commit messages follow conventions

## Review Process

### What to Expect

1. **Automated checks:** CI/CD pipeline runs tests, linting, security scans
2. **Code review:** Maintainers will review your code
3. **Feedback:** You may be asked to make changes
4. **Approval:** Once approved, your PR will be merged

### Review Criteria

- **Correctness:** Code does what it's supposed to do
- **Quality:** Code is clean, readable, and maintainable
- **Tests:** Adequate test coverage
- **Documentation:** Clear and complete documentation
- **Performance:** No unnecessary performance degradation
- **Security:** No security vulnerabilities introduced

## Development Tips

### Useful Commands

```bash
# Check compilation without running
cargo check

# Build in release mode
cargo build --release

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open

# Run specific binary
cargo run --bin cms-admin

# Watch mode (requires cargo-watch)
cargo watch -x test
```

### Debugging

- Use `RUST_LOG=debug` for detailed logging
- Use `RUST_BACKTRACE=1` for stack traces
- Use the Rust debugging tools in your IDE

### Performance Profiling

```bash
# Profile with flamegraph
cargo flamegraph --bin cms-server

# Run benchmarks
cargo bench
```

## Getting Help

- **Documentation:** Check `/docs` directory
- **Issues:** Search existing issues on GitHub
- **Discussions:** Use GitHub Discussions for questions
- **Discord:** Join our community Discord server

## License

By contributing to RustCMS, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to RustCMS! 🦀
