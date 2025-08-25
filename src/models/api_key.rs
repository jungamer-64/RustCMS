use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::database::schema::api_keys;
use crate::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Queryable, Insertable)]
#[diesel(table_name = api_keys)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub user_id: Uuid,
    pub permissions: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn new(name: String, user_id: Uuid, permissions: Vec<String>) -> (Self, String) {
        let now = Utc::now();
        let raw_key = Self::generate_key();
        let key_hash = Self::hash_key(&raw_key);

        let api_key = Self {
            id: Uuid::new_v4(),
            name,
            key_hash,
            user_id,
            permissions: serde_json::to_value(permissions).unwrap_or_default(),
            created_at: now,
            updated_at: now,
            expires_at: None,
            last_used_at: None,
        };

        (api_key, raw_key)
    }

    pub fn create(
        conn: &mut crate::database::PooledConnection,
        api_key: &ApiKey,
    ) -> Result<ApiKey, AppError> {
        use crate::database::schema::api_keys;
        diesel::insert_into(api_keys::table)
            .values(api_key)
            .get_result(conn)
            .map_err(AppError::from)
    }

    pub fn find_by_id(
        conn: &mut crate::database::PooledConnection,
        api_key_id: Uuid,
    ) -> Result<ApiKey, AppError> {
        use crate::database::schema::api_keys::dsl::*;
        api_keys
            .find(api_key_id)
            .first(conn)
            .map_err(AppError::from)
    }

    pub fn find_by_key_hash(
        conn: &mut crate::database::PooledConnection,
        hash: &str,
    ) -> Result<ApiKey, AppError> {
        use crate::database::schema::api_keys::dsl::*;
        api_keys
            .filter(key_hash.eq(hash))
            .first(conn)
            .map_err(AppError::from)
    }

    pub fn delete(
        conn: &mut crate::database::PooledConnection,
        api_key_id: Uuid,
    ) -> Result<usize, AppError> {
        use crate::database::schema::api_keys::dsl::*;
        diesel::delete(api_keys.find(api_key_id))
            .execute(conn)
            .map_err(AppError::from)
    }

    pub fn update_last_used(
        conn: &mut crate::database::PooledConnection,
        api_key_id: Uuid,
    ) -> Result<(), AppError> {
        use crate::database::schema::api_keys::dsl::*;
        diesel::update(api_keys.find(api_key_id))
            .set(last_used_at.eq(Some(Utc::now())))
            .execute(conn)?;
        Ok(())
    }

    fn generate_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
        let mut rng = rand::thread_rng();

        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    fn hash_key(key: &str) -> String {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(key.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .unwrap_or_default()
    }

    pub fn verify_key(&self, key: &str) -> Result<bool, AppError> {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(&self.key_hash)
            .map_err(|e| AppError::Authentication(format!("Invalid hash format: {}", e)))?;

        Ok(Argon2::default()
            .verify_password(key.as_bytes(), &parsed_hash)
            .is_ok())
    }

    pub fn get_permissions(&self) -> Vec<String> {
        self.permissions
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}
