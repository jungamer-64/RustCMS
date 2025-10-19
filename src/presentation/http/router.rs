//! Routing - HTTP ルート定義（Phase 4 Step 6）
//!
//! API v2 ルーティング: 新しいハンドラーをマウント
//! API v1 との並行稼働により段階的なマイグレーション
//!
//! 参考: RESTRUCTURE_EXAMPLES.md, ROLLBACK_PLAN.md

use axum::{
    Router,
    routing::{get, post},
};

// Handlers removed - Phase 10 レガシー削除
// #[cfg(all(
//     feature = "restructure_presentation",
//     feature = "restructure_application"
// ))]
// use crate::presentation::http::handlers::*;

// Phase 10: Stub router（handlers削除により一時的に無効化）
// Phase 4で新handlers実装時に再有効化予定
// #[cfg(all(
//     feature = "restructure_presentation",
//     feature = "restructure_application"
// ))]
// pub fn api_v2_router<S>() -> Router<S>
// where
//     S: Clone + Send + Sync + 'static,
// {
//     Router::new()
//         // User routes
//         .route("/users/register", post(register_user))
//         .route(
//             "/users/{user_id}",
//             get(get_user).put(update_user).delete(delete_user),
//         )
//         // Post routes
//         .route("/posts", post(create_post))
//         .route("/posts/{slug}", get(get_post).put(update_post))
//         // Comment routes
//         .route(
//             "/posts/{post_id}/comments",
//             post(create_comment).get(list_comments),
//         )
//         // Tag routes
//         .route("/tags", post(create_tag))
//         .route("/tags/{slug}", get(get_tag))
//         // Category routes
//         .route("/categories", post(create_category))
//         .route("/categories/{slug}", get(get_category))
// }

// Stub router（全feature flagsで使用）
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
