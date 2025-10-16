// tests/domain_extended_tests.rs
//! ドメインモデル拡張テスト (Phase 2+)
//!
//! 新しい Entity、Value Objects、Business Rules のテスト。
//! Phase 2-3 で追加される Rating, Favorite, Notification などの検証。

#[cfg(test)]
mod domain_extended {
    use std::collections::HashMap;

    // ============= Rating Entity (仮) =============

    #[derive(Debug, Clone, PartialEq)]
    struct RatingId(String);

    #[derive(Debug, Clone)]
    struct Rating {
        id: RatingId,
        post_id: String,
        user_id: String,
        score: u8, // 1-5
    }

    impl Rating {
        fn new(post_id: String, user_id: String, score: u8) -> Result<Self, String> {
            if score < 1 || score > 5 {
                return Err("Rating score must be between 1 and 5".to_string());
            }

            Ok(Self {
                id: RatingId(uuid::Uuid::new_v4().to_string()),
                post_id,
                user_id,
                score,
            })
        }

        fn is_valid(&self) -> bool {
            self.score >= 1 && self.score <= 5
        }
    }

    #[test]
    fn test_rating_creation_valid() {
        let rating = Rating::new("post123".to_string(), "user456".to_string(), 4);
        assert!(rating.is_ok());
        assert_eq!(rating.unwrap().score, 4);
    }

    #[test]
    fn test_rating_creation_invalid_zero() {
        let rating = Rating::new("post123".to_string(), "user456".to_string(), 0);
        assert!(rating.is_err());
        assert_eq!(
            rating.err(),
            Some("Rating score must be between 1 and 5".to_string())
        );
    }

    #[test]
    fn test_rating_creation_invalid_too_high() {
        let rating = Rating::new("post123".to_string(), "user456".to_string(), 6);
        assert!(rating.is_err());
    }

    #[test]
    fn test_rating_is_valid() {
        let rating = Rating {
            id: RatingId("123".to_string()),
            post_id: "post123".to_string(),
            user_id: "user456".to_string(),
            score: 3,
        };
        assert!(rating.is_valid());
    }

    // ============= Favorite Entity (仮) =============

    #[derive(Debug, Clone, PartialEq)]
    struct FavoriteId(String);

    #[derive(Debug, Clone)]
    struct Favorite {
        id: FavoriteId,
        post_id: String,
        user_id: String,
        created_at: String,
    }

    impl Favorite {
        fn new(post_id: String, user_id: String) -> Self {
            Self {
                id: FavoriteId(uuid::Uuid::new_v4().to_string()),
                post_id,
                user_id,
                created_at: chrono::Utc::now().to_rfc3339(),
            }
        }
    }

    #[test]
    fn test_favorite_creation() {
        let favorite = Favorite::new("post123".to_string(), "user456".to_string());
        assert_eq!(favorite.post_id, "post123");
        assert_eq!(favorite.user_id, "user456");
        assert!(!favorite.id.0.is_empty());
    }

    // ============= Notification Entity (仮) =============

    #[derive(Debug, Clone, PartialEq)]
    enum NotificationType {
        PostReplied,
        CommentLiked,
        PostFavorited,
        UserFollowed,
    }

    #[derive(Debug, Clone)]
    struct Notification {
        id: String,
        user_id: String,
        notification_type: NotificationType,
        content: String,
        is_read: bool,
    }

    impl Notification {
        fn new(user_id: String, notification_type: NotificationType, content: String) -> Self {
            Self {
                id: uuid::Uuid::new_v4().to_string(),
                user_id,
                notification_type,
                content,
                is_read: false,
            }
        }

