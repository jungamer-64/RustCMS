/// Domain model for comments - encapsulates all comment-related business logic and validation.
///
/// This module follows the unified Entity + Value Objects pattern established in Phase 1.
/// All validation occurs in Value Object constructors to ensure type safety.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::common::types::DomainError;
use crate::domain::post::PostId;
use crate::domain::user::UserId;

// ============================================================================
// Value Objects
// ============================================================================

/// Comment ID (NewType Pattern)
///
/// Type-safe identifier for comments using NewType pattern.
/// Ensures compile-time protection against ID mixing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(Uuid);

impl CommentId {
    /// Generate a new unique comment ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from existing UUID (use with caution - no validation)
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get inner UUID reference
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert into inner UUID (consuming)
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for CommentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CommentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Comment text (validated)
///
/// Validated text content for comments.
/// Invariants:
/// - Not empty
/// - Between 1 and 5,000 characters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentText(String);

impl CommentText {
    /// Create a validated comment text
    ///
    /// # Validation
    /// - `text` cannot be empty
    /// - `text` length must be between 1 and 5,000 characters
    pub fn new(text: String) -> Result<Self, DomainError> {
        if text.is_empty() {
            return Err(DomainError::InvalidCommentText(
                "Comment text cannot be empty".to_string(),
            ));
        }

        if text.len() > 5000 {
            return Err(DomainError::InvalidCommentText(format!(
                "Comment text exceeds 5,000 character limit: {} chars",
                text.len()
            )));
        }

        Ok(Self(text))
    }

    /// Get reference to text
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommentText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Comment status
///
/// Represents the lifecycle state of a comment.
/// Invariants:
/// - Published comments can be marked as edited (immutable)
/// - Deleted comments cannot transition to other states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    /// Newly created, not yet visible
    Pending,
    /// Visible to all users
    Published,
    /// Marked as edited (immutable flag, created_at unchanged)
    Edited,
    /// Soft-deleted (not shown in lists)
    Deleted,
}

// ============================================================================
// Entity
// ============================================================================

/// Comment entity - encapsulates comment business logic
///
/// Invariants:
/// - A comment must belong to a post
/// - A comment must have an author
/// - Only published or edited comments can be seen
/// - Published comments cannot transition to pending
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    id: CommentId,
    post_id: PostId,
    author_id: UserId,
    text: CommentText,
    status: CommentStatus,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl Comment {
    /// Create a new comment (factory method)
    ///
    /// A new comment starts in Pending status and must be published.
    ///
    /// # Arguments
    /// * `post_id` - The post this comment belongs to
    /// * `author_id` - The user authoring this comment
    /// * `text` - The comment text (validated)
    ///
    /// # Returns
    /// * `Result<Self, DomainError>` - Comment if valid, error otherwise
    pub fn new(post_id: PostId, author_id: UserId, text: CommentText) -> Result<Self, DomainError> {
        let now = Utc::now();
        Ok(Self {
            id: CommentId::new(),
            post_id,
            author_id,
            text,
            status: CommentStatus::Pending,
            created_at: now,
            edited_at: None,
            updated_at: now,
        })
    }

    /// Restore a comment from database (factory method)
    ///
    /// Used by Repository implementations to reconstruct domain entities.
    /// Does NOT perform validation beyond Value Object constraints.
    ///
    /// # Arguments
    /// * `id` - Comment identifier
    /// * `post_id` - Post foreign key
    /// * `author_id` - Author foreign key
    /// * `text` - Comment text (validated)
    /// * `status` - Comment status
    /// * `created_at` - Creation timestamp
    /// * `edited_at` - Last edit timestamp (optional)
    /// * `updated_at` - Last update timestamp
    pub fn restore(
        id: CommentId,
        post_id: PostId,
        author_id: UserId,
        text: CommentText,
        status: CommentStatus,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            post_id,
            author_id,
            text,
            status,
            created_at,
            edited_at,
            updated_at,
        }
    }

    // ========================================================================
    // State Transitions
    // ========================================================================

