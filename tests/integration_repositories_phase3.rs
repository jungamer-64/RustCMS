// tests/integration_repositories_phase3.rs
// Phase 3 Repository 統合テスト
//
// このファイルは PostgreSQL データベースに対する Repository 実装の統合テストを提供します。
// テストは実際のデータベースに接続し、CRUD 操作、トランザクション、並行アクセスを検証します。
//
// # 実行要件
// - PostgreSQL が localhost:5432 で起動していること
// - データベース 'cms_test' が存在すること
// - または環境変数 TEST_DATABASE_URL でカスタム接続先を指定
//
// # 実行方法
// ```bash
// # PostgreSQL起動（Docker使用例）
// docker run -d --name postgres-test -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=cms_test -p 5432:5432 postgres:16
//
// # テスト実行
// cargo test --test integration_repositories_phase3 --features "restructure_domain database" -- --test-threads=1
// ```

#![cfg(all(test, feature = "database", feature = "restructure_domain"))]

mod helpers;

use cms_backend::application::ports::repositories::{
    CommentRepository, PostRepository, UserRepository,
};
use cms_backend::domain::comment::Comment;
use cms_backend::domain::post::{Post, PostStatus};
use cms_backend::domain::user::User;
use cms_backend::infrastructure::database::repositories::{
    DieselCommentRepository, DieselPostRepository, DieselUserRepository,
};
use helpers::integration::{cleanup_database, setup_test_database};
use serial_test::serial;

// ============================================================================
// User Repository 統合テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn test_user_repository_save_and_find_by_id() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    // テストユーザー作成
    let user = User::new("testuser".to_string(), "test@example.com".to_string(), "hashedpw")
        .expect("Failed to create user");
    let user_id = user.id();

    // 保存
    repo.save(user.clone())
        .await
        .expect("Failed to save user");

    // ID で検索
    let found = repo
        .find_by_id(user_id)
        .await
        .expect("Failed to find user");

    assert!(found.is_some());
    let found_user = found.unwrap();
    assert_eq!(found_user.id(), user_id);
    assert_eq!(found_user.username().as_str(), "testuser");
    assert_eq!(found_user.email().as_str(), "test@example.com");

    // クリーンアップ
    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_user_repository_find_by_email() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    // テストユーザー作成
    let user = User::new(
        "emailuser".to_string(),
        "email@test.com".to_string(),
        "hashedpw",
    )
    .expect("Failed to create user");

    repo.save(user.clone())
        .await
        .expect("Failed to save user");

    // Email で検索
    let email = cms_backend::domain::user::Email::new("email@test.com".to_string())
        .expect("Invalid email");
    let found = repo
        .find_by_email(email)
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
async fn test_user_repository_list_all() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    // 複数ユーザー作成
    let user1 = User::new("user1".to_string(), "user1@test.com".to_string(), "pw1")
        .expect("Failed to create user1");
    let user2 = User::new("user2".to_string(), "user2@test.com".to_string(), "pw2")
        .expect("Failed to create user2");
    let user3 = User::new("user3".to_string(), "user3@test.com".to_string(), "pw3")
        .expect("Failed to create user3");

    repo.save(user1).await.expect("Failed to save user1");
    repo.save(user2).await.expect("Failed to save user2");
    repo.save(user3).await.expect("Failed to save user3");

    // 全件取得
    let users = repo.list_all().await.expect("Failed to list all users");

    assert_eq!(users.len(), 3);
    assert!(users.iter().any(|u| u.username().as_str() == "user1"));
    assert!(users.iter().any(|u| u.username().as_str() == "user2"));
    assert!(users.iter().any(|u| u.username().as_str() == "user3"));

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_user_repository_delete() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    // テストユーザー作成
    let user = User::new(
        "deleteuser".to_string(),
        "delete@test.com".to_string(),
        "hashedpw",
    )
    .expect("Failed to create user");
    let user_id = user.id();

    repo.save(user).await.expect("Failed to save user");

    // 削除
    repo.delete(user_id).await.expect("Failed to delete user");

    // 削除確認
    let found = repo
        .find_by_id(user_id)
        .await
        .expect("Failed to find user");
    assert!(found.is_none());

    cleanup_database(&*pool);
}

