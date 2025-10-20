/// Domain Events Layer
///
/// ドメインイベントは、ドメイン層で発生した重要なビジネスイベントを表現します。
/// これらのイベントは Application/Infrastructure 層で購読（subscribe）されます。
///
/// # イベント駆動設計の利点
///
/// 1. **疎結合**: ドメイン層が application/infrastructure 層の詳細を知らない
/// 2. **非同期処理**: イベント購読者が非同期で実行される
/// 3. **監査可能性**: すべてのドメインイベントはログ記録される
/// 4. **拡張性**: 新しいイベントリスナーを追加しても既存コードは変更不要
///
/// # 使用例
///
/// ```rust,ignore
/// // Entity で何か重要なことが起きた
/// let mut user = User::new(email, username)?;
/// user.activate()?;
/// // → UserActivated イベントが発行される
///
/// // イベントはリスナーで購読される
/// #[tokio::spawn]
/// async {
///     while let Ok(event) = rx.recv().await {
///         match event {
///             DomainEvent::UserActivated(user_id) => {
///                 // ウェルカムメール送信など...
///             }
///             // ...
///         }
///     }
/// }
/// ```
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// uuid import not directly used here; keep available via UserId type

use crate::category::CategoryId;
use crate::comment::CommentId;
use crate::post::PostId;
use crate::tag::TagId;
use crate::user::UserId;

// ============================================================================
// Domain Event Types
// ============================================================================

