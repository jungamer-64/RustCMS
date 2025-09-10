use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
// diesel traits are not needed here anymore; all DB ops go through AppState wrappers
use serde::Deserialize;

use crate::auth::{AuthContext, require_admin_permission};
use crate::utils::common_types::PostSummary;
use crate::utils::crud;
use crate::utils::response_ext::{ApiOk, delete_with};
use crate::{AppState, Result};

pub async fn list_posts(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse> {
    // Check admin permissions (Biscuit-based authorization)
    require_admin_permission(&auth)?;

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
    require_admin_permission(&auth)?;
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
    let (status, api_ok) = crud::create_entity(
        state.clone(),
        req,
        |st, r| async move { st.db_create_post(r).await },
        |p: &crate::models::post::Post| PostSummary {
            id: p.id.to_string(),
            title: p.title.clone(),
            author_id: p.author_id.to_string(),
            status: p.status.clone(),
            created_at: p.created_at.to_rfc3339(),
        },
        Some(|_p: &crate::models::post::Post, _st: AppState| async move {}),
    )
    .await?;
    Ok((status, api_ok))
}

pub async fn delete_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    require_admin_permission(&auth)?;
    let fut = async move { state.db_admin_delete_post(id).await.map(|_| ()) };
    // Reuse delete_with for consistent JSON payload (admin previously returned 204)
    delete_with(fut, "Post deleted successfully").await
}
