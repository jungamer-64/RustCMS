use axum::{extract::State, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use diesel::{RunQueryDsl, QueryDsl, ExpressionMethods};

use crate::{AppState, Result};

fn check_token(req_token: &str) -> bool {
    std::env::var("ADMIN_TOKEN").map(|t| t == req_token).unwrap_or(false)
}

#[derive(Serialize)]
pub struct PostSummary {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub status: String,
    pub created_at: String,
}

pub async fn list_posts(State(state): State<AppState>, headers: axum::http::HeaderMap) -> Result<Json<Vec<PostSummary>>> {
    // Simple header auth
    if let Some(val) = headers.get("x-admin-token") {
        if !check_token(val.to_str().unwrap_or("")) {
            return Err(crate::AppError::Authentication("invalid token".to_string()));
        }
    } else {
        return Err(crate::AppError::Authentication("missing token".to_string()));
    }

    let db = &state.database;
    let mut conn = db.get_connection()?;

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
        .map_err(|e| crate::AppError::Database(e))?;

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

    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct CreatePostBody {
    pub title: String,
    pub content: String,
    pub published: Option<bool>,
}

pub async fn create_post(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<CreatePostBody>) -> Result<(StatusCode, Json<PostSummary>)> {
    if let Some(val) = headers.get("x-admin-token") {
        if !check_token(val.to_str().unwrap_or("")) {
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

    let db = &state.database;
    let post = db.create_post(req).await?;

    let out = PostSummary {
        id: post.id.to_string(),
        title: post.title,
        author_id: post.author_id.to_string(),
        status: post.status,
        created_at: post.created_at.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(out)))
}

pub async fn delete_post(State(state): State<AppState>, headers: axum::http::HeaderMap, axum::extract::Path(id): axum::extract::Path<String>) -> Result<StatusCode> {
    if let Some(val) = headers.get("x-admin-token") {
        if !check_token(val.to_str().unwrap_or("")) {
            return Err(crate::AppError::Authentication("invalid token".to_string()));
        }
    } else {
        return Err(crate::AppError::Authentication("missing token".to_string()));
    }

    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| crate::AppError::BadRequest("invalid uuid".to_string()))?;
    let db = &state.database;
    let mut conn = db.get_connection()?;

    use crate::database::schema::posts::dsl as posts_dsl;
    let deleted = diesel::delete(posts_dsl::posts.filter(posts_dsl::id.eq(uuid)))
        .execute(&mut conn)
        .map_err(|e| crate::AppError::Database(e))?;

    if deleted == 0 {
        return Err(crate::AppError::NotFound("post not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}
