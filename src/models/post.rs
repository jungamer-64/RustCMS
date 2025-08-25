use crate::database::schema::posts;
use crate::error::{AppError, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

lazy_static::lazy_static! {
    static ref SLUG_REGEX: Regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

impl PostStatus {
    pub fn parse_str(s: &str) -> Result<Self> {
        match s {
            "draft" => Ok(PostStatus::Draft),
            "published" => Ok(PostStatus::Published),
            "archived" => Ok(PostStatus::Archived),
            _ => Err(AppError::BadRequest(format!("Invalid post status: {}", s))),
        }
    }
}

impl Default for PostStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl std::fmt::Display for PostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostStatus::Draft => write!(f, "draft"),
            PostStatus::Published => write!(f, "published"),
            PostStatus::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, AsChangeset, ToSchema)]
#[diesel(table_name = posts)]
pub struct Post {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    pub author_id: Uuid,
    pub status: String, // This matches the schema varchar field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_image_id: Option<Uuid>,
    pub tags: Vec<String>,       // This matches the schema array field
    pub categories: Vec<String>, // This matches the schema array field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// For creating new posts
#[derive(Debug, Serialize, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = posts)]
pub struct NewPost {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub author_id: Uuid,
    pub status: String,
    pub featured_image_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePostRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title must be between 1 and 255 characters"
    ))]
    pub title: String,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: String,

    #[validate(length(max = 500, message = "Excerpt must be less than 500 characters"))]
    pub excerpt: Option<String>,

    #[validate(length(max = 255, message = "Slug must be less than 255 characters"))]
    pub slug: Option<String>,

    pub published: Option<bool>,

    #[validate(length(max = 20, message = "Too many tags"))]
    pub tags: Option<Vec<String>>,

    #[validate(length(max = 100, message = "Category name too long"))]
    pub category: Option<String>,

    #[validate(url(message = "Featured image must be a valid URL"))]
    pub featured_image: Option<String>,

    #[validate(length(max = 60, message = "Meta title must be less than 60 characters"))]
    pub meta_title: Option<String>,

    #[validate(length(
        max = 160,
        message = "Meta description must be less than 160 characters"
    ))]
    pub meta_description: Option<String>,

    pub published_at: Option<DateTime<Utc>>,

    pub status: Option<PostStatus>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePostRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title must be between 1 and 255 characters"
    ))]
    pub title: Option<String>,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: Option<String>,

    #[validate(length(max = 500, message = "Excerpt must be less than 500 characters"))]
    pub excerpt: Option<String>,

    #[validate(length(max = 255, message = "Slug must be less than 255 characters"))]
    pub slug: Option<String>,

    pub published: Option<bool>,

    #[validate(length(max = 20, message = "Too many tags"))]
    pub tags: Option<Vec<String>>,

    #[validate(length(max = 100, message = "Category name too long"))]
    pub category: Option<String>,

    #[validate(url(message = "Featured image must be a valid URL"))]
    pub featured_image: Option<String>,

    #[validate(length(max = 60, message = "Meta title must be less than 60 characters"))]
    pub meta_title: Option<String>,

    #[validate(length(
        max = 160,
        message = "Meta description must be less than 160 characters"
    ))]
    pub meta_description: Option<String>,

    pub published_at: Option<DateTime<Utc>>,

    pub status: Option<PostStatus>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostFilter {
    #[serde(default = "default_page")]
    pub page: usize,

    #[serde(default = "default_limit")]
    pub limit: usize,

    pub published: Option<bool>,
    pub author_id: Option<Uuid>,
    pub search: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    #[serde(default)]
    pub sort_by: PostSortBy,
    #[serde(default)]
    pub sort_order: SortOrder,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum PostSortBy {
    #[default]
    CreatedAt,
    UpdatedAt,
    PublishedAt,
    Title,
    ViewCount,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Desc,
    Asc,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PostResponse {
    pub posts: Vec<Post>,
    pub pagination: crate::models::pagination::PaginationInfo,
    pub filters: PostFilterMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PostFilterMeta {
    pub total_published: usize,
    pub total_drafts: usize,
    pub categories: Vec<String>,
    pub popular_tags: Vec<String>,
    pub date_range: Option<DateRange>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DateRange {
    pub earliest: DateTime<Utc>,
    pub latest: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PostSummary {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub author_id: Uuid,
    pub status: String,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Post> for PostSummary {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            title: post.title,
            slug: post.slug,
            author_id: post.author_id,
            status: post.status,
            published_at: post.published_at,
            created_at: post.created_at,
            updated_at: post.updated_at,
        }
    }
}

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    10
}

impl Post {
    /// Generate excerpt from content if not provided
    pub fn generate_excerpt(&self, length: usize) -> String {
        if let Some(excerpt) = &self.excerpt {
            excerpt.clone()
        } else {
            let content_text = strip_html(&self.content);
            if content_text.len() <= length {
                content_text
            } else {
                format!("{}...", &content_text[..length])
            }
        }
    }

    /// Check if post is published and publication date has passed
    pub fn is_publicly_visible(&self) -> bool {
        self.status == "published"
            && self
                .published_at
                .is_none_or(|pub_date| pub_date <= Utc::now())
    }

    /// Check if post is published
    pub fn is_published(&self) -> bool {
        self.status == "published"
    }

    /// Get reading time estimate in minutes
    pub fn reading_time(&self) -> u32 {
        let word_count = self.content.split_whitespace().count();
        ((word_count as f32 / 200.0).ceil() as u32).max(1) // Assuming 200 words per minute
    }

    /// Get post status enum for compatibility
    pub fn get_status(&self) -> Result<PostStatus> {
    PostStatus::parse_str(&self.status)
    }

    /// Get author ID as string (for compatibility with existing code)
    pub fn author_id_string(&self) -> String {
        self.author_id.to_string()
    }

    /// Check if post has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Check if post is in a specific category
    pub fn has_category(&self, category: &str) -> bool {
        self.categories.iter().any(|c| c == category)
    }
}

impl CreatePostRequest {
    /// Generate slug from title if not provided
    pub fn get_or_generate_slug(&self) -> String {
        self.slug
            .clone()
            .unwrap_or_else(|| generate_slug(&self.title))
    }

    /// Validate and clean tags
    pub fn clean_tags(&self) -> Vec<String> {
        self.tags
            .as_ref()
            .map(|tags| {
                tags.iter()
                    .filter_map(|tag| {
                        let cleaned = tag.trim().to_lowercase();
                        if cleaned.len() > 2 && cleaned.len() < 50 {
                            Some(cleaned)
                        } else {
                            None
                        }
                    })
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clean categories
    pub fn clean_categories(&self) -> Vec<String> {
        self.category
            .as_ref()
            .map(|cat| vec![cat.trim().to_lowercase()])
            .unwrap_or_default()
    }

    /// Convert to NewPost for database insertion
    pub fn into_new_post(self, author_id: Uuid) -> NewPost {
        let slug = self.get_or_generate_slug();
        let status = if self.published.unwrap_or(false) {
            "published".to_string()
        } else {
            "draft".to_string()
        };

        NewPost {
            id: Uuid::new_v4(),
            title: self.title.clone(),
            slug,
            content: self.content.clone(),
            excerpt: self.excerpt.clone(),
            author_id,
            status,
            featured_image_id: None, // Will be set separately if needed
            tags: self.clean_tags(),
            categories: self.clean_categories(),
            meta_title: self.meta_title.clone(),
            meta_description: self.meta_description.clone(),
            published_at: if self.published.unwrap_or(false) {
                Some(Utc::now())
            } else {
                None
            },
        }
    }
}

impl PostFilter {
    /// Validate and sanitize filter parameters
    pub fn validate_and_sanitize(&mut self) {
        // Ensure page is at least 1
        if self.page == 0 {
            self.page = 1;
        }

        // Limit page size for performance
        if self.limit == 0 || self.limit > 100 {
            self.limit = 10;
        }

        // Sanitize search query
        if let Some(search) = &self.search {
            if search.trim().is_empty() || search.len() > 200 {
                self.search = None;
            }
        }
    }

    /// Convert to SQL ORDER BY clause
    pub fn to_order_clause(&self) -> String {
        let column = match self.sort_by {
            PostSortBy::CreatedAt => "created_at",
            PostSortBy::UpdatedAt => "updated_at",
            PostSortBy::PublishedAt => "published_at",
            PostSortBy::Title => "title",
            PostSortBy::ViewCount => "view_count",
        };

        let direction = match self.sort_order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };

        format!("{} {}", column, direction)
    }
}

/// Generate URL-friendly slug from title
pub fn generate_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c)
            } else if c.is_whitespace() || c == '-' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Strip HTML tags from content (basic implementation)
fn strip_html(content: &str) -> String {
    // Basic HTML tag removal - in production you'd use a proper HTML parser
    let tag_regex = regex::Regex::new(r"<[^>]*>").unwrap();
    tag_regex
        .replace_all(content, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("Hello, World!"), "hello-world");
        assert_eq!(generate_slug("Multiple   Spaces"), "multiple-spaces");
        assert_eq!(generate_slug("Special@#$Characters"), "specialcharacters");
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
        assert_eq!(strip_html("No HTML here"), "No HTML here");
    }

    #[test]
    fn test_reading_time() {
        let post = Post {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            slug: "test".to_string(),
            content: "word ".repeat(400), // 400 words
            excerpt: None,
            author_id: Uuid::new_v4(),
            status: "published".to_string(),
            featured_image_id: None,
            tags: vec![],
            categories: vec![],
            meta_title: None,
            meta_description: None,
            published_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(post.reading_time(), 2); // 400 words / 200 words per minute = 2 minutes
    }
}
