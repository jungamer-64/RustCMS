use std::sync::Arc;

use crate::application::ports::UserRepository;

/// Create user use case: delegates creation to repository port.
pub struct CreateUserUseCase<R: UserRepository> {
    repo: Arc<R>,
}

impl<R: UserRepository> CreateUserUseCase<R> {
    #[must_use]
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        req: crate::models::CreateUserRequest,
    ) -> Result<R::User, crate::application::ports::user_repository::RepositoryError> {
        self.repo.create(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::User as ModelUser;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockRepo {
        map: Mutex<HashMap<uuid::Uuid, ModelUser>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                map: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl crate::application::ports::UserRepository for MockRepo {
        type User = ModelUser;

        async fn find_by_id(
            &self,
            _id: crate::domain::value_objects::UserId,
        ) -> Result<Option<Self::User>, crate::application::ports::user_repository::RepositoryError>
        {
            Ok(None)
        }
        async fn find_by_email(
            &self,
            _email: &crate::domain::value_objects::Email,
        ) -> Result<Option<Self::User>, crate::application::ports::user_repository::RepositoryError>
        {
            Ok(None)
        }
        async fn create(
            &self,
            request: crate::models::CreateUserRequest,
        ) -> Result<Self::User, crate::application::ports::user_repository::RepositoryError>
        {
            let now = chrono::Utc::now();
            let user = ModelUser {
                id: uuid::Uuid::new_v4(),
                username: request.username,
                email: request.email,
                password_hash: Some("hash".to_string()),
                first_name: request.first_name,
                last_name: request.last_name,
                role: request.role.as_str().to_string(),
                is_active: true,
                email_verified: false,
                last_login: None,
                created_at: now,
                updated_at: now,
            };
            self.map.lock().unwrap().insert(user.id, user.clone());
            Ok(user)
        }
        async fn update(
            &self,
            _id: crate::domain::value_objects::UserId,
            _request: crate::models::UpdateUserRequest,
        ) -> Result<Self::User, crate::application::ports::user_repository::RepositoryError>
        {
            Err(
                crate::application::ports::user_repository::RepositoryError::Unexpected(
                    "not implemented".to_string(),
                ),
            )
        }
        async fn save(
            &self,
            _user: &Self::User,
        ) -> Result<(), crate::application::ports::user_repository::RepositoryError> {
            Ok(())
        }
        async fn delete(
            &self,
            _id: crate::domain::value_objects::UserId,
        ) -> Result<(), crate::application::ports::user_repository::RepositoryError> {
            Ok(())
        }

        async fn find_paginated(
            &self,
            _page: u32,
            _per_page: u32,
            _role: Option<String>,
            _active: Option<bool>,
            _sort: Option<String>,
        ) -> Result<Vec<Self::User>, crate::application::ports::user_repository::RepositoryError>
        {
            Ok(Vec::new())
        }

        async fn count_filtered(
            &self,
            _role: Option<String>,
            _active: Option<bool>,
        ) -> Result<usize, crate::application::ports::user_repository::RepositoryError> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn create_user_ok() {
        let repo = std::sync::Arc::new(MockRepo::new());
        let uc = CreateUserUseCase::new(repo);
        let req = crate::models::CreateUserRequest {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "Password123".to_string(),
            first_name: None,
            last_name: None,
            role: crate::models::UserRole::Subscriber,
        };
        let created = uc.execute(req).await.unwrap();
        assert_eq!(created.username, "alice");
    }
}