    /// Publish a pending comment
    ///
    /// Transitions: Pending → Published
    ///
    /// # Errors
    /// - If comment is not in Pending status
    pub fn publish(&mut self) -> Result<(), DomainError> {
        match self.status {
            CommentStatus::Pending => {
                self.status = CommentStatus::Published;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(DomainError::InvalidCommentStatus(format!(
                "Cannot publish comment in {:?} state",
                self.status
            ))),
        }
    }

    /// Edit a published comment
    ///
    /// Updates text and transitions to Edited status.
    /// Transitions: Published → Edited (cannot edit again)
    ///
    /// # Errors
    /// - If comment is not Published
    pub fn edit(&mut self, new_text: CommentText) -> Result<(), DomainError> {
        match self.status {
            CommentStatus::Published => {
                self.text = new_text;
                self.status = CommentStatus::Edited;
                self.edited_at = Some(Utc::now());
                self.updated_at = Utc::now();
                Ok(())
            }
            CommentStatus::Edited => {
                self.text = new_text;
                self.edited_at = Some(Utc::now());
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(DomainError::InvalidCommentStatus(format!(
                "Cannot edit comment in {:?} state",
                self.status
            ))),
        }
    }

    /// Delete a comment (soft delete)
    ///
    /// Transitions: Published | Edited → Deleted
    ///
    /// # Errors
    /// - If comment is already deleted or still pending
    pub fn delete(&mut self) -> Result<(), DomainError> {
        match self.status {
            CommentStatus::Deleted => {
                // Idempotent: already deleted
                Ok(())
            }
            CommentStatus::Pending => Err(DomainError::InvalidCommentStatus(
                "Cannot delete pending comment".to_string(),
            )),
            CommentStatus::Published | CommentStatus::Edited => {
                self.status = CommentStatus::Deleted;
                self.updated_at = Utc::now();
                Ok(())
            }
        }
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get comment ID
    pub fn id(&self) -> CommentId {
        self.id
    }

    /// Get post ID (foreign key)
    pub fn post_id(&self) -> PostId {
        self.post_id
    }

    /// Get author user ID
    pub fn author_id(&self) -> UserId {
        self.author_id
    }

    /// Get comment text
    pub fn text(&self) -> &CommentText {
        &self.text
    }

    /// Get current status
    pub fn status(&self) -> CommentStatus {
        self.status
    }

    /// Get creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get last edit timestamp (if edited)
    pub fn edited_at(&self) -> Option<DateTime<Utc>> {
        self.edited_at
    }

    /// Get last update timestamp
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // ========================================================================
    // State Query Methods
    // ========================================================================

    /// Check if comment is visible (published or edited)
    pub fn is_visible(&self) -> bool {
        matches!(
            self.status,
            CommentStatus::Published | CommentStatus::Edited
        )
    }

    /// Check if comment is published
    pub fn is_published(&self) -> bool {
        self.status == CommentStatus::Published
    }

    /// Check if comment has been edited
    pub fn is_edited(&self) -> bool {
        self.status == CommentStatus::Edited
    }

    /// Check if comment is deleted
    pub fn is_deleted(&self) -> bool {
        self.status == CommentStatus::Deleted
    }

    /// Check if comment is pending approval
    pub fn is_pending(&self) -> bool {
        self.status == CommentStatus::Pending
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // CommentId tests
    #[test]
    fn test_comment_id_generation() {
        let id1 = CommentId::new();
        let id2 = CommentId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_comment_id_display() {
        let id = CommentId::new();
        let display_str = format!("{}", id);
        assert!(!display_str.is_empty());
        assert!(display_str.len() == 36, "UUID string should be 36 chars"); // UUID v4 format
    }

    // CommentText tests
    #[test]
    fn test_comment_text_valid() {
        let text = CommentText::new("This is a valid comment".to_string()).unwrap();
        assert_eq!(text.as_str(), "This is a valid comment");
    }

    #[test]
    fn test_comment_text_empty() {
        let result = CommentText::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCommentText(_))));
    }

    #[test]
    fn test_comment_text_too_long() {
        let long_text = "x".repeat(5001);
        let result = CommentText::new(long_text);
        assert!(matches!(result, Err(DomainError::InvalidCommentText(_))));
    }

    #[test]
    fn test_comment_text_boundary_5000() {
        let boundary_text = "x".repeat(5000);
        let result = CommentText::new(boundary_text);
        assert!(result.is_ok());
    }

    // Comment entity tests
    #[test]
    fn test_comment_creation() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let comment = Comment::new(post_id, author_id, text).unwrap();

        assert_eq!(comment.post_id(), post_id);
        assert_eq!(comment.author_id(), author_id);
        assert!(comment.is_pending());
        assert!(!comment.is_visible());
    }

    #[test]
    fn test_comment_publish() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();

        assert!(comment.is_pending());
        let publish_result = comment.publish();
        assert!(publish_result.is_ok());
        assert!(comment.is_published());
        assert!(comment.is_visible());
    }

    #[test]
    fn test_comment_publish_twice_fails() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        let _ = comment.publish();

        let result = comment.publish();
        assert!(matches!(result, Err(DomainError::InvalidCommentStatus(_))));
    }

    #[test]
    fn test_comment_edit() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Original text".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        let _ = comment.publish();

        let new_text = CommentText::new("Edited text".to_string()).unwrap();
        let edit_result = comment.edit(new_text);

        assert!(edit_result.is_ok());
        assert!(comment.is_edited());
        assert!(comment.edited_at().is_some());
    }

    #[test]
    fn test_comment_edit_again() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Original text".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        let _ = comment.publish();

        let edited1 = CommentText::new("Edit 1".to_string()).unwrap();
        let _ = comment.edit(edited1);

        let edited2 = CommentText::new("Edit 2".to_string()).unwrap();
        let result = comment.edit(edited2);

        assert!(result.is_ok(), "Edited comments can be edited again");
        assert!(comment.is_edited());
    }

    #[test]
    fn test_comment_delete() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        let _ = comment.publish();

        let delete_result = comment.delete();
        assert!(delete_result.is_ok());
        assert!(comment.is_deleted());
        assert!(!comment.is_visible());
    }

    #[test]
    fn test_comment_delete_pending_fails() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();

        let result = comment.delete();
        assert!(matches!(result, Err(DomainError::InvalidCommentStatus(_))));
    }

    #[test]
    fn test_comment_delete_idempotent() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        let _ = comment.publish();
        let _ = comment.delete();

        let result = comment.delete();
        assert!(
            result.is_ok(),
            "Deleting an already deleted comment should succeed"
        );
        assert!(comment.is_deleted());
    }

    #[test]
    fn test_comment_visibility_states() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        assert!(!comment.is_visible());

        let _ = comment.publish();
        assert!(comment.is_visible());

        let new_text = CommentText::new("Edited".to_string()).unwrap();
        let _ = comment.edit(new_text);
        assert!(comment.is_visible());

        let _ = comment.delete();
        assert!(!comment.is_visible());
    }

    #[test]
    fn test_comment_timestamps_update_on_state_change() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text.clone()).unwrap();
        let created_at = comment.created_at();
        let updated_at_initial = comment.updated_at();

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        let _ = comment.publish();
        let updated_at_after_publish = comment.updated_at();

        assert!(
            updated_at_after_publish > updated_at_initial,
            "updated_at should change on publish"
        );
        assert_eq!(
            created_at,
            comment.created_at(),
            "created_at should not change"
        );
    }
}