// ============================================================================
// Post Repository 統合テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn test_post_repository_save_and_find_by_id() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());

    // 著者ユーザー作成
    let author = User::new("author".to_string(), "author@test.com".to_string(), "pw")
        .expect("Failed to create author");
    let author_id = author.id();
    user_repo
        .save(author)
        .await
        .expect("Failed to save author");

    // 投稿作成
    let post = Post::new(
        "Test Post".to_string(),
        "test-post".to_string(),
        "This is a test post content.".to_string(),
        author_id,
    )
    .expect("Failed to create post");
    let post_id = post.id();

    // 保存
    post_repo.save(post).await.expect("Failed to save post");

    // ID で検索
    let found = post_repo
        .find_by_id(post_id)
        .await
        .expect("Failed to find post");

    assert!(found.is_some());
    let found_post = found.unwrap();
    assert_eq!(found_post.id(), post_id);
    assert_eq!(found_post.title().as_str(), "Test Post");
    assert_eq!(found_post.slug().as_str(), "test-post");
    assert_eq!(found_post.status(), &PostStatus::Draft);

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_post_repository_find_by_slug() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());

    // 著者作成
    let author = User::new(
        "slugauthor".to_string(),
        "slugauthor@test.com".to_string(),
        "pw",
    )
    .expect("Failed to create author");
    user_repo
        .save(author.clone())
        .await
        .expect("Failed to save author");

    // 投稿作成
    let post = Post::new(
        "Slug Test".to_string(),
        "unique-slug".to_string(),
        "Content".to_string(),
        author.id(),
    )
    .expect("Failed to create post");
    post_repo.save(post).await.expect("Failed to save post");

    // Slug で検索
    let slug = cms_backend::domain::post::Slug::new("unique-slug".to_string())
        .expect("Invalid slug");
    let found = post_repo
        .find_by_slug(slug)
        .await
        .expect("Failed to find post by slug");

    assert!(found.is_some());
    let found_post = found.unwrap();
    assert_eq!(found_post.slug().as_str(), "unique-slug");
    assert_eq!(found_post.title().as_str(), "Slug Test");

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_post_repository_list_all() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());

    // 著者作成
    let author = User::new(
        "listauthor".to_string(),
        "listauthor@test.com".to_string(),
        "pw",
    )
    .expect("Failed to create author");
    user_repo
        .save(author.clone())
        .await
        .expect("Failed to save author");

    // 複数投稿作成
    let post1 = Post::new(
        "Post 1".to_string(),
        "post-1".to_string(),
        "Content 1".to_string(),
        author.id(),
    )
    .expect("Failed to create post1");
    let post2 = Post::new(
        "Post 2".to_string(),
        "post-2".to_string(),
        "Content 2".to_string(),
        author.id(),
    )
    .expect("Failed to create post2");

    post_repo.save(post1).await.expect("Failed to save post1");
    post_repo.save(post2).await.expect("Failed to save post2");

    // 全件取得
    let posts = post_repo.list_all().await.expect("Failed to list all posts");

    assert_eq!(posts.len(), 2);
    assert!(posts.iter().any(|p| p.title().as_str() == "Post 1"));
    assert!(posts.iter().any(|p| p.title().as_str() == "Post 2"));

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_post_repository_delete() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());

    // 著者作成
    let author = User::new(
        "deleteauthor".to_string(),
        "deleteauthor@test.com".to_string(),
        "pw",
    )
    .expect("Failed to create author");
    user_repo
        .save(author.clone())
        .await
        .expect("Failed to save author");

    // 投稿作成
    let post = Post::new(
        "Delete Post".to_string(),
        "delete-post".to_string(),
        "Content".to_string(),
        author.id(),
    )
    .expect("Failed to create post");
    let post_id = post.id();
    post_repo.save(post).await.expect("Failed to save post");

    // 削除
    post_repo
        .delete(post_id)
        .await
        .expect("Failed to delete post");

    // 削除確認
    let found = post_repo
        .find_by_id(post_id)
        .await
        .expect("Failed to find post");
    assert!(found.is_none());

    cleanup_database(&*pool);
}

