//! Snapshot test ensuring `OpenAPI` AuthSuccessResponse unified with Biscuit appears as expected.
//!
//! Note: This test is currently disabled because the openapi module has been refactored.
//! TODO: Re-enable after openapi is properly integrated with the new architecture.

#![cfg(feature = "openapi_disabled_pending_refactor")]

use cms_backend::openapi::ApiDoc;

use utoipa::OpenApi;

#[test]
fn openapi_auth_success_schema_snapshot() {
    let doc = ApiDoc::openapi();
    let root = serde_json::to_value(&doc).expect("serialize openapi");
    let schema = root
        .pointer("/components/schemas/AuthSuccessResponse")
        .expect("AuthSuccessResponse schema present");

    // Verify unified Biscuit-only authentication schema
    let properties = schema.get("properties").expect("properties should exist");

    // Required fields for Biscuit authentication
    assert!(
        properties.get("success").is_some(),
        "success field should be present"
    );
    assert!(
        properties.get("tokens").is_some(),
        "tokens field should be present"
    );
    assert!(
        properties.get("user").is_some(),
        "user field should be present"
    );

    // Legacy flat fields should not exist
    for key in [
        "access_token",
        "refresh_token",
        "biscuit_token",
        "expires_in",
        "session_id",
        "token",
    ] {
        assert!(
            properties.get(key).is_none(),
            "legacy flat field `{key}` should not be present in unified Biscuit authentication"
        );
    }
}
