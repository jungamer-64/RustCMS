use cms_backend::models::ApiKey;

mod common;

#[cfg(not(feature = "database"))]
#[test]
fn skip_without_db() { /* database feature required for ApiKey model */
}
use uuid::Uuid;

#[cfg(feature = "database")]
#[test]
fn api_key_generation_and_verify() {
    common::setup();
    let user_id = Uuid::new_v4();
    let (model, raw) = ApiKey::new_validated(
        "test-key".to_string(),
        user_id,
        vec!["posts:read".into(), "users:read".into()],
    )
    .expect("should create a valid API key");
    assert_eq!(model.get_permissions().len(), 2);
    assert!(
        model
            .verify_key(&raw)
            .expect("verification should not fail")
    );
    assert!(!raw.is_empty());
    assert!(raw.starts_with("ak_"));
    let masked = ApiKey::mask_raw(&raw);
    assert!(masked.contains('…'));
    let resp = model.to_response();
    assert_eq!(resp.name, "test-key");
}

#[cfg(feature = "database")]
#[test]
fn api_key_verify_wrong() {
    common::setup();
    let user_id = Uuid::new_v4();
    let (model, _raw) =
        ApiKey::new_validated("test-key".to_string(), user_id, vec!["posts:read".into()])
            .expect("should create a valid API key");
    assert!(
        !model
            .verify_key("invalid")
            .expect("verification should not fail")
    );
    assert!(!model.is_expired(chrono::Utc::now()));
}

#[cfg(feature = "database")]
#[test]
fn api_key_invalid_permission_rejected() {
    common::setup();
    let user_id = Uuid::new_v4();
    let err = ApiKey::new_validated("bad".to_string(), user_id, vec!["unknown:perm".into()])
        .expect_err("should fail");
    let as_string = format!("{err}"); // ensure Display
    assert!(as_string.contains("invalid_permission") || as_string.contains("Validation"));
}