// ============================================================================
// Comment Repository 統合テスト
// ============================================================================

#[tokio::test]
#[serial]
async fn test_comment_repository_save_and_find_by_id() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());
    let comment_repo = DieselCommentRepository::new(pool.clone());

    // ユーザーと投稿作成
    let user = User::new(
        "commenter".to_string(),
        "commenter@test.com".to_string(),
        "pw",
    )
    .expect("Failed to create user");
    user_repo
        .save(user.clone())
        .await
        .expect("Failed to save user");

    let post = Post::new(
        "Post".to_string(),
        "post".to_string(),
        "Content".to_string(),
        user.id(),
    )
    .expect("Failed to create post");
    post_repo
        .save(post.clone())
        .await
        .expect("Failed to save post");

    // コメント作成
    let comment = Comment::new(
        post.id(),
        user.id(),
        "This is a test comment.".to_string(),
        None,
    )
    .expect("Failed to create comment");
    let comment_id = comment.id();

    // 保存
    comment_repo
        .save(comment)
        .await
        .expect("Failed to save comment");

    // ID で検索
    let found = comment_repo
        .find_by_id(comment_id)
        .await
        .expect("Failed to find comment");

    assert!(found.is_some());
    let found_comment = found.unwrap();
    assert_eq!(found_comment.id(), comment_id);
    assert_eq!(found_comment.content().as_str(), "This is a test comment.");

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_comment_repository_find_by_post_id() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());
    let comment_repo = DieselCommentRepository::new(pool.clone());

    // ユーザーと投稿作成
    let user = User::new(
        "postcommenter".to_string(),
        "postcommenter@test.com".to_string(),
        "pw",
    )
    .expect("Failed to create user");
    user_repo
        .save(user.clone())
        .await
        .expect("Failed to save user");

    let post = Post::new(
        "Comment Post".to_string(),
        "comment-post".to_string(),
        "Content".to_string(),
        user.id(),
    )
    .expect("Failed to create post");
    post_repo
        .save(post.clone())
        .await
        .expect("Failed to save post");

    // 複数コメント作成
    let comment1 = Comment::new(
        post.id(),
        user.id(),
        "First comment".to_string(),
        None,
    )
    .expect("Failed to create comment1");
    let comment2 = Comment::new(
        post.id(),
        user.id(),
        "Second comment".to_string(),
        None,
    )
    .expect("Failed to create comment2");

    comment_repo
        .save(comment1)
        .await
        .expect("Failed to save comment1");
    comment_repo
        .save(comment2)
        .await
        .expect("Failed to save comment2");

    // 投稿IDで検索
    let comments = comment_repo
        .find_by_post_id(post.id())
        .await
        .expect("Failed to find comments by post_id");

    assert_eq!(comments.len(), 2);
    assert!(comments
        .iter()
        .any(|c| c.content().as_str() == "First comment"));
    assert!(comments
        .iter()
        .any(|c| c.content().as_str() == "Second comment"));

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_comment_repository_delete() {
    let pool = setup_test_database();
    let user_repo = DieselUserRepository::new(pool.clone());
    let post_repo = DieselPostRepository::new(pool.clone());
    let comment_repo = DieselCommentRepository::new(pool.clone());

    // ユーザーと投稿作成
    let user = User::new(
        "deletecommenter".to_string(),
        "deletecommenter@test.com".to_string(),
        "pw",
    )
    .expect("Failed to create user");
    user_repo
        .save(user.clone())
        .await
        .expect("Failed to save user");

    let post = Post::new(
        "Delete Comment Post".to_string(),
        "delete-comment-post".to_string(),
        "Content".to_string(),
        user.id(),
    )
    .expect("Failed to create post");
    post_repo
        .save(post.clone())
        .await
        .expect("Failed to save post");

    // コメント作成
    let comment = Comment::new(
        post.id(),
        user.id(),
        "Comment to delete".to_string(),
        None,
    )
    .expect("Failed to create comment");
    let comment_id = comment.id();
    comment_repo
        .save(comment)
        .await
        .expect("Failed to save comment");

    // 削除
    comment_repo
        .delete(comment_id)
        .await
        .expect("Failed to delete comment");

    // 削除確認
    let found = comment_repo
        .find_by_id(comment_id)
        .await
        .expect("Failed to find comment");
    assert!(found.is_none());

    cleanup_database(&*pool);
}

