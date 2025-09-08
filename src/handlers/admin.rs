use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Extension};
// diesel traits are not needed here anymore; all DB ops go through AppState wrappers
use serde::Deserialize;

use crate::auth::AuthContext;
use crate::models::UserRole;
use crate::utils::{common_types::PostSummary};
use crate::utils::response_ext::ApiOk;
use crate::{AppState, Result, AppError};

/// Check if user has admin permissions
fn require_admin(auth: &AuthContext) -> Result<()> {
    match auth.role {
        UserRole::Admin | UserRole::SuperAdmin => Ok(()),
        _ => Err(AppError::Authorization("Admin access required".to_string())),
    }
}

pub async fn list_posts(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse> {
    // Verify admin role
    require_admin(&auth)?;

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
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreatePostBody>,
) -> Result<(StatusCode, impl IntoResponse)> {
    // Verify admin role
    require_admin(&auth)?;

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
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode> {
    // Verify admin role
    require_admin(&auth)?;

    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| crate::AppError::BadRequest("invalid uuid".to_string()))?;
    state.db_admin_delete_post(uuid).await?;
    Ok(StatusCode::NO_CONTENT)
}
