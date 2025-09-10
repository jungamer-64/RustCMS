//! Validates that AuthSuccessResponse schema description matches feature state.
use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn auth_success_description_reflects_feature_state() {
    let doc = ApiDoc::openapi();
    let root = serde_json::to_value(&doc).expect("serialize openapi");
    let schema = root
        .pointer("/components/schemas/AuthSuccessResponse")
        .expect("schema");
    let desc = schema
        .get("description")
        .and_then(|d| d.as_str())
        .expect("description");
    #[cfg(feature = "auth-flat-fields")]
    assert!(
        desc.contains("フラットなフィールド"),
        "expected description to mention flattened fields when feature enabled: {desc}"
    );
    #[cfg(not(feature = "auth-flat-fields"))]
    assert!(
        !desc.contains("フラットなフィールド"),
        "description should not mention flattened fields when feature disabled: {desc}"
    );
}
