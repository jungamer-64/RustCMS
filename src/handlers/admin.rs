use axum::{extract::State, http::StatusCode, Json};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::Deserialize;

use crate::utils::{api_types::ApiResponse, auth_utils, common_types::PostSummary};
use crate::{AppState, Result};

pub async fn list_posts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<PostSummary>>>> {
    // Simple header auth
    if let Some(val) = headers.get("x-admin-token") {
        if !auth_utils::check_admin_token(val.to_str().unwrap_or("")) {
            return Err(crate::AppError::Authentication("invalid token".to_string()));
        }
    } else {
        return Err(crate::AppError::Authentication("missing token".to_string()));
    }

    let mut conn = state.get_conn()?;

    #[derive(diesel::QueryableByName, Debug)]
    struct PostRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        title: String,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        author_id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        status: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<PostRow> = diesel::sql_query("SELECT id, title, author_id, status, created_at FROM posts ORDER BY created_at DESC LIMIT 100")
        .load(&mut conn)
    .map_err(crate::AppError::Database)?;

    let out = rows
        .into_iter()
        .map(|r| PostSummary {
            id: r.id.to_string(),
            title: r.title,
            author_id: r.author_id.to_string(),
            status: r.status,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(out)))
}

#[derive(Deserialize)]
pub struct CreatePostBody {
    pub title: String,
    pub content: String,
    pub published: Option<bool>,
}

pub async fn create_post(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreatePostBody>,
) -> Result<(StatusCode, Json<ApiResponse<PostSummary>>)> {
    if let Some(val) = headers.get("x-admin-token") {
        if !auth_utils::check_admin_token(val.to_str().unwrap_or("")) {
            return Err(crate::AppError::Authentication("invalid token".to_string()));
        }
    } else {
        return Err(crate::AppError::Authentication("missing token".to_string()));
    }

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

    Ok((StatusCode::CREATED, Json(ApiResponse::success(out))))
}

pub async fn delete_post(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode> {
    if let Some(val) = headers.get("x-admin-token") {
        if !auth_utils::check_admin_token(val.to_str().unwrap_or("")) {
            return Err(crate::AppError::Authentication("invalid token".to_string()));
        }
    } else {
        return Err(crate::AppError::Authentication("missing token".to_string()));
    }

    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| crate::AppError::BadRequest("invalid uuid".to_string()))?;
    let mut conn = state.get_conn()?;

    use crate::database::schema::posts::dsl as posts_dsl;
    let deleted = diesel::delete(posts_dsl::posts.filter(posts_dsl::id.eq(uuid)))
        .execute(&mut conn)
    .map_err(crate::AppError::Database)?;

    if deleted == 0 {
        return Err(crate::AppError::NotFound("post not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}
