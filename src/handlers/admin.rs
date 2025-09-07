use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
// diesel traits are not needed here anymore; all DB ops go through AppState wrappers
use serde::Deserialize;

use crate::utils::{common_types::PostSummary};
use crate::utils::response_ext::ApiOk;
use crate::{AppState, Result};

pub async fn list_posts(
    State(state): State<AppState>,
) -> Result<impl IntoResponse> {
    // Admin authentication is enforced by middleware at /api/v1/admin

    let out: Vec<PostSummary> = state.db_admin_list_recent_posts(100).await?;

    Ok(ApiOk(out))
}

#[derive(Deserialize)]
pub struct CreatePostBody {
    pub title: String,
    pub content: String,
    pub published: Option<bool>,
}

pub async fn create_post(
    State(state): State<AppState>,
    Json(payload): Json<CreatePostBody>,
) -> Result<(StatusCode, impl IntoResponse)> {
    // Admin authentication is enforced by middleware at /api/v1/admin

    // Build CreatePostRequest
    let req = crate::models::post::CreatePostRequest {
        title: payload.title,
        content: payload.content,
        excerpt: None,
        slug: None,
        published: payload.published,
        tags: None,
        category: None,
        featured_image: None,
        meta_title: None,
        meta_description: None,
        published_at: None,
        status: None,
    };

    let post = state.db_create_post(req).await?;

    let out = PostSummary {
        id: post.id.to_string(),
        title: post.title,
        author_id: post.author_id.to_string(),
        status: post.status,
        created_at: post.created_at.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, ApiOk(out)))
}

pub async fn delete_post(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode> {
    // Admin authentication is enforced by middleware at /api/v1/admin

    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| crate::AppError::BadRequest("invalid uuid".to_string()))?;
    state.db_admin_delete_post(uuid).await?;
    Ok(StatusCode::NO_CONTENT)
}
