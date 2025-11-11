//! Common test setup utilities

use std::sync::Once;
use temp_env::with_var;

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
pub fn setup() {
    INIT.call_once(|| {
        // Load .env file if present.
        // This is useful for tests that need environment variables (e.g., DATABASE_URL).
        let _ = dotenvy::dotenv();

        // Initialize telemetry/logging.
        // The `try_init` in `init_telemetry` makes it safe to call this multiple times,
        // but `Once` ensures we don't even try after the first time.
        // We can set LOG_FORMAT=text for more readable test output.
        let verbose_tests = std::env::var("TEST_VERBOSE").is_ok();
        let init_result = if std::env::var("LOG_FORMAT").is_err() {
            with_var("LOG_FORMAT", Some("text"), || {
                cms_backend::telemetry::init_telemetry(verbose_tests)
            })
        } else {
            cms_backend::telemetry::init_telemetry(verbose_tests)
        };

        match init_result {
            Ok(_) => {}
            Err(e) => {
                // Try structured logging first for consistency with production
                // observability. Always also print to stderr so the failure
                // is visible in CI logs even if a tracing subscriber wasn't
                // installed.
                tracing::warn!(error = %e, "init_telemetry failed for tests");
                eprintln!("init_telemetry failed for tests: {e}");
            }
        }
    });
}

// Test helpers for auth and database integration tests

// The following helpers are only available when both `auth` and `database` features are enabled.
// We limit the scope of `cfg` to the items that require those features so other tests
// don't get their imports gated unintentionally.
use uuid::Uuid;

// Reusable dummy password hash for tests. Centralize the value so it's easier
// to change or spot in test output.
#[allow(dead_code)]
const DUMMY_HASH: &str = "$argon2id$v=19$m=65536,t=3,p=4$YWJj$YWJj";

/// Like `build_db`, but returns a `Result` so callers can distinguish
/// between "no `DATABASE_URL` set" (Ok(None)) and a connection error (Err).
#[allow(dead_code)]
#[cfg(feature = "database")]
pub async fn build_db_result()
-> Result<Option<cms_backend::infrastructure::database::connection::DatabasePool>, cms_backend::AppError> {
    use cms_backend::config::DatabaseConfig;
    use secrecy::ExposeSecret;

    let url = std::env::var("DATABASE_URL").ok();
    let url = match url {
        None => return Ok(None),
        Some(u) if u.is_empty() => return Ok(None),
        Some(u) => u,
    };

    let cfg = DatabaseConfig {
        url: url.into(),
        max_connections: 2,
        min_connections: 1,
        connection_timeout: 5,
        idle_timeout: 30,
        max_lifetime: 300,
        enable_migrations: false,
    };

    match cms_backend::infrastructure::database::connection::DatabasePool::new(cfg.url.expose_secret()) {
        Ok(db) => Ok(Some(db)),
        Err(e) => Err(cms_backend::AppError::Internal(e.to_string())),
    }
}

/// Convenience helper mirroring legacy behavior: returns `None` when the
/// database is unavailable and logs any initialization errors.
#[allow(dead_code)]
#[cfg(feature = "database")]
pub async fn build_db() -> Option<cms_backend::infrastructure::database::connection::DatabasePool> {
    match build_db_result().await {
        Ok(db_opt) => db_opt,
        Err(e) => {
            tracing::warn!(error = %e, "failed to initialize database for tests");
            eprintln!("failed to initialize database for tests: {e}");
            None
        }
    }
}

/// Builds an `AuthService` instance for tests with configurable TTLs.
#[cfg(all(feature = "auth", feature = "database"))]
#[allow(dead_code, clippy::unused_async)] // Used by tests, no actual async operations needed
pub async fn build_auth(
    db: &cms_backend::infrastructure::database::connection::DatabasePool,
    access_ttl: u64,
    refresh_ttl: u64,
) -> cms_backend::auth::AuthService {
    use base64::Engine;
    use biscuit_auth::KeyPair;
    use cms_backend::config::AuthConfig;

    let kp = KeyPair::new();
    let priv_b64 = base64::engine::general_purpose::STANDARD.encode(kp.private().to_bytes());
    let cfg = AuthConfig {
        biscuit_root_key: priv_b64.into(),
        access_token_ttl_secs: access_ttl,
        refresh_token_ttl_secs: refresh_ttl,
        ..Default::default()
    };
    cms_backend::auth::AuthService::new(&cfg, db)
        .expect("auth service initialization for test failed")
}

/// Creates a dummy `User` model for testing purposes.
#[cfg(all(feature = "auth", feature = "database"))]
#[allow(dead_code)]
pub fn dummy_user() -> cms_backend::models::User {
    use chrono::Utc;
    use cms_backend::domain::user::{Email, Username, UserId, UserRole};
    use uuid::Uuid;

    let username = Username::new(format!("user_{}", Uuid::new_v4().simple()))
        .expect("Failed to create username");
    let email = Email::new(format!("test+{}@example.com", Uuid::new_v4()))
        .expect("Failed to create email");
    
    let now = Utc::now();
    cms_backend::models::User::restore(
        UserId::new(),
        username,
        email,
        Some(DUMMY_HASH.into()),
        UserRole::Subscriber,
        true,
        now,
        now,
        None,
    )
}
