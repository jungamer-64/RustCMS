//! Diesel database models for infrastructure layer

use chrono::{DateTime, Utc};
use uuid::Uuid;

// User model
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::users)]
pub struct DbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::users)]
pub struct NewDbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Post model
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::posts)]
pub struct DbPost {
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::posts)]
pub struct NewDbPost {
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Comment model
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::comments)]
pub struct DbComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::comments)]
pub struct NewDbComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Category model
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::categories)]
pub struct DbCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub post_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::categories)]
pub struct NewDbCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub post_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Tag model
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::tags)]
pub struct DbTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::tags)]
pub struct NewDbTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_models_creation() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let user = DbUser {
            id: user_id,
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            password_hash: Some("hash".to_string()),
            first_name: Some("first".to_string()),
            last_name: Some("last".to_string()),
            role: "user".to_string(),
            is_active: true,
            email_verified: false,
            last_login: Some(now),
            created_at: now,
            updated_at: now,
        };
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "test");

        let post_id = Uuid::new_v4();
        let post = DbPost {
            id: post_id,
            title: "Test".to_string(),
            slug: "test".to_string(),
            content: "Content".to_string(),
            excerpt: Some("excerpt".to_string()),
            author_id: user_id,
            status: "draft".to_string(),
            featured_image_id: Some(Uuid::new_v4()),
            tags: vec!["tag1".to_string()],
            categories: vec!["cat1".to_string()],
            meta_title: Some("meta".to_string()),
            meta_description: Some("desc".to_string()),
            published_at: Some(now),
            created_at: now,
            updated_at: now,
        };
        assert_eq!(post.id, post_id);
        assert_eq!(post.title, "Test");

        let comment_id = Uuid::new_v4();
        let comment = DbComment {
            id: comment_id,
            post_id,
            author_id: user_id,
            content: "comment".to_string(),
            parent_id: None,
            is_approved: true,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(comment.id, comment_id);
        assert_eq!(comment.content, "comment");

        let category_id = Uuid::new_v4();
        let category = DbCategory {
            id: category_id,
            name: "Category".to_string(),
            slug: "category".to_string(),
            description: Some("desc".to_string()),
            is_active: true,
            post_count: 1,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(category.id, category_id);
        assert_eq!(category.name, "Category");

        let tag_id = Uuid::new_v4();
        let tag = DbTag {
            id: tag_id,
            name: "Tag".to_string(),
            slug: "tag".to_string(),
            usage_count: 1,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(tag.id, tag_id);
        assert_eq!(tag.name, "Tag");
    }

    #[test]
    fn test_new_db_models_creation() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let new_user = NewDbUser {
            id: user_id,
            username: "new_test".to_string(),
            email: "new_test@example.com".to_string(),
            password_hash: Some("new_hash".to_string()),
            first_name: Some("new_first".to_string()),
            last_name: Some("new_last".to_string()),
            role: "admin".to_string(),
            is_active: false,
            email_verified: true,
            last_login: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(new_user.id, user_id);
        assert_eq!(new_user.username, "new_test");

        let post_id = Uuid::new_v4();
        let new_post = NewDbPost {
            id: post_id,
            title: "New Post".to_string(),
            slug: "new-post".to_string(),
            content: "New Content".to_string(),
            excerpt: None,
            author_id: user_id,
            status: "published".to_string(),
            featured_image_id: None,
            tags: vec![],
            categories: vec![],
            meta_title: None,
            meta_description: None,
            published_at: Some(now),
            created_at: now,
            updated_at: now,
        };
        assert_eq!(new_post.id, post_id);
        assert_eq!(new_post.title, "New Post");

        let comment_id = Uuid::new_v4();
        let new_comment = NewDbComment {
            id: comment_id,
            post_id,
            author_id: user_id,
            content: "new comment".to_string(),
            parent_id: Some(Uuid::new_v4()),
            is_approved: false,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(new_comment.id, comment_id);
        assert_eq!(new_comment.content, "new comment");

        let category_id = Uuid::new_v4();
        let new_category = NewDbCategory {
            id: category_id,
            name: "New Category".to_string(),
            slug: "new-category".to_string(),
            description: None,
            is_active: false,
            post_count: 0,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(new_category.id, category_id);
        assert_eq!(new_category.name, "New Category");

        let tag_id = Uuid::new_v4();
        let new_tag = NewDbTag {
            id: tag_id,
            name: "New Tag".to_string(),
            slug: "new-tag".to_string(),
            usage_count: 0,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(new_tag.id, tag_id);
        assert_eq!(new_tag.name, "New Tag");
    }
}
