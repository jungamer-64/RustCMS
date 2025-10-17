//! DieselUserRepository - UserRepository trait の Diesel 実装
//!
//! ## 責務
//! - UserRepository Port の具体的実装
//! - Domain User Entity ↔ Diesel DbUser モデルのマッピング
//! - データベースエラーの InfrastructureError への変換
//!
//! ## 設計原則
//! - Domain層に依存、Domain層はこの層に依存しない（依存性の逆転）
//! - 全変換ロジックをこのモジュールに局所化
//! - Connection Pool を通じてトランザクション管理

use crate::application::ports::repositories::{RepositoryError, UserRepository};
use crate::common::types::InfrastructureError;
use crate::database::schema::users;
use crate::domain::user::{Email, EmailError, User, UserId, Username, UsernameError};
use crate::infrastructure::database::models::{DbUser, NewDbUser};
use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use uuid::Uuid;

/// Diesel ベースの UserRepository 実装
///
/// PostgreSQL との接続を管理し、Domain User Entity と DB モデルの変換を行います。
pub struct DieselUserRepository {
    pool: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl DieselUserRepository {
    /// 新しい DieselUserRepository を作成
    #[must_use]
    pub fn new(pool: Arc<Pool<ConnectionManager<PgConnection>>>) -> Self {
        Self { pool }
    }

    /// DbUser から Domain User Entity への変換
    ///
    /// # Errors
    ///
    /// - Email または Username の検証に失敗した場合
    fn db_user_to_domain(db_user: DbUser) -> Result<User, RepositoryError> {
        let user_id = UserId::from_uuid(db_user.id);
        let email = Email::new(db_user.email).map_err(|e| match e {
            EmailError::Empty => {
                RepositoryError::ConversionError("Email cannot be empty".to_string())
            }
            EmailError::MissingAtSign => {
                RepositoryError::ConversionError("Email must contain @".to_string())
            }
            EmailError::TooLong => {
                RepositoryError::ConversionError("Email exceeds 254 characters".to_string())
            }
        })?;
        let username = Username::new(db_user.username).map_err(|e| match e {
            UsernameError::Empty => {
                RepositoryError::ConversionError("Username cannot be empty".to_string())
            }
            UsernameError::TooLong => {
                RepositoryError::ConversionError("Username exceeds 50 characters".to_string())
            }
            UsernameError::InvalidCharacters => RepositoryError::ConversionError(
                "Username contains invalid characters".to_string(),
            ),
        })?;

        Ok(User::restore(user_id, username, email, db_user.is_active))
    }

    /// Domain User Entity から NewDbUser への変換
    fn domain_user_to_new_db(user: &User) -> NewDbUser {
        NewDbUser {
            id: user.id().into_uuid(),
            username: user.username().as_str().to_string(),
            email: user.email().as_str().to_string(),
            password_hash: None, // TODO: パスワードハッシュ管理は別途実装
            first_name: None,
            last_name: None,
            role: "user".to_string(), // デフォルトロール
            is_active: user.is_active(),
            email_verified: false,
            last_login: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    /// ユーザーを保存（作成または更新）
    async fn save(&self, user: User) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let user_id = user.id().into_uuid();
        let new_db_user = Self::domain_user_to_new_db(&user);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            // UPSERT: ON CONFLICT DO UPDATE
            diesel::insert_into(users::table)
                .values(&new_db_user)
                .on_conflict(users::id)
                .do_update()
                .set((
                    users::username.eq(&new_db_user.username),
                    users::email.eq(&new_db_user.email),
                    users::is_active.eq(new_db_user.is_active),
                    users::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to save user: {}", e))
                })?;

            Ok(())
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// IDでユーザーを検索
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let user_uuid = id.into_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_user = users::table
                .filter(users::id.eq(user_uuid))
                .first::<DbUser>(&mut conn)
                .optional()
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to find user by id: {}", e))
                })?;

            match db_user {
                Some(db_user) => Ok(Some(Self::db_user_to_domain(db_user)?)),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// メールアドレスでユーザーを検索
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let email_str = email.as_str().to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_user = users::table
                .filter(users::email.eq(&email_str))
                .first::<DbUser>(&mut conn)
                .optional()
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to find user by email: {}", e))
                })?;

            match db_user {
                Some(db_user) => Ok(Some(Self::db_user_to_domain(db_user)?)),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// ユーザーを削除
    async fn delete(&self, id: UserId) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let user_uuid = id.into_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            diesel::delete(users::table.filter(users::id.eq(user_uuid)))
                .execute(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to delete user: {}", e))
                })?;

            Ok(())
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// 全ユーザーを取得（ページネーション対応）
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_users = users::table
                .order(users::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load::<DbUser>(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to list users: {}", e))
                })?;

            db_users
                .into_iter()
                .map(Self::db_user_to_domain)
                .collect::<Result<Vec<User>, RepositoryError>>()
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{Email, Username};

    #[test]
    fn test_domain_user_to_new_db_conversion() {
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        let new_db_user = DieselUserRepository::domain_user_to_new_db(&user);

        assert_eq!(new_db_user.id, user.id().into_uuid());
        assert_eq!(new_db_user.username, "testuser");
        assert_eq!(new_db_user.email, "test@example.com");
        assert!(new_db_user.is_active);
        assert_eq!(new_db_user.role, "user");
    }

    #[test]
    fn test_db_user_to_domain_conversion_success() {
        let db_user = DbUser {
            id: Uuid::new_v4(),
            username: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: Some("hash123".to_string()),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            role: "admin".to_string(),
            is_active: true,
            email_verified: true,
            last_login: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let user = DieselUserRepository::db_user_to_domain(db_user.clone()).unwrap();

        assert_eq!(user.id().into_uuid(), db_user.id);
        assert_eq!(user.username().as_str(), "john_doe");
        assert_eq!(user.email().as_str(), "john@example.com");
        assert!(user.is_active());
    }

    #[test]
    fn test_db_user_to_domain_conversion_invalid_email() {
        let db_user = DbUser {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "invalid-email".to_string(), // No @ sign
            password_hash: None,
            first_name: None,
            last_name: None,
            role: "user".to_string(),
            is_active: true,
            email_verified: false,
            last_login: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = DieselUserRepository::db_user_to_domain(db_user);

        assert!(result.is_err());
        match result {
            Err(RepositoryError::ConversionError(msg)) => {
                assert!(msg.contains("Email must contain @"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_db_user_to_domain_conversion_invalid_username() {
        let db_user = DbUser {
            id: Uuid::new_v4(),
            username: "".to_string(), // Empty username
            email: "test@example.com".to_string(),
            password_hash: None,
            first_name: None,
            last_name: None,
            role: "user".to_string(),
            is_active: true,
            email_verified: false,
            last_login: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = DieselUserRepository::db_user_to_domain(db_user);

        assert!(result.is_err());
        match result {
            Err(RepositoryError::ConversionError(msg)) => {
                assert!(msg.contains("Username cannot be empty"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }
}