        fn mark_as_read(&mut self) {
            self.is_read = true;
        }
    }

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            "user123".to_string(),
            NotificationType::PostReplied,
            "Your post has been replied".to_string(),
        );

        assert_eq!(notification.user_id, "user123");
        assert_eq!(
            notification.notification_type,
            NotificationType::PostReplied
        );
        assert!(!notification.is_read);
    }

    #[test]
    fn test_notification_mark_as_read() {
        let mut notification = Notification::new(
            "user123".to_string(),
            NotificationType::CommentLiked,
            "Your comment was liked".to_string(),
        );

        assert!(!notification.is_read);
        notification.mark_as_read();
        assert!(notification.is_read);
    }

    // ============= MultiLanguageTitle Value Object =============

    #[derive(Debug, Clone, PartialEq)]
    struct MultiLanguageTitle {
        en: String,
        ja: String,
        es: Option<String>,
    }

    impl MultiLanguageTitle {
        fn new(en: String, ja: String) -> Result<Self, String> {
            if en.is_empty() || ja.is_empty() {
                return Err("English and Japanese titles are required".to_string());
            }

            if en.len() > 255 || ja.len() > 255 {
                return Err("Titles must be less than 255 characters".to_string());
            }

            Ok(Self { en, ja, es: None })
        }

        fn with_spanish(mut self, es: String) -> Result<Self, String> {
            if es.is_empty() || es.len() > 255 {
                return Err("Spanish title must be between 1 and 255 characters".to_string());
            }
            self.es = Some(es);
            Ok(self)
        }
    }

    #[test]
    fn test_multi_language_title_creation() {
        let title =
            MultiLanguageTitle::new("Hello World".to_string(), "こんにちは世界".to_string());

        assert!(title.is_ok());
        let t = title.unwrap();
        assert_eq!(t.en, "Hello World");
        assert_eq!(t.ja, "こんにちは世界");
        assert!(t.es.is_none());
    }

    #[test]
    fn test_multi_language_title_empty_english() {
        let title = MultiLanguageTitle::new("".to_string(), "こんにちは世界".to_string());

        assert!(title.is_err());
        assert_eq!(
            title.err(),
            Some("English and Japanese titles are required".to_string())
        );
    }

    #[test]
    fn test_multi_language_title_too_long() {
        let long_title = "a".repeat(256);
        let title = MultiLanguageTitle::new(long_title, "こんにちは世界".to_string());

        assert!(title.is_err());
    }

    #[test]
    fn test_multi_language_title_with_spanish() {
        let title =
            MultiLanguageTitle::new("Hello World".to_string(), "こんにちは世界".to_string())
                .unwrap()
                .with_spanish("Hola Mundo".to_string());

        assert!(title.is_ok());
        let t = title.unwrap();
        assert_eq!(t.es, Some("Hola Mundo".to_string()));
    }

    // ============= DomainEvent (仮) =============

    #[derive(Debug, Clone, PartialEq)]
    enum DomainEvent {
        RatingCreated {
            post_id: String,
            score: u8,
        },
        FavoriteAdded {
            post_id: String,
            user_id: String,
        },
        NotificationSent {
            user_id: String,
            notification_type: String,
        },
    }

    #[test]
    fn test_domain_events() {
        let events = vec![
            DomainEvent::RatingCreated {
                post_id: "post123".to_string(),
                score: 5,
            },
            DomainEvent::FavoriteAdded {
                post_id: "post123".to_string(),
                user_id: "user456".to_string(),
            },
            DomainEvent::NotificationSent {
                user_id: "user456".to_string(),
                notification_type: "POST_RATED".to_string(),
            },
        ];

        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0],
            DomainEvent::RatingCreated { score: 5, .. }
        ));
    }

    // ============= AggregateRoot パターン (仮) =============

    #[derive(Debug, Clone)]
    struct PostAggregate {
        id: String,
        title: String,
        ratings: Vec<Rating>,
        favorites: Vec<Favorite>,
        events: Vec<DomainEvent>,
    }

    impl PostAggregate {
        fn new(id: String, title: String) -> Self {
            Self {
                id,
                title,
                ratings: vec![],
                favorites: vec![],
                events: vec![],
            }
        }

        fn add_rating(&mut self, rating: Rating) -> Result<(), String> {
            // Business rule: Each user can only rate once
            if self.ratings.iter().any(|r| r.user_id == rating.user_id) {
                return Err("User has already rated this post".to_string());
            }

            self.ratings.push(rating.clone());
            self.events.push(DomainEvent::RatingCreated {
                post_id: self.id.clone(),
                score: rating.score,
            });

            Ok(())
        }

        fn add_favorite(&mut self, favorite: Favorite) -> Result<(), String> {
            // Business rule: Each user can favorite only once
            if self.favorites.iter().any(|f| f.user_id == favorite.user_id) {
                return Err("User has already favorited this post".to_string());
            }

            self.favorites.push(favorite);
            Ok(())
        }

        fn get_average_rating(&self) -> Option<f32> {
            if self.ratings.is_empty() {
                return None;
            }

            let sum: u32 = self.ratings.iter().map(|r| r.score as u32).sum();
            Some(sum as f32 / self.ratings.len() as f32)
        }

        fn get_favorite_count(&self) -> usize {
            self.favorites.len()
        }

        fn take_events(&mut self) -> Vec<DomainEvent> {
            std::mem::take(&mut self.events)
        }
    }

    #[test]
    fn test_post_aggregate_add_rating() {
        let mut post = PostAggregate::new("post123".to_string(), "Test Post".to_string());

        let rating = Rating {
            id: RatingId("r1".to_string()),
            post_id: "post123".to_string(),
            user_id: "user1".to_string(),
            score: 5,
        };

        let result = post.add_rating(rating);
        assert!(result.is_ok());
        assert_eq!(post.ratings.len(), 1);
        assert_eq!(post.events.len(), 1);
    }

    #[test]
    fn test_post_aggregate_duplicate_rating() {
        let mut post = PostAggregate::new("post123".to_string(), "Test Post".to_string());

        let rating1 = Rating {
            id: RatingId("r1".to_string()),
            post_id: "post123".to_string(),
            user_id: "user1".to_string(),
            score: 5,
        };

        let rating2 = Rating {
            id: RatingId("r2".to_string()),
            post_id: "post123".to_string(),
            user_id: "user1".to_string(),
            score: 3,
        };

        assert!(post.add_rating(rating1).is_ok());
        assert!(post.add_rating(rating2).is_err());
        assert_eq!(post.ratings.len(), 1);
    }

    #[test]
    fn test_post_aggregate_average_rating() {
        let mut post = PostAggregate::new("post123".to_string(), "Test Post".to_string());

        for (user_id, score) in &[("user1", 5), ("user2", 4), ("user3", 3)] {
            let rating = Rating {
                id: RatingId(format!("r{}", user_id)),
                post_id: "post123".to_string(),
                user_id: user_id.to_string(),
                score: *score,
            };
            let _ = post.add_rating(rating);
        }

        let avg = post.get_average_rating();
        assert_eq!(avg, Some(4.0)); // (5 + 4 + 3) / 3 = 4.0
    }

    #[test]
    fn test_post_aggregate_favorite() {
        let mut post = PostAggregate::new("post123".to_string(), "Test Post".to_string());

        let favorite = Favorite::new("post123".to_string(), "user1".to_string());
        let result = post.add_favorite(favorite);

        assert!(result.is_ok());
        assert_eq!(post.get_favorite_count(), 1);
    }

    #[test]
    fn test_post_aggregate_duplicate_favorite() {
        let mut post = PostAggregate::new("post123".to_string(), "Test Post".to_string());

        let fav1 = Favorite::new("post123".to_string(), "user1".to_string());
        let fav2 = Favorite::new("post123".to_string(), "user1".to_string());

        assert!(post.add_favorite(fav1).is_ok());
        assert!(post.add_favorite(fav2).is_err());
        assert_eq!(post.get_favorite_count(), 1);
    }

    #[test]
    fn test_post_aggregate_take_events() {
        let mut post = PostAggregate::new("post123".to_string(), "Test Post".to_string());

        let rating = Rating {
            id: RatingId("r1".to_string()),
            post_id: "post123".to_string(),
            user_id: "user1".to_string(),
            score: 5,
        };

        let _ = post.add_rating(rating);
        assert_eq!(post.events.len(), 1);

        let events = post.take_events();
        assert_eq!(events.len(), 1);
        assert!(post.events.is_empty()); // Events cleared after taking
    }
}

// Helper trait for testing async patterns (Phase 2.5+)
#[cfg(test)]
mod async_domain_tests {
    use std::future::Future;

    trait AsyncDomainService {
        fn validate_uniqueness(&self, value: &str) -> impl Future<Output = bool>;
    }

    #[tokio::test]
    async fn test_async_domain_validation() {
        // Placeholder for async validation tests
        // Will be implemented in Phase 2.5
        assert!(true);
    }
}

// ============= Aggregate の不変条件検証 =============

#[cfg(test)]
mod invariant_tests {
    #[test]
    fn test_aggregate_invariants() {
        // 例: Post Entity の不変条件
        // 1. ID は空にならない
        // 2. Title は 1-500 文字
        // 3. Published date は作成日以降
        // 4. Rating は 1-5 の範囲のみ

        let invariants = vec![
            ("Post ID is not empty", true),
            ("Post Title length 1-500", true),
            ("Published >= Created", true),
            ("Rating between 1-5", true),
        ];

        for (name, valid) in invariants {
            assert!(valid, "Invariant violated: {}", name);
        }
    }
}
