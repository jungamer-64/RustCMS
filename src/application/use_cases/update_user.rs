use std::sync::Arc;

use crate::application::ports::UserRepository;

pub struct UpdateUserUseCase<R: UserRepository> {
    repo: Arc<R>,
}

impl<R: UserRepository> UpdateUserUseCase<R> {
    #[must_use]
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        id: crate::domain::value_objects::UserId,
        req: crate::models::UpdateUserRequest,
    ) -> Result<R::User, crate::application::ports::user_repository::RepositoryError> {
        self.repo.update(id, req).await
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
            _request: crate::models::CreateUserRequest,
        ) -> Result<Self::User, crate::application::ports::user_repository::RepositoryError>
        {
            Err(
                crate::application::ports::user_repository::RepositoryError::Unexpected(
                    "not impl".to_string(),
                ),
            )
        }
        async fn update(
            &self,
            id: crate::domain::value_objects::UserId,
            request: crate::models::UpdateUserRequest,
        ) -> Result<Self::User, crate::application::ports::user_repository::RepositoryError>
        {
            let mut map = self.map.lock().unwrap();
            let uid = *id.as_uuid();
            let now = chrono::Utc::now();
            let user = ModelUser {
                id: uid,
                username: request.username.unwrap_or_else(|| "bob".to_string()),
                email: request
                    .email
                    .unwrap_or_else(|| "bob@example.com".to_string()),
                password_hash: Some("hash".to_string()),
                first_name: request.first_name.clone(),
                last_name: request.last_name.clone(),
                role: request
                    .role
                    .map(|r| r.as_str().to_string())
                    .unwrap_or_else(|| "subscriber".to_string()),
                is_active: request.is_active.unwrap_or(true),
                email_verified: false,
                last_login: None,
                created_at: now,
                updated_at: now,
            };
            map.insert(uid, user.clone());
            Ok(user)
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
    async fn update_user_ok() {
        let repo = std::sync::Arc::new(MockRepo::new());
        let uc = UpdateUserUseCase::new(repo);
        let id = crate::domain::value_objects::UserId::new();
        let req = crate::models::UpdateUserRequest {
            username: Some("bob".to_string()),
            email: None,
            first_name: None,
            last_name: None,
            role: None,
            is_active: None,
        };
        let updated = uc.execute(id, req).await.unwrap();
        assert_eq!(updated.username, "bob");
    }
}
