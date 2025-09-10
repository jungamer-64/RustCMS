//! Tampering tests for refresh biscuit tokens

use base64::Engine; // for decode/encode methods on STANDARD
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
        username: format!("user_tamper_{}", Uuid::new_v4().simple()),
        email: format!("tamper+{}@example.com", Uuid::new_v4()),
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
    let cfg = AuthConfig {
        biscuit_root_key: priv_b64,
        access_token_ttl_secs: 60,
        refresh_token_ttl_secs: 300,
        ..Default::default()
    };
    AuthService::new(&cfg, db).await.expect("auth init")
}

fn naive_tamper(b64: &str) -> String {
    // Base64 decode then perform a harmless ascii replacement to change token_type string; this invalidates signature
    let raw = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap_or_default();
    let mut text = String::from_utf8_lossy(&raw).to_string();
    if text.contains("refresh") {
        text = text.replace("refresh", "access");
    }
    // re-encode => biscuit verification should fail due to signature mismatch
    base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
}

#[tokio::test]
async fn tampered_refresh_token_rejected() {
    let db = match build_db().await {
        Some(d) => d,
        None => {
            eprintln!("SKIP tampered_refresh_token_rejected: DATABASE_URL not set");
            return;
        }
    };
    let auth = build_auth(&db).await;
    let user = dummy_user();
    let issued = auth.create_auth_response(user, false).await.expect("issue");
    let tampered = naive_tamper(&issued.tokens.refresh_token);
    assert!(
        auth.refresh_access_token(&tampered).await.is_err(),
        "tampered token should be invalid"
    );
}

#[tokio::test]
async fn random_garbage_token_rejected() {
    let db = match build_db().await {
        Some(d) => d,
        None => {
            eprintln!("SKIP random_garbage_token_rejected: DATABASE_URL not set");
            return;
        }
    };
    let auth = build_auth(&db).await;
    // 生成されていない乱数 base64 文字列
    let garbage = base64::engine::general_purpose::STANDARD.encode("totally-invalid-token");
    assert!(
        auth.refresh_access_token(&garbage).await.is_err(),
        "garbage token should be invalid"
    );
}
