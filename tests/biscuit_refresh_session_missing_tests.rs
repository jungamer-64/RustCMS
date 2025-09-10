//! Session missing scenario: refresh should fail when session record is removed

use chrono::Utc;
use cms_backend::auth::AuthService;
use cms_backend::config::{AuthConfig, DatabaseConfig};
use cms_backend::database::Database;
use cms_backend::models::{User, UserRole};
use uuid::Uuid;

async fn build_db() -> Option<Database> {
    let url = std::env::var("DATABASE_URL").ok()?;
    if url.is_empty() {
        return None;
    }
    let cfg = DatabaseConfig {
        url,
        max_connections: 2,
        min_connections: 1,
        connection_timeout: 5,
        idle_timeout: 30,
        max_lifetime: 300,
        enable_migrations: false,
    };
    Database::new(&cfg).await.ok()
}
fn dummy_user() -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        username: format!("user_sess_missing_{}", Uuid::new_v4().simple()),
        email: format!("sm+{}@example.com", Uuid::new_v4()),
        password_hash: Some("$argon2id$v=19$m=65536,t=3,p=4$YWJj$YWJj".into()),
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
async fn build_auth(db: &Database) -> AuthService {
    use base64::Engine;
    use biscuit_auth::KeyPair;
    let kp = KeyPair::new();
    let priv_b64 = base64::engine::general_purpose::STANDARD.encode(kp.private().to_bytes());
    let mut cfg = AuthConfig::default();
    cfg.biscuit_root_key = priv_b64;
    cfg.access_token_ttl_secs = 60;
    cfg.refresh_token_ttl_secs = 300;
    AuthService::new(&cfg, db).await.expect("auth init")
}

#[tokio::test]
async fn refresh_fails_when_session_missing() {
    let db = match build_db().await {
        Some(d) => d,
        None => {
            eprintln!("SKIP refresh_fails_when_session_missing: DATABASE_URL not set");
            return;
        }
    };
    let auth = build_auth(&db).await;
    let user = dummy_user();
    let issued = auth.create_auth_response(user, false).await.expect("issue");
    // Clear sessions (simulate eviction / restart without persistence)
    auth.clear_sessions_for_test().await;
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err(),
        "refresh should fail after session map cleared"
    );
}
