//! Integration tests for use-case pipeline
//!
//! Validates the complete flow: AppContainer → use-case → handler → response
//! Tests ensure proper wiring and error handling across layers.

#![cfg(all(test, feature = "database"))]

use cms_backend::{
    AppState, Result,
    application::{
        ports::user_repository::RepositoryError,
        use_cases::{CreateUserUseCase, GetUserByIdUseCase, ListUsersUseCase},
    },
    domain::value_objects::UserId,
    infrastructure::repositories::DieselUserRepository,
    models::{CreateUserRequest, UserRole},
};
use std::sync::Arc;
use uuid::Uuid;

// Mock infrastructure for testing without DB
mod mocks {
    use super::*;
    use async_trait::async_trait;

    #[derive(Clone)]
    pub struct MockUserRepository {
        users: Arc<std::sync::Mutex<Vec<cms_backend::models::User>>>,
    }

    impl MockUserRepository {
        pub fn new() -> Self {
            Self {
                users: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        pub fn add_user(&self, user: cms_backend::models::User) {
            self.users.lock().unwrap().push(user);
        }
    }

    #[async_trait]
    impl cms_backend::application::ports::UserRepository for MockUserRepository {
        type User = cms_backend::models::User;

        async fn find_by_id(
            &self,
            id: UserId,
        ) -> std::result::Result<Option<Self::User>, RepositoryError> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.id == *id.as_uuid())
                .cloned())
        }

        async fn find_by_email(
            &self,
            email: &cms_backend::domain::value_objects::Email,
        ) -> std::result::Result<Option<Self::User>, RepositoryError> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.email == email.as_str())
                .cloned())
        }

        async fn create(
            &self,
            request: CreateUserRequest,
        ) -> std::result::Result<Self::User, RepositoryError> {
            let user = cms_backend::models::User {
                id: Uuid::new_v4(),
                username: request.username,
                email: request.email,
                password_hash: Some("hash".to_string()),
                first_name: request.first_name,
                last_name: request.last_name,
                role: request.role.as_str().to_string(),
                is_active: true,
                email_verified: false,
                last_login: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.add_user(user.clone());
            Ok(user)
        }

        async fn update(
            &self,
            _id: UserId,
            _request: cms_backend::models::UpdateUserRequest,
        ) -> std::result::Result<Self::User, RepositoryError> {
            Err(RepositoryError::Unexpected("not implemented".to_string()))
        }

        async fn save(&self, _user: &Self::User) -> std::result::Result<(), RepositoryError> {
            Ok(())
        }

        async fn delete(&self, _id: UserId) -> std::result::Result<(), RepositoryError> {
            Ok(())
        }

        async fn find_paginated(
            &self,
            page: u32,
            per_page: u32,
            _role: Option<String>,
            _active: Option<bool>,
            _sort: Option<String>,
        ) -> std::result::Result<Vec<Self::User>, RepositoryError> {
            let users = self.users.lock().unwrap();
            let offset = ((page.saturating_sub(1)) * per_page) as usize;
            let limit = per_page as usize;
            Ok(users.iter().skip(offset).take(limit).cloned().collect())
        }

        async fn count_filtered(
            &self,
            _role: Option<String>,
            _active: Option<bool>,
        ) -> std::result::Result<usize, RepositoryError> {
            Ok(self.users.lock().unwrap().len())
        }
    }
}

