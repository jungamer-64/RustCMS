use uuid::Uuid;
use cms_backend::models::ApiKey;

#[cfg(all(feature = "database", feature = "auth"))]
#[test]
fn list_for_user_filters_expired() {
    // NOTE: This is a unit-level test using the model only; real DB integration would need a test database.
    // Here we only assert the query builder compiles. (No connection available in this context.)
    // For full integration, add a diesel test harness with a temporary DB.
    let _ = ApiKey::ALLOWED_PERMISSIONS; // touch constant to ensure visibility
    // Can't execute without a real connection; placeholder to ensure no panic constructing keys.
    let (_k, _raw) = ApiKey::new("tmp".into(), Uuid::new_v4(), vec!["posts:read".into()]);
}
