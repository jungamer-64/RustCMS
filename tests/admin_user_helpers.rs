//! Admin CLI user helper integration tests (create / update / delete)
#![cfg(feature = "database")]

use cms_backend::config::DatabaseConfig;
use cms_backend::database::Database;
use cms_backend::models::{CreateUserRequest, UpdateUserRequest, UserRole};
use uuid::Uuid;

async fn build_db() -> Option<Database> {
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
        enable_migrations: false, // tests expect existing schema / speed
    };
    Database::new(&cfg).await.ok()
}

fn random_suffix() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

#[tokio::test]
async fn admin_user_create_update_delete() {
    let Some(db) = build_db().await else {
        eprintln!("SKIP admin_user_create_update_delete: DATABASE_URL not set");
        return;
    };

    let suffix = random_suffix();
    let username = format!("test_user_{}", suffix);
    let email = format!("test+{}@example.com", suffix);

    // Create
    let create_req = CreateUserRequest {
        username: username.clone(),
        email: email.clone(),
        password: "Password1!".to_string(),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        role: UserRole::Subscriber,
    };

    let created = db.create_user(create_req).await.expect("create_user");
    assert_eq!(created.username, username);
    assert_eq!(created.email, email);

    // Update (email + role + deactivate)
    let new_email = format!("updated+{}@example.com", suffix);
    let update_req = UpdateUserRequest {
        username: None,
        email: Some(new_email.clone()),
        first_name: None,
        last_name: None,
        role: Some(UserRole::Admin),
        is_active: Some(false),
    };

    let updated = db
        .update_user(created.id, &update_req)
        .expect("update_user");
    assert_eq!(updated.email, new_email);
    assert_eq!(updated.role, UserRole::Admin.as_str());
    assert!(!updated.is_active);

    // Delete
    db.delete_user(created.id).expect("delete_user");

    // Verify deletion
    let got = db.get_user_by_id(created.id).await;
    assert!(got.is_err(), "expected user to be deleted");
}