#[tokio::test]
async fn test_get_user_by_id_use_case() {
    let mock_repo = Arc::new(mocks::MockUserRepository::new());

    // Setup: Create a test user
    let user_id = Uuid::new_v4();
    let test_user = cms_backend::models::User {
        id: user_id,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        password_hash: Some("hash".to_string()),
        first_name: Some("Alice".to_string()),
        last_name: None,
        role: "subscriber".to_string(),
        is_active: true,
        email_verified: false,
        last_login: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    mock_repo.add_user(test_user.clone());

    // Create use-case
    let uc = GetUserByIdUseCase::new(mock_repo);

    // Execute
    let result = uc.execute(UserId::from_uuid(user_id)).await;

    // Verify
    assert!(result.is_ok());
    let found_user = result.unwrap();
    assert!(found_user.is_some());
    assert_eq!(found_user.unwrap().id, user_id);
}

#[tokio::test]
async fn test_get_user_by_id_not_found() {
    let mock_repo = Arc::new(mocks::MockUserRepository::new());
    let uc = GetUserByIdUseCase::new(mock_repo);

    // Execute with non-existent ID
    let result = uc.execute(UserId::new()).await;

    // Verify
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_list_users_use_case() {
    let mock_repo = Arc::new(mocks::MockUserRepository::new());

    // Setup: Create multiple test users
    for i in 0..5 {
        let user = cms_backend::models::User {
            id: Uuid::new_v4(),
            username: format!("user{}", i),
            email: format!("user{}@example.com", i),
            password_hash: Some("hash".to_string()),
            first_name: None,
            last_name: None,
            role: "subscriber".to_string(),
            is_active: true,
            email_verified: false,
            last_login: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        mock_repo.add_user(user);
    }

    // Create use-case
    let uc = ListUsersUseCase::new(mock_repo);

    // Execute: Get first page with 2 items per page
    let result = uc.execute(1, 2, None, None, None).await;

    // Verify
    assert!(result.is_ok());
    let (users, total) = result.unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn test_list_users_pagination() {
    let mock_repo = Arc::new(mocks::MockUserRepository::new());

    // Setup: Create 10 test users
    for i in 0..10 {
        let user = cms_backend::models::User {
            id: Uuid::new_v4(),
            username: format!("user{}", i),
            email: format!("user{}@example.com", i),
            password_hash: Some("hash".to_string()),
            first_name: None,
            last_name: None,
            role: if i % 2 == 0 { "admin" } else { "subscriber" }.to_string(),
            is_active: i < 8,
            email_verified: false,
            last_login: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        mock_repo.add_user(user);
    }

    let uc = ListUsersUseCase::new(mock_repo);

    // Test page 1
    let (page1, total) = uc.execute(1, 3, None, None, None).await.unwrap();
    assert_eq!(page1.len(), 3);
    assert_eq!(total, 10);

    // Test page 2
    let (page2, _) = uc.execute(2, 3, None, None, None).await.unwrap();
    assert_eq!(page2.len(), 3);

    // Test page 4 (partial)
    let (page4, _) = uc.execute(4, 3, None, None, None).await.unwrap();
    assert_eq!(page4.len(), 1);

    // Verify no overlap between pages
    let page1_ids: Vec<_> = page1.iter().map(|u| u.id).collect();
    let page2_ids: Vec<_> = page2.iter().map(|u| u.id).collect();
    for id in &page1_ids {
        assert!(!page2_ids.contains(id));
    }
}

#[tokio::test]
async fn test_create_user_use_case() {
    let mock_repo = Arc::new(mocks::MockUserRepository::new());
    let uc = CreateUserUseCase::new(mock_repo.clone());

    let request = CreateUserRequest {
        username: "bob".to_string(),
        email: "bob@example.com".to_string(),
        password: "SecurePass123".to_string(),
        first_name: Some("Bob".to_string()),
        last_name: None,
        role: UserRole::Subscriber,
    };

    // Execute
    let result = uc.execute(request).await;

    // Verify
    assert!(result.is_ok());
    let created_user = result.unwrap();
    assert_eq!(created_user.username, "bob");
    assert_eq!(created_user.email, "bob@example.com");
    assert_eq!(created_user.role, "subscriber");
}

#[tokio::test]
async fn test_use_case_error_handling() {
    let mock_repo = Arc::new(mocks::MockUserRepository::new());

    // Setup: Add a user
    let user = cms_backend::models::User {
        id: Uuid::new_v4(),
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        password_hash: Some("hash".to_string()),
        first_name: None,
        last_name: None,
        role: "subscriber".to_string(),
        is_active: true,
        email_verified: false,
        last_login: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    mock_repo.add_user(user);

    let uc = GetUserByIdUseCase::new(mock_repo);

    // Test: Get existing user (OK)
    let existing_id = UserId::from_uuid(user.id);
    let result = uc.execute(existing_id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());

    // Test: Get non-existing user (None, not error)
    let missing_id = UserId::new();
    let result = uc.execute(missing_id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_container_wiring_simulation() {
    // This test simulates how AppContainer wires use-cases together
    let mock_repo = Arc::new(mocks::MockUserRepository::new());

    // Setup test data
    for i in 0..3 {
        let user = cms_backend::models::User {
            id: Uuid::new_v4(),
            username: format!("user{}", i),
            email: format!("user{}@example.com", i),
            password_hash: Some("hash".to_string()),
            first_name: None,
            last_name: None,
            role: "subscriber".to_string(),
            is_active: true,
            email_verified: false,
            last_login: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        mock_repo.add_user(user);
    }

    // Simulate container construction
    let get_user_uc = GetUserByIdUseCase::new(mock_repo.clone());
    let list_users_uc = ListUsersUseCase::new(mock_repo.clone());
    let create_user_uc = CreateUserUseCase::new(mock_repo);

    // Test all use-cases work together
    let (all_users, count) = list_users_uc
        .execute(1, 10, None, None, None)
        .await
        .unwrap();
    assert_eq!(all_users.len(), 3);
    assert_eq!(count, 3);

    // Fetch individual user via use-case
    let first_user = get_user_uc
        .execute(UserId::from_uuid(all_users[0].id))
        .await
        .unwrap();
    assert!(first_user.is_some());
    assert_eq!(first_user.unwrap().id, all_users[0].id);

    // Create new user
    let new_user_req = CreateUserRequest {
        username: "newuser".to_string(),
        email: "new@example.com".to_string(),
        password: "Pass123".to_string(),
        first_name: None,
        last_name: None,
        role: UserRole::Editor,
    };
    let created = create_user_uc.execute(new_user_req).await.unwrap();
    assert_eq!(created.username, "newuser");
}
