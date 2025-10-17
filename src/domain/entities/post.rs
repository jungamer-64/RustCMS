//! 投稿ドメインモデル (Post Domain Model)
//!
//! Entity + Value Objects 統合パターン（Phase 2）
//!
//! このファイルには以下が含まれます：
//! - Value Objects: PostId, Slug, Title, Content, PostStatus, PublishedAt
//! - Entity: Post
//! - ビジネスルール実装
//!
//! 設計原則：
//! - Value Objects は検証ロジックを内包
//! - Post Entity は不変条件を struct フィールドの private 化で保証
//! - ビジネスメソッドはイベント発行を考慮

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::common::types::DomainError;

// ============================================================================
// Value Objects
// ============================================================================

/// 投稿ID（NewType Pattern）
///
/// # 不変条件
/// - 内部のUUIDは常に有効
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PostId(Uuid);

impl PostId {
    /// 新しい投稿IDを生成
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 既存のUUIDから投稿IDを作成
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// 内部のUUIDへの参照を取得
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// UUIDを消費して取得
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for PostId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for PostId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<PostId> for Uuid {
    fn from(id: PostId) -> Self {
        id.0
    }
}

/// Slug（URL用スラッグ、検証済み）
///
/// # 不変条件
/// - 空でない
/// - 3文字以上 50文字以下
/// - 小文字英数字とハイフンのみ
/// - ハイフンで開始・終了しない
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Slug(String);

impl Slug {
    /// スラッグを検証して作成
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - 空の文字列
    /// - 3文字未満または50文字超
    /// - 不正な文字を含む（大文字、特殊文字など）
    /// - ハイフンで開始・終了
    pub fn new(slug: String) -> Result<Self, DomainError> {
        if slug.is_empty() {
            return Err(DomainError::InvalidSlug("Slug cannot be empty".to_string()));
        }

        if slug.len() < 3 || slug.len() > 50 {
            return Err(DomainError::InvalidSlug(format!(
                "Slug length must be between 3 and 50, got {}",
                slug.len()
            )));
        }

        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(DomainError::InvalidSlug(
                "Slug cannot start or end with hyphen".to_string(),
            ));
        }

        if !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(DomainError::InvalidSlug(
                "Slug must contain only lowercase letters, digits, and hyphens".to_string(),
            ));
        }

        Ok(Self(slug))
    }

    /// スラッグの文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Slug> for String {
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

/// タイトル（検証済み）
///
/// # 不変条件
/// - 空でない
/// - 1文字以上 200文字以下
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Title(String);

impl Title {
    /// タイトルを検証して作成
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - 空の文字列
    /// - 200文字を超える
    pub fn new(title: String) -> Result<Self, DomainError> {
        if title.is_empty() {
            return Err(DomainError::InvalidTitle(
                "Title cannot be empty".to_string(),
            ));
        }

        if title.len() > 200 {
            return Err(DomainError::InvalidTitle(format!(
                "Title must not exceed 200 characters, got {}",
                title.len()
            )));
        }

        Ok(Self(title))
    }

    /// タイトルの文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Title> for String {
    fn from(title: Title) -> Self {
        title.0
    }
}

/// コンテンツ（検証済み）
///
/// # 不変条件
/// - 空でない
/// - 10文字以上 100,000文字以下
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Content(String);

impl Content {
    /// コンテンツを検証して作成
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - 空の文字列
    /// - 10文字未満
    /// - 100,000文字を超える
    pub fn new(content: String) -> Result<Self, DomainError> {
        if content.is_empty() {
            return Err(DomainError::InvalidContent(
                "Content cannot be empty".to_string(),
            ));
        }

        if content.len() < 10 {
            return Err(DomainError::InvalidContent(
                "Content must be at least 10 characters".to_string(),
            ));
        }

        if content.len() > 100_000 {
            return Err(DomainError::InvalidContent(format!(
                "Content must not exceed 100,000 characters, got {}",
                content.len()
            )));
        }

        Ok(Self(content))
    }

    /// コンテンツの文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Content> for String {
    fn from(content: Content) -> Self {
        content.0
    }
}

/// 投稿ステータス（列挙型）
///
/// # 状態遷移
/// - Draft → Published
/// - Published → Archived
/// - Draft → Archived
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    /// 下書き
    Draft,
    /// 公開
    Published,
    /// アーカイブ
    Archived,
}

impl fmt::Display for PostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostStatus::Draft => write!(f, "draft"),
            PostStatus::Published => write!(f, "published"),
            PostStatus::Archived => write!(f, "archived"),
        }
    }
}

