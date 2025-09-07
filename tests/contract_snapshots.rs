use cms_backend::utils::api_types::{ApiResponse, ValidationError};
use serde_json::json;
use insta::assert_json_snapshot;

// 初期スナップショット: 統一レスポンス構造 (成功)
#[test]
fn snapshot_api_success_basic() {
    let resp = ApiResponse::success(json!({"foo":"bar","num":123}));
    assert_json_snapshot!("api_success_basic", resp);
    assert!(resp.success);
}

// メッセージ付き成功 (回帰検知用)
#[test]
fn snapshot_api_success_with_message() {
    let resp = ApiResponse::success_with_message(json!({"ok":true}), "done".to_string());
    assert_json_snapshot!("api_success_with_message", resp);
    assert!(resp.success);
}

// エラーパターン (バリデーションなし)
#[test]
fn snapshot_api_error_basic() {
    let resp = ApiResponse::error("something went wrong".to_string());
    assert_json_snapshot!("api_error_basic", resp);
    assert!(!resp.success);
}

// バリデーションエラー付き
#[test]
fn snapshot_api_error_with_validation() {
    let resp = ApiResponse::error_with_validation(
        "validation failed".to_string(),
        vec![
            ValidationError { field: "email".into(), message: "invalid format".into() },
            ValidationError { field: "password".into(), message: "too short".into() },
        ]
    );
    assert_json_snapshot!("api_error_with_validation", resp);
    assert!(!resp.success);
}