// ============================================================================
// トランザクションテスト（Unit of Work 統合）
// ============================================================================

#[tokio::test]
#[serial]
async fn test_transaction_rollback_on_error() {
    use cms_backend::infrastructure::database::unit_of_work::DieselUnitOfWork;

    let pool = setup_test_database();
    let uow = DieselUnitOfWork::new(pool.clone());
    let user_repo = DieselUserRepository::new(pool.clone());

    // トランザクション内で操作を実行し、意図的にエラーを発生させる
    let result = uow
        .execute_in_transaction(|conn| {
            use cms_backend::application::ports::repositories::RepositoryError;
            use diesel::prelude::*;
            use diesel::RunQueryDsl;

            // ユーザーをDBに直接挿入
            diesel::sql_query(
                "INSERT INTO users (id, username, email, password_hash, created_at, updated_at) \
                 VALUES (gen_random_uuid(), 'txuser', 'tx@test.com', 'hash', NOW(), NOW())",
            )
            .execute(conn)?;

            // 意図的にエラーを返してロールバック
            Err(RepositoryError::DatabaseError(
                "Intentional error for rollback test".to_string(),
            ))
        })
        .await;

    assert!(result.is_err());

    // ロールバック確認: ユーザーが保存されていないことを確認
    let email = cms_backend::domain::user::Email::new("tx@test.com".to_string())
        .expect("Invalid email");
    let found = user_repo
        .find_by_email(email)
        .await
        .expect("Failed to find user");
    assert!(found.is_none(), "User should not exist after rollback");

    cleanup_database(&*pool);
}

#[tokio::test]
#[serial]
async fn test_transaction_commit_on_success() {
    use cms_backend::infrastructure::database::unit_of_work::DieselUnitOfWork;

    let pool = setup_test_database();
    let uow = DieselUnitOfWork::new(pool.clone());
    let user_repo = DieselUserRepository::new(pool.clone());

    // トランザクション内で操作を実行し、成功させる
    let result = uow
        .execute_in_transaction(|conn| {
            use diesel::prelude::*;
            use diesel::RunQueryDsl;

            diesel::sql_query(
                "INSERT INTO users (id, username, email, password_hash, created_at, updated_at) \
                 VALUES (gen_random_uuid(), 'commituser', 'commit@test.com', 'hash', NOW(), NOW())",
            )
            .execute(conn)?;

            Ok(())
        })
        .await;

    assert!(result.is_ok());

    // コミット確認: ユーザーが保存されていることを確認
    let email = cms_backend::domain::user::Email::new("commit@test.com".to_string())
        .expect("Invalid email");
    let found = user_repo
        .find_by_email(email)
        .await
        .expect("Failed to find user");
    assert!(
        found.is_some(),
        "User should exist after successful commit"
    );

    cleanup_database(&*pool);
}

// ============================================================================
// 並行アクセステスト（Connection Pool）
// ============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_user_creation() {
    let pool = setup_test_database();
    let repo = DieselUserRepository::new(pool.clone());

    // 並行して複数ユーザーを作成
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let repo = repo.clone();
            tokio::spawn(async move {
                let user = User::new(
                    format!("concurrent{i}"),
                    format!("concurrent{i}@test.com"),
                    "hashedpw",
                )
                .expect("Failed to create user");
                repo.save(user).await.expect("Failed to save user");
            })
        })
        .collect();

    // すべてのタスク完了を待つ
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // 5ユーザーが作成されたことを確認
    let users = repo.list_all().await.expect("Failed to list all users");
    assert_eq!(users.len(), 5);

    cleanup_database(&*pool);
}
