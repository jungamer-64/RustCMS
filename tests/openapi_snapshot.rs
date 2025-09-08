//! OpenAPI schema snapshot test
//! Ensures unintended schema regressions (e.g. accidental inclusion/removal of legacy types) are detected.
use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi; // bring openapi() into scope

#[test]
fn openapi_snapshot_default_features() {
    let doc = ApiDoc::openapi();
    // Serialize with deterministic ordering (utoipa already preserves ordering, but we enforce via serde_json value roundtrip)
    let value = serde_json::to_value(&doc).expect("serialize openapi");

    // Prune potentially noisy sections if needed (currently none). Example placeholder:
    // if let Some(serde_json::Value::Object(components)) = value.get_mut("components") { /* adjust */ }

    // Assertion: LoginResponse schema MUST be absent when feature `legacy-auth-flat` is disabled.
    if cfg!(not(feature = "legacy-auth-flat")) {
        let has_login = value
            .pointer("/components/schemas/LoginResponse")
            .is_some();
        assert!(!has_login, "LoginResponse should not be present without legacy-auth-flat feature");
    }

    insta::assert_json_snapshot!("openapi_default", value);
}

/// When the transitional feature `legacy-auth-flat` is enabled we expect the legacy LoginResponse
/// schema to be present. This test only compiles under that feature.
#[cfg(feature = "legacy-auth-flat")]
#[test]
fn openapi_snapshot_legacy_auth_flat_feature() {
    let doc = ApiDoc::openapi();
    let value = serde_json::to_value(&doc).expect("serialize openapi");
    let has_login = value.pointer("/components/schemas/LoginResponse").is_some();
    assert!(has_login, "LoginResponse schema should be present when legacy-auth-flat feature is enabled");
}