/// 公開日時（検証済み）
///
/// # 不変条件
/// - 現在時刻以降（未来の公開予約を許可）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublishedAt(DateTime<Utc>);

impl PublishedAt {
    /// 公開日時を作成（現在時刻を使用）
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// 指定の日時で公開日時を作成
    ///
    /// # Errors
    ///
    /// 日時が現在時刻より前の場合エラーを返します
    pub fn new(published_at: DateTime<Utc>) -> Result<Self, DomainError> {
        if published_at < Utc::now() {
            return Err(DomainError::InvalidPublishedAt(
                "Published date must not be in the past".to_string(),
            ));
        }
        Ok(Self(published_at))
    }

    /// 内部の DateTime を取得
    #[must_use]
    pub const fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl fmt::Display for PublishedAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d %H:%M:%S UTC"))
    }
}

// ============================================================================
// Entity
// ============================================================================

/// 投稿エンティティ（ドメインモデル）
///
/// ビジネスルールとライフサイクルメソッドを含む。
/// すべてのフィールドは private であり、invariants を保証。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    id: PostId,
    author_id: crate::domain::user::UserId,
    title: Title,
    slug: Slug,
    content: Content,
    status: PostStatus,
    created_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl Post {
    /// 新しい投稿を作成（ファクトリメソッド）
    ///
    /// # Arguments
    ///
    /// * `author_id` - 投稿の著者ID
    /// * `title` - 投稿タイトル
    /// * `slug` - URL用スラッグ
    /// * `content` - 投稿コンテンツ
    ///
    /// # Returns
    ///
    /// 初期状態は `Draft` で作成された投稿
    #[must_use]
    pub fn new(
        author_id: crate::domain::user::UserId,
        title: Title,
        slug: Slug,
        content: Content,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: PostId::new(),
            author_id,
            title,
            slug,
            content,
            status: PostStatus::Draft,
            created_at: now,
            published_at: None,
            updated_at: now,
        }
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// 投稿IDを取得
    #[must_use]
    pub const fn id(&self) -> PostId {
        self.id
    }

    /// 著者IDを取得
    #[must_use]
    pub const fn author_id(&self) -> crate::domain::user::UserId {
        self.author_id
    }

    /// タイトルの参照を取得
    #[must_use]
    pub const fn title(&self) -> &Title {
        &self.title
    }

    /// スラッグの参照を取得
    #[must_use]
    pub const fn slug(&self) -> &Slug {
        &self.slug
    }

    /// コンテンツの参照を取得
    #[must_use]
    pub const fn content(&self) -> &Content {
        &self.content
    }

    /// ステータスを取得
    #[must_use]
    pub const fn status(&self) -> PostStatus {
        self.status
    }

    /// 作成日時を取得
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// 公開日時を取得
    #[must_use]
    pub const fn published_at(&self) -> Option<DateTime<Utc>> {
        self.published_at
    }

    /// 更新日時を取得
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // ========================================================================
    // Business Methods
    // ========================================================================

    /// 投稿を公開する
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - ステータスが Draft でない場合
    pub fn publish(&mut self) -> Result<(), DomainError> {
        if self.status != PostStatus::Draft {
            return Err(DomainError::InvalidStateTransition(
                "Can only publish Draft posts".to_string(),
            ));
        }

        self.status = PostStatus::Published;
        self.published_at = Some(Utc::now());
        self.updated_at = Utc::now();

        Ok(())
    }

    /// 投稿をアーカイブする
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - ステータスが Archived である場合
    pub fn archive(&mut self) -> Result<(), DomainError> {
        if self.status == PostStatus::Archived {
            return Err(DomainError::InvalidStateTransition(
                "Post is already archived".to_string(),
            ));
        }

        self.status = PostStatus::Archived;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// タイトルを変更する
    ///
    /// # Arguments
    ///
    /// * `new_title` - 新しいタイトル
    pub fn change_title(&mut self, new_title: Title) {
        self.title = new_title;
        self.updated_at = Utc::now();
    }

    /// コンテンツを変更する
    ///
    /// # Arguments
    ///
    /// * `new_content` - 新しいコンテンツ
    pub fn change_content(&mut self, new_content: Content) {
        self.content = new_content;
        self.updated_at = Utc::now();
    }

    /// スラッグを変更する
    ///
    /// # Arguments
    ///
    /// * `new_slug` - 新しいスラッグ
    pub fn change_slug(&mut self, new_slug: Slug) {
        self.slug = new_slug;
        self.updated_at = Utc::now();
    }

    /// 投稿が公開されているかを確認
    #[must_use]
    pub const fn is_published(&self) -> bool {
        matches!(self.status, PostStatus::Published)
    }

    /// 投稿がドラフトかを確認
    #[must_use]
    pub const fn is_draft(&self) -> bool {
        matches!(self.status, PostStatus::Draft)
    }

    /// 投稿がアーカイブされているかを確認
    #[must_use]
    pub const fn is_archived(&self) -> bool {
        matches!(self.status, PostStatus::Archived)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_id_generation() {
        let id1 = PostId::new();
        let id2 = PostId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_post_id_display() {
        let id = PostId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_slug_valid() {
        let slug = Slug::new("my-awesome-post".to_string()).unwrap();
        assert_eq!(slug.as_str(), "my-awesome-post");
    }

    #[test]
    fn test_slug_empty() {
        let result = Slug::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidSlug(_))));
    }

    #[test]
    fn test_slug_too_short() {
        let result = Slug::new("ab".to_string());
        assert!(matches!(result, Err(DomainError::InvalidSlug(_))));
    }

    #[test]
    fn test_slug_too_long() {
        let slug_str = "a".repeat(51);
        let result = Slug::new(slug_str);
        assert!(matches!(result, Err(DomainError::InvalidSlug(_))));
    }

    #[test]
    fn test_slug_uppercase_rejected() {
        let result = Slug::new("My-Post".to_string());
        assert!(matches!(result, Err(DomainError::InvalidSlug(_))));
    }

    #[test]
    fn test_slug_starts_with_hyphen() {
        let result = Slug::new("-my-post".to_string());
        assert!(matches!(result, Err(DomainError::InvalidSlug(_))));
    }

    #[test]
    fn test_title_valid() {
        let title = Title::new("My Post Title".to_string()).unwrap();
        assert_eq!(title.as_str(), "My Post Title");
    }

    #[test]
    fn test_title_empty() {
        let result = Title::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidTitle(_))));
    }

    #[test]
    fn test_title_too_long() {
        let title_str = "a".repeat(201);
        let result = Title::new(title_str);
        assert!(matches!(result, Err(DomainError::InvalidTitle(_))));
    }

    #[test]
    fn test_content_valid() {
        let content =
            Content::new("This is a valid post content with sufficient length".to_string())
                .unwrap();
        assert!(content.as_str().len() >= 10);
    }

    #[test]
    fn test_content_empty() {
        let result = Content::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidContent(_))));
    }

    #[test]
    fn test_content_too_short() {
        let result = Content::new("short".to_string());
        assert!(matches!(result, Err(DomainError::InvalidContent(_))));
    }

    #[test]
    fn test_post_creation() {
        let user_id = crate::domain::user::UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is test content for the post".to_string()).unwrap();

        let post = Post::new(user_id, title, slug, content);

        assert_eq!(post.status, PostStatus::Draft);
        assert_eq!(post.author_id, user_id);
        assert!(!post.is_published());
        assert!(post.is_draft());
    }

    #[test]
    fn test_post_publish() {
        let user_id = crate::domain::user::UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is test content for the post".to_string()).unwrap();

        let mut post = Post::new(user_id, title, slug, content);
        assert!(post.is_draft());

        post.publish().unwrap();
        assert!(post.is_published());
        assert!(post.published_at.is_some());
    }

    #[test]
    fn test_post_publish_twice_fails() {
        let user_id = crate::domain::user::UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is test content for the post".to_string()).unwrap();

        let mut post = Post::new(user_id, title, slug, content);
        post.publish().unwrap();

        let result = post.publish();
        assert!(matches!(
            result,
            Err(DomainError::InvalidStateTransition(_))
        ));
    }

    #[test]
    fn test_post_archive() {
        let user_id = crate::domain::user::UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is test content for the post".to_string()).unwrap();

        let mut post = Post::new(user_id, title, slug, content);
        post.publish().unwrap();

        assert!(post.archive().is_ok());
        assert!(post.is_archived());
    }

    #[test]
    fn test_post_change_title() {
        let user_id = crate::domain::user::UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is test content for the post".to_string()).unwrap();

        let mut post = Post::new(user_id, title, slug, content);
        let old_updated_at = post.updated_at;

        let new_title = Title::new("Updated Title".to_string()).unwrap();
        post.change_title(new_title.clone());

        assert_eq!(post.title(), &new_title);
        assert!(post.updated_at > old_updated_at);
    }
}
