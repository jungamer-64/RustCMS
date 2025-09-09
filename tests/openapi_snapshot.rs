//! OpenAPI schema snapshot test
//! Ensures unintended schema regressions (e.g. accidental inclusion/removal of legacy types) are detected.
use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi; // bring openapi() into scope

#[test]
fn openapi_snapshot_default_features() {
    let doc = ApiDoc::openapi();
    let value = serde_json::to_value(&doc).expect("serialize openapi");

    // Structural guard: ensure AuthSuccessResponse required fields present (feature dependent)
    let auth_schema = value
        .pointer("/components/schemas/AuthSuccessResponse")
        .expect("AuthSuccessResponse schema present");
    let required = auth_schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("required array");
    let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    for f in ["success", "tokens", "user"].iter() {
        assert!(required_fields.contains(f), "missing required field {f}");
    }
    if cfg!(feature = "auth-flat-fields") {
        for f in ["access_token", "refresh_token", "biscuit_token", "expires_in", "session_id", "token"].iter() {
            assert!(required_fields.contains(f), "missing flat required field {f}");
        }
    } else {
        // Ensure flat fields not required
        for f in ["access_token", "refresh_token", "biscuit_token", "expires_in", "session_id", "token"].iter() {
            assert!(!required_fields.contains(f), "flat field {f} should be absent when feature disabled");
        }
    }

    // Legacy LoginResponse absence check
    if cfg!(not(feature = "legacy-auth-flat")) {
        assert!(value.pointer("/components/schemas/LoginResponse").is_none());
    }
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