/// ドメイン内で発生するすべてのイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum DomainEvent {
    // ========== User Events ==========
    /// ユーザーが登録された
    UserRegistered {
        user_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// ユーザーがアクティベートされた
    UserActivated {
        user_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// ユーザーがデアクティベートされた（凍結）
    UserDeactivated {
        user_id: UserId,
        reason: Option<String>,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// ユーザーが削除された
    UserDeleted {
        user_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// ユーザーのメールアドレスが変更された
    UserEmailChanged {
        user_id: UserId,
        old_email: String,
        new_email: String,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },

    // ========== Post Events ==========
    /// 投稿が作成された
    PostCreated {
        post_id: PostId,
        author_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// 投稿が公開された
    PostPublished {
        post_id: PostId,
        author_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// 投稿が下書きに戻された
    PostArchived {
        post_id: PostId,
        author_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// 投稿が削除された
    PostDeleted {
        post_id: PostId,
        author_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// 投稿が更新された
    PostUpdated {
        post_id: PostId,
        author_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },

    // ========== Comment Events ==========
    /// コメントが作成された
    CommentCreated {
        comment_id: CommentId,
        post_id: PostId,
        author_id: UserId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// コメントが削除された
    CommentDeleted {
        comment_id: CommentId,
        post_id: PostId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// コメントが更新された
    CommentUpdated {
        comment_id: CommentId,
        post_id: PostId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },

    // ========== Tag Events ==========
    /// タグが作成された
    TagCreated {
        tag_id: TagId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// タグが削除された
    TagDeleted {
        tag_id: TagId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// タグの使用状況が変わった
    TagUsageChanged {
        tag_id: TagId,
        new_usage_count: i64,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },

    // ========== Category Events ==========
    /// カテゴリが作成された
    CategoryCreated {
        category_id: CategoryId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// カテゴリが無効化された
    CategoryDeactivated {
        category_id: CategoryId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// カテゴリが削除された
    CategoryDeleted {
        category_id: CategoryId,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
    /// カテゴリの投稿数が変わった
    CategoryPostCountChanged {
        category_id: CategoryId,
        new_post_count: i64,
        #[serde(skip)]
        timestamp: DateTime<Utc>,
    },
}

impl DomainEvent {
    /// イベントのタイムスタンプを取得
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            DomainEvent::UserRegistered { timestamp, .. }
            | DomainEvent::UserActivated { timestamp, .. }
            | DomainEvent::UserDeactivated { timestamp, .. }
            | DomainEvent::UserDeleted { timestamp, .. }
            | DomainEvent::UserEmailChanged { timestamp, .. }
            | DomainEvent::PostCreated { timestamp, .. }
            | DomainEvent::PostPublished { timestamp, .. }
            | DomainEvent::PostArchived { timestamp, .. }
            | DomainEvent::PostDeleted { timestamp, .. }
            | DomainEvent::PostUpdated { timestamp, .. }
            | DomainEvent::CommentCreated { timestamp, .. }
            | DomainEvent::CommentDeleted { timestamp, .. }
            | DomainEvent::CommentUpdated { timestamp, .. }
            | DomainEvent::TagCreated { timestamp, .. }
            | DomainEvent::TagDeleted { timestamp, .. }
            | DomainEvent::TagUsageChanged { timestamp, .. }
            | DomainEvent::CategoryCreated { timestamp, .. }
            | DomainEvent::CategoryDeactivated { timestamp, .. }
            | DomainEvent::CategoryDeleted { timestamp, .. }
            | DomainEvent::CategoryPostCountChanged { timestamp, .. } => *timestamp,
        }
    }

    /// イベント名を文字列で取得
    pub fn event_name(&self) -> &'static str {
        match self {
            DomainEvent::UserRegistered { .. } => "UserRegistered",
            DomainEvent::UserActivated { .. } => "UserActivated",
            DomainEvent::UserDeactivated { .. } => "UserDeactivated",
            DomainEvent::UserDeleted { .. } => "UserDeleted",
            DomainEvent::UserEmailChanged { .. } => "UserEmailChanged",
            DomainEvent::PostCreated { .. } => "PostCreated",
            DomainEvent::PostPublished { .. } => "PostPublished",
            DomainEvent::PostArchived { .. } => "PostArchived",
            DomainEvent::PostDeleted { .. } => "PostDeleted",
            DomainEvent::PostUpdated { .. } => "PostUpdated",
            DomainEvent::CommentCreated { .. } => "CommentCreated",
            DomainEvent::CommentDeleted { .. } => "CommentDeleted",
            DomainEvent::CommentUpdated { .. } => "CommentUpdated",
            DomainEvent::TagCreated { .. } => "TagCreated",
            DomainEvent::TagDeleted { .. } => "TagDeleted",
            DomainEvent::TagUsageChanged { .. } => "TagUsageChanged",
            DomainEvent::CategoryCreated { .. } => "CategoryCreated",
            DomainEvent::CategoryDeactivated { .. } => "CategoryDeactivated",
            DomainEvent::CategoryDeleted { .. } => "CategoryDeleted",
            DomainEvent::CategoryPostCountChanged { .. } => "CategoryPostCountChanged",
        }
    }
}

// ============================================================================
// Event Publisher Trait (Port)
// ============================================================================

/// イベント発行の Port (interface)
///
/// Application/Infrastructure 層が実装し、Domain 層がこれを通じてイベント発行
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// ドメインイベントを発行
    async fn publish(&self, event: DomainEvent) -> Result<(), String>;

    /// 複数のイベントを一括発行
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), String> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event_timestamp() {
        let event = DomainEvent::UserRegistered {
            user_id: UserId::new(),
            timestamp: Utc::now(),
        };
        let ts1 = event.timestamp();
        let ts2 = event.timestamp();
        assert_eq!(ts1, ts2);
    }

    #[test]
    fn test_domain_event_names() {
        let events = vec![
            (
                DomainEvent::UserRegistered {
                    user_id: UserId::new(),
                    timestamp: Utc::now(),
                },
                "UserRegistered",
            ),
            (
                DomainEvent::PostPublished {
                    post_id: PostId::new(),
                    author_id: UserId::new(),
                    timestamp: Utc::now(),
                },
                "PostPublished",
            ),
            (
                DomainEvent::CommentCreated {
                    comment_id: CommentId::new(),
                    post_id: PostId::new(),
                    author_id: UserId::new(),
                    timestamp: Utc::now(),
                },
                "CommentCreated",
            ),
            (
                DomainEvent::TagCreated {
                    tag_id: TagId::new(),
                    timestamp: Utc::now(),
                },
                "TagCreated",
            ),
            (
                DomainEvent::CategoryCreated {
                    category_id: CategoryId::new(),
                    timestamp: Utc::now(),
                },
                "CategoryCreated",
            ),
        ];

        for (event, expected_name) in events {
            assert_eq!(event.event_name(), expected_name);
        }
    }

    #[test]
    fn test_all_event_variants_have_names() {
        // Ensure all DomainEvent variants have been named
        let user_id = UserId::new();
        let post_id = PostId::new();
        let comment_id = CommentId::new();
        let tag_id = TagId::new();
        let category_id = CategoryId::new();
        let now = Utc::now();

        let all_events = vec![
            DomainEvent::UserRegistered {
                user_id,
                timestamp: now,
            },
            DomainEvent::UserActivated {
                user_id,
                timestamp: now,
            },
            DomainEvent::UserDeactivated {
                user_id,
                reason: None,
                timestamp: now,
            },
            DomainEvent::UserDeleted {
                user_id,
                timestamp: now,
            },
            DomainEvent::UserEmailChanged {
                user_id,
                old_email: "old@example.com".to_string(),
                new_email: "new@example.com".to_string(),
                timestamp: now,
            },
            DomainEvent::PostCreated {
                post_id,
                author_id: user_id,
                timestamp: now,
            },
            DomainEvent::PostPublished {
                post_id,
                author_id: user_id,
                timestamp: now,
            },
            DomainEvent::PostArchived {
                post_id,
                author_id: user_id,
                timestamp: now,
            },
            DomainEvent::PostDeleted {
                post_id,
                author_id: user_id,
                timestamp: now,
            },
            DomainEvent::PostUpdated {
                post_id,
                author_id: user_id,
                timestamp: now,
            },
            DomainEvent::CommentCreated {
                comment_id,
                post_id,
                author_id: user_id,
                timestamp: now,
            },
            DomainEvent::CommentDeleted {
                comment_id,
                post_id,
                timestamp: now,
            },
            DomainEvent::CommentUpdated {
                comment_id,
                post_id,
                timestamp: now,
            },
            DomainEvent::TagCreated {
                tag_id,
                timestamp: now,
            },
            DomainEvent::TagDeleted {
                tag_id,
                timestamp: now,
            },
            DomainEvent::TagUsageChanged {
                tag_id,
                new_usage_count: 5,
                timestamp: now,
            },
            DomainEvent::CategoryCreated {
                category_id,
                timestamp: now,
            },
            DomainEvent::CategoryDeactivated {
                category_id,
                timestamp: now,
            },
            DomainEvent::CategoryDeleted {
                category_id,
                timestamp: now,
            },
            DomainEvent::CategoryPostCountChanged {
                category_id,
                new_post_count: 10,
                timestamp: now,
            },
        ];

        // All events should have non-empty names
        for event in all_events {
            let name = event.event_name();
            assert!(!name.is_empty(), "Event should have a name");
        }
    }
}
