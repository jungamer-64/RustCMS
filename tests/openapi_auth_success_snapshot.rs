//! Focused snapshot of only the AuthSuccessResponse schema to reduce diff noise when unrelated
//! endpoints evolve. Guards structural stability of unified auth response.
use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_auth_success_schema_snapshot() {
    let doc = ApiDoc::openapi();
    let root = serde_json::to_value(&doc).expect("serialize openapi");
    let schema = root
        .pointer("/components/schemas/AuthSuccessResponse")
        .expect("AuthSuccessResponse schema present");

    // Assert deprecated flattened fields still exist pre-3.0.0 so we notice accidental early removal.
    // (They are marked #[deprecated]; removal happens in Phase 4.)
    for key in [
        "access_token",
        "refresh_token",
        "biscuit_token",
        "expires_in",
        "session_id",
        "token",
    ] {
        assert!(schema.get("properties").and_then(|p| p.get(key)).is_some(), "expected deprecated field `{key}` to remain until 3.0.0 phase removal");
    }

    insta::assert_json_snapshot!("openapi_auth_success_schema", schema);
}
