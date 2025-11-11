// tests/integration_repositories_phase3.rs
// Phase 3 Repository Integration Tests (Restored in Phase 5)

#![cfg(all(test, feature = "database", feature = "restructure_domain"))]

mod helpers;

use cms_backend::application::ports::repositories::{
    CommentRepository, PostRepository, UserRepository,
};
use cms_backend::domain::comment::{Comment, CommentText};
use cms_backend::domain::post::{Content, Post, PostStatus, Slug, Title};
use cms_backend::domain::user::{Email, User, UserId, Username};
use cms_backend::infrastructure::database::repositories::{
    DieselCommentRepository, DieselPostRepository, DieselUserRepository,
};
use helpers::integration::{cleanup_database, setup_test_database};
use serial_test::serial;

// ============================================================================
// Test Helpers (Phase 5: Value Object API)
// ============================================================================

fn create_test_user(username: &str, email: &str) -> User {
    let username = Username::new(username.to_string()).expect("Invalid username");
    let email = Email::new(email.to_string()).expect("Invalid email");
    User::new(username, email)
}

fn create_test_post(author_id: UserId, title: &str, slug: &str, content: &str) -> Post {
    let title = Title::new(title.to_string()).expect("Invalid title");
    let slug = Slug::new(slug.to_string()).expect("Invalid slug");
    let content = Content::new(content.to_string()).expect("Invalid content");
    Post::new(author_id, title, slug, content)
}

fn create_test_comment(
    post_id: cms_backend::domain::post::PostId,
    author_id: UserId,
    text: &str,
) -> Comment {
    let text = CommentText::new(text.to_string()).expect("Invalid comment text");
    Comment::new(post_id, author_id, text).expect("Failed to create comment")
}

// ============================================================================
// User Repository Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_user_save_and_find_by_id() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    let user = create_test_user("testuser", "test@example.com");
    let user_id = user.id();

    repo.save(user.clone()).await.expect("Failed to save user");

    let found = repo.find_by_id(user_id).await.expect("Failed to find user");

    assert!(found.is_some());
    let found_user = found.unwrap();
    assert_eq!(found_user.id(), user_id);
    assert_eq!(found_user.username().as_str(), "testuser");
    assert_eq!(found_user.email().as_str(), "test@example.com");

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_user_find_by_email() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    let user = create_test_user("emailuser", "email@test.com");
    repo.save(user.clone()).await.expect("Failed to save user");

    let email = Email::new("email@test.com".to_string()).expect("Invalid email");
    let found = repo
        .find_by_email(&email)
        .await
        .expect("Failed to find user by email");

    assert!(found.is_some());
    let found_user = found.unwrap();
    assert_eq!(found_user.email().as_str(), "email@test.com");
    assert_eq!(found_user.username().as_str(), "emailuser");

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_user_list_all() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    let user1 = create_test_user("user1", "user1@test.com");
    let user2 = create_test_user("user2", "user2@test.com");
    let user3 = create_test_user("user3", "user3@test.com");

    repo.save(user1).await.expect("Failed to save user1");
    repo.save(user2).await.expect("Failed to save user2");
    repo.save(user3).await.expect("Failed to save user3");

    let users = repo.list_all(100, 0).await.expect("Failed to list users");

    assert_eq!(users.len(), 3);
    assert!(users.iter().any(|u| u.username().as_str() == "user1"));
    assert!(users.iter().any(|u| u.username().as_str() == "user2"));
    assert!(users.iter().any(|u| u.username().as_str() == "user3"));

    cleanup_database(&*pool);
}

// ============================================================================
// Post Repository Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_post_save_and_find_by_id() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());

    let author = create_test_user("author", "author@test.com");
    let author_id = author.id();
    user_repo.save(author).await.expect("Failed to save author");

    let post = create_test_post(author_id, "Test Post", "test-post", "Content");
    let post_id = post.id();

    post_repo.save(post).await.expect("Failed to save post");

    let found = post_repo.find_by_id(post_id).await.expect("Failed to find post");

    assert!(found.is_some());
    let found_post = found.unwrap();
    assert_eq!(found_post.id(), post_id);
    assert_eq!(found_post.title().as_str(), "Test Post");
    assert_eq!(found_post.slug().as_str(), "test-post");
    assert_eq!(found_post.status(), PostStatus::Draft);

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_post_find_by_slug() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());

    let author = create_test_user("slugauthor", "slugauthor@test.com");
    let author_id = author.id();
    user_repo.save(author).await.expect("Failed to save author");

    let post = create_test_post(author_id, "Slug Test", "unique-slug", "Content");
    let slug_str = post.slug().as_str().to_string();

    post_repo.save(post).await.expect("Failed to save post");

    let found = post_repo
        .find_by_slug(&slug_str)
        .await
        .expect("Failed to find post by slug");

    assert!(found.is_some());
    let found_post = found.unwrap();
    assert_eq!(found_post.slug().as_str(), "unique-slug");
    assert_eq!(found_post.title().as_str(), "Slug Test");

    cleanup_database(&*pool);
}

// ============================================================================
// Comment Repository Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_comment_save_and_find_by_id() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());
    let comment_repo = DieselCommentRepository::new(pool.clone());

    let user = create_test_user("commenter", "commenter@test.com");
    let user_id = user.id();
    user_repo.save(user).await.expect("Failed to save user");

    let post = create_test_post(user_id, "Post", "post", "Content");
    let post_id = post.id();
    post_repo.save(post).await.expect("Failed to save post");

    let comment = create_test_comment(post_id, user_id, "Test comment");
    let comment_id = comment.id();

    comment_repo.save(comment).await.expect("Failed to save comment");

    let found = comment_repo
        .find_by_id(comment_id)
        .await
        .expect("Failed to find comment");

    assert!(found.is_some());
    let found_comment = found.unwrap();
    assert_eq!(found_comment.id(), comment_id);
    assert_eq!(found_comment.text().as_str(), "Test comment");

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_comment_find_by_post() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());
    let comment_repo = DieselCommentRepository::new(pool.clone());

    let user = create_test_user("postcommenter", "postcommenter@test.com");
    let user_id = user.id();
    user_repo.save(user).await.expect("Failed to save user");

    let post = create_test_post(user_id, "Comment Post", "comment-post", "Content");
    let post_id = post.id();
    post_repo.save(post).await.expect("Failed to save post");

    let comment1 = create_test_comment(post_id, user_id, "First comment");
    let comment2 = create_test_comment(post_id, user_id, "Second comment");

    comment_repo.save(comment1).await.expect("Failed to save comment1");
    comment_repo.save(comment2).await.expect("Failed to save comment2");

    let comments = comment_repo
        .find_by_post(post_id, 100, 0)
        .await
        .expect("Failed to find comments");

    assert_eq!(comments.len(), 2);
    assert!(comments.iter().any(|c| c.text().as_str() == "First comment"));
    assert!(comments.iter().any(|c| c.text().as_str() == "Second comment"));

    cleanup_database(&*pool);
}
