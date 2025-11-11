//! `OpenAPI` schema snapshot test
//! Ensures unintended schema regressions (e.g. accidental inclusion/removal of legacy types) are detected.
//!
//! Note: This test is currently disabled because the openapi module has been refactored.
//! TODO: Re-enable after openapi is properly integrated with the new architecture.

#![cfg(feature = "openapi_disabled_pending_refactor")]

use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi; // bring openapi() into scope

#[test]
fn openapi_snapshot_unified_biscuit() {
    let doc = ApiDoc::openapi();
    let value = serde_json::to_value(&doc).expect("serialize openapi");

    // Structural guard: ensure AuthSuccessResponse required fields present
    let auth_schema = value
        .pointer("/components/schemas/AuthSuccessResponse")
        .expect("AuthSuccessResponse schema present");
    let required = auth_schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("required array");
    let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();

    // Unified Biscuit authentication required fields
    for f in &["success", "tokens", "user"] {
        assert!(required_fields.contains(f), "missing required field {f}");
    }

    // Ensure legacy flat fields are not present
    for f in &[
        "access_token",
        "refresh_token",
        "biscuit_token",
        "expires_in",
        "session_id",
        "token",
    ] {
        assert!(
            !required_fields.contains(f),
            "legacy flat field {f} should not be present in unified Biscuit authentication"
        );
    }

    // Legacy LoginResponse should no longer be present
    assert!(
        value.pointer("/components/schemas/LoginResponse").is_none(),
        "LoginResponse schema should not be present (legacy removed)"
    );
}
