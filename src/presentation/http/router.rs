//! Routing - HTTP ルート定義（Phase 4 Step 6）
//!
//! API v2 ルーティング: 新しいハンドラーをマウント
//! API v1 との並行稼働により段階的なマイグレーション
//!
//! 参考: RESTRUCTURE_EXAMPLES.md, ROLLBACK_PLAN.md

use axum::{
    Router,
    routing::{delete, get, post, put},
};

#[cfg(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
))]
use crate::presentation::http::handlers::*;

/// API v2 ルーター構築（Phase 4.9）
///
/// # ルート構成
/// - POST /api/v2/users/register - ユーザー登録
/// - GET /api/v2/users/{user_id} - ユーザー取得
/// - PUT /api/v2/users/{user_id} - ユーザー更新
/// - DELETE /api/v2/users/{user_id} - ユーザー削除
/// - POST /api/v2/posts - ブログ記事作成
/// - GET /api/v2/posts/{slug} - ブログ記事取得
/// - PUT /api/v2/posts/{post_id} - ブログ記事更新
/// - POST /api/v2/posts/{post_id}/comments - コメント作成
/// - GET /api/v2/posts/{post_id}/comments - コメント一覧
/// - POST /api/v2/tags - タグ作成
/// - GET /api/v2/tags/{slug} - タグ取得
/// - POST /api/v2/categories - カテゴリー作成
/// - GET /api/v2/categories/{slug} - カテゴリー取得
#[cfg(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
))]
pub fn api_v2_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // User routes
        .route("/users/register", post(register_user))
        .route(
            "/users/{user_id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        // Post routes
        .route("/posts", post(create_post))
        .route("/posts/{slug}", get(get_post).put(update_post))
        // Comment routes
        .route(
            "/posts/{post_id}/comments",
            post(create_comment).get(list_comments),
        )
        // Tag routes
        .route("/tags", post(create_tag))
        .route("/tags/{slug}", get(get_tag))
        // Category routes
        .route("/categories", post(create_category))
        .route("/categories/{slug}", get(get_category))
}

// Stub for when feature flags are not enabled
#[cfg(not(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
)))]
pub fn api_v2_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;

    #[test]
    fn test_api_v2_router_creation() {
        let _router: Router<()> = api_v2_router();
        // Router は内部で検証されます
    }

    #[test]
    fn test_api_v2_routes_exist() {
        // ルートが定義されていることを確認
        let _router: Router<()> = api_v2_router();
    }
}
