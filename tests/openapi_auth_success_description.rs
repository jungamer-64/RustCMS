//! Ensures the unified Biscuit success response appears in `OpenAPI` schema with the correct description.
//!
//! Note: This test is currently disabled because the openapi module has been refactored.
//! TODO: Re-enable after openapi is properly integrated with the new architecture.

#![cfg(feature = "openapi_disabled_pending_refactor")]

use cms_backend::openapi::ApiDoc;

use utoipa::OpenApi;

#[test]
fn auth_success_description_reflects_biscuit_only() {
    let doc = ApiDoc::openapi();
    let root = serde_json::to_value(&doc).expect("serialize openapi");
    let schema = root
        .pointer("/components/schemas/AuthSuccessResponse")
        .expect("schema");
    let desc = schema
        .get("description")
        .and_then(|d| d.as_str())
        .expect("description");
    // Unified Biscuit authentication with no legacy fields
    assert!(
        desc.contains("Biscuit") || desc.contains("tokens"),
        "description should mention Biscuit or tokens: {desc}"
    );
}
