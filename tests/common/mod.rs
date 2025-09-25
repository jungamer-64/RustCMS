//! Common test setup utilities

use std::sync::Once;

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
        if std::env::var("LOG_FORMAT").is_err() {
            unsafe {
                std::env::set_var("LOG_FORMAT", "text");
            }
        }

        // Allow overriding log level for tests with `TEST_VERBOSE=1`
        let verbose_tests = std::env::var("TEST_VERBOSE").is_ok();
        // In tests, we can afford to panic if logging initialization fails.
        cms_backend::telemetry::init_telemetry(verbose_tests)
            .expect("Failed to initialize telemetry for tests");
    });
}

/// Test helpers for auth and database integration tests

// These helpers are conditionally compiled and only available when both `auth` and `database` features are enabled.
#[cfg(all(feature = "auth", feature = "database"))]
use chrono::Utc;
use cms_backend::{
    auth::AuthService,
    config::{AuthConfig, DatabaseConfig},
    database::Database,
    models::{User, UserRole},
};
use uuid::Uuid;

/// Builds a database connection for tests. Returns `None` if `DATABASE_URL` is not set.
pub async fn build_db() -> Option<Database> {
    let url = std::env::var("DATABASE_URL").ok()?;
    if url.is_empty() {
        return None;
    }
    let cfg = DatabaseConfig {
        url: url.into(),
        max_connections: 2,
        min_connections: 1,
        connection_timeout: 5,
        idle_timeout: 30,
        max_lifetime: 300,
        enable_migrations: false, // In tests, we assume tables exist for speed.
    };
    Database::new(&cfg).await.ok()
}

/// Builds an `AuthService` instance for tests with configurable TTLs.
pub async fn build_auth(db: &Database, access_ttl: u64, refresh_ttl: u64) -> AuthService {
    use base64::Engine;
    use biscuit_auth::KeyPair;
    let kp = KeyPair::new();
    let priv_b64 = base64::engine::general_purpose::STANDARD.encode(kp.private().to_bytes());
    let cfg = AuthConfig {
        biscuit_root_key: priv_b64.into(),
        access_token_ttl_secs: access_ttl,
        refresh_token_ttl_secs: refresh_ttl,
        ..Default::default()
    };
    AuthService::new(&cfg, db).expect("auth service initialization for test failed")
}

/// Creates a dummy `User` model for testing purposes.
pub fn dummy_user() -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        username: format!("user_{}", Uuid::new_v4().simple()),
        email: format!("test+{}@example.com", Uuid::new_v4()),
        password_hash: Some("$argon2id$v=19$m=65536,t=3,p=4$YWJj$YWJj".into()), // Dummy hash
        first_name: None,
        last_name: None,
        role: UserRole::Subscriber.as_str().to_string(),
        is_active: true,
        email_verified: false,
        last_login: None,
        created_at: now,
        updated_at: now,
    }
}
