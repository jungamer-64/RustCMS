use std::sync::Arc;

// Use fully-qualified attribute to avoid unused-import lints in some build configs

use crate::application::ports::UserRepository;
use crate::domain::value_objects::UserId;

/// Simple use-case that retrieves a user by id via the repository port.
pub struct GetUserByIdUseCase<R: UserRepository> {
    repo: Arc<R>,
}

impl<R: UserRepository> GetUserByIdUseCase<R> {
    #[must_use]
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Execute the use case: return Ok(Some(user)) when found, Ok(None)
    /// when not found, or Err when the repository fails.
    pub async fn execute(
        &self,
        id: UserId,
    ) -> Result<Option<R::User>, crate::application::ports::user_repository::RepositoryError> {
        self.repo.find_by_id(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::models::User as ModelUser;

    struct MockUserRepo {
        map: Mutex<HashMap<UserId, ModelUser>>,
    }

    impl MockUserRepo {
        fn new() -> Self {
            Self {
                map: Mutex::new(HashMap::new()),
            }
        }

        fn insert(&self, id: UserId, user: ModelUser) {
            self.map.lock().unwrap().insert(id, user);
        }
    }

    #[async_trait::async_trait]
    impl crate::application::ports::UserRepository for MockUserRepo {
        type User = ModelUser;

        async fn find_by_id(
            &self,
            id: UserId,
        ) -> Result<Option<Self::User>, crate::application::ports::user_repository::RepositoryError>
        {
            Ok(self.map.lock().unwrap().get(&id).cloned())
        }

        async fn find_by_email(
            &self,
            _email: &crate::domain::value_objects::Email,
        ) -> Result<Option<Self::User>, crate::application::ports::user_repository::RepositoryError>
        {
            Ok(None)
        }

        async fn save(
            &self,
            _user: &Self::User,
        ) -> Result<(), crate::application::ports::user_repository::RepositoryError> {
            Ok(())
        }

        async fn delete(
            &self,
            _id: UserId,
        ) -> Result<(), crate::application::ports::user_repository::RepositoryError> {
            Ok(())
        }

        async fn create(
            &self,
            _request: crate::models::CreateUserRequest,
        ) -> Result<Self::User, crate::application::ports::user_repository::RepositoryError>
        {
            Err(
                crate::application::ports::user_repository::RepositoryError::Unexpected(
                    "not implemented".to_string(),
                ),
            )
        }

        async fn update(
            &self,
            _id: UserId,
            _request: crate::models::UpdateUserRequest,
        ) -> Result<Self::User, crate::application::ports::user_repository::RepositoryError>
        {
            Err(
                crate::application::ports::user_repository::RepositoryError::Unexpected(
                    "not implemented".to_string(),
                ),
            )
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
    async fn get_user_by_id_found() {
        let mock = Arc::new(MockUserRepo::new());
        let id = crate::domain::value_objects::UserId::new();
        let now = chrono::Utc::now();
        let user = ModelUser {
            id: uuid::Uuid::new_v4(),
            username: "u1".to_string(),
            email: "u1@example.com".to_string(),
            password_hash: Some("hash".to_string()),
            first_name: None,
            last_name: None,
            role: "subscriber".to_string(),
            is_active: true,
            email_verified: false,
            last_login: None,
            created_at: now,
            updated_at: now,
        };
        mock.insert(id, user.clone());

        let uc = GetUserByIdUseCase::new(mock.clone());
        let res = uc.execute(id).await.unwrap();
        assert!(res.is_some());
        assert_eq!(res.unwrap().id, user.id);
    }

    #[tokio::test]
    async fn get_user_by_id_missing() {
        let mock = Arc::new(MockUserRepo::new());
        let id = crate::domain::value_objects::UserId::new();
        let uc = GetUserByIdUseCase::new(mock.clone());
        let res = uc.execute(id).await.unwrap();
        assert!(res.is_none());
    }
}
