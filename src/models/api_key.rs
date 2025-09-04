use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use validator::{ValidationError, ValidationErrors};
use base64::Engine; // for URL_SAFE_NO_PAD.encode

use crate::database::schema::api_keys;
use crate::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Queryable, Insertable)]
#[diesel(table_name = api_keys)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    /// APIキーの高速ルックアップ用 (deterministic, non-salted). SHA-256(base64url) など。
    pub api_key_lookup_hash: String,
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
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ApiKey {
    /// 利用可能なパーミッション列挙（将来必要になれば拡張）
    pub const ALLOWED_PERMISSIONS: &'static [&'static str] = &[
        "posts:read",
        "posts:write",
        "users:read",
        "users:write",
        "search:reindex",
    ];

    fn validate_permissions(perms: &[String]) -> Result<(), crate::AppError> {
        if perms.is_empty() {
            return Ok(()); // 空は許可（最小権限）
        }
        let mut invalid: Vec<String> = perms
            .iter()
            .filter(|p| !Self::ALLOWED_PERMISSIONS.contains(&p.as_str()))
            .cloned()
            .collect();
        if invalid.is_empty() {
            return Ok(());
        }
        let mut errors = ValidationErrors::new();
        for inv in invalid.drain(..) {
            let mut ve = ValidationError::new("invalid_permission");
            ve.add_param("value".into(), &inv);
            errors.add("permissions", ve);
        }
        Err(crate::AppError::Validation(errors))
    }

    /// バリデーション付きコンストラクタ
    pub fn new_validated(name: String, user_id: Uuid, permissions: Vec<String>) -> Result<(Self, String), crate::AppError> {
        Self::validate_permissions(&permissions)?;
        Ok(Self::new(name, user_id, permissions))
    }

    pub fn new(name: String, user_id: Uuid, permissions: Vec<String>) -> (Self, String) {
        let now = Utc::now();
    let raw_key = Self::generate_key();
    let key_hash = Self::hash_key(&raw_key);
    let api_key_lookup_hash = Self::lookup_hash(&raw_key);

        let api_key = Self {
            id: Uuid::new_v4(),
            name,
            key_hash,
            user_id,
            api_key_lookup_hash,
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

    pub fn find_by_lookup_hash(
        conn: &mut crate::database::PooledConnection,
        lookup: &str,
    ) -> Result<ApiKey, AppError> {
        use crate::database::schema::api_keys::dsl::*;
        api_keys
            .filter(api_key_lookup_hash.eq(lookup))
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

    pub fn list_for_user(
        conn: &mut crate::database::PooledConnection,
        target_user_id: Uuid,
        include_expired: bool,
    ) -> Result<Vec<ApiKey>, AppError> {
        use crate::database::schema::api_keys::dsl::*;
        let mut query = api_keys.filter(user_id.eq(target_user_id)).into_boxed();
        if !include_expired {
            let now = Utc::now();
            // keep rows where expires_at is null OR expires_at > now
            query = query.filter(expires_at.is_null().or(expires_at.gt(now)));
        }
        query
            .order(created_at.desc())
            .load::<ApiKey>(conn)
            .map_err(AppError::from)
    }

    fn generate_key() -> String {
        // Prefixで種別を明示し、将来のローテ/種別拡張時に識別しやすくする
        const PREFIX: &str = "ak_"; // api key
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"; // URL safe
        use rand::{Rng, rng};
        let mut r = rng();
        let body: String = (0..56) // 56 chars + prefix ≒ 336 bits entropy (十分な強度)
            .map(|_| {
                let idx = r.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        format!("{}{}", PREFIX, body)
    }

    pub(crate) fn hash_key(key: &str) -> String {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        // NOTE: 現状は毎回新規 salt + 再ハッシュとなるため 生キー→保存済ハッシュ の照合には不向き。
        // Middleware での検索用には KDF を固定化 (pepper + deterministic) する別カラム導入を検討。
        argon2
            .hash_password(key.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .unwrap_or_default()
    }

    /// ルックアップ用の決定的ハッシュ (衝突耐性と高速性重視 / 再計算可能)。
    /// 生キーが漏れてもハッシュ逆算は困難だが、オフライン総当りは可能なので rate-limit 前提。
    pub fn lookup_hash(key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let digest = hasher.finalize();
        // URL safe base64 (no padding) で保存長を抑える
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
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

    /// 期限切れかどうか (expires_at があり、それが現在時刻を過ぎている)
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.map(|e| e <= now).unwrap_or(false)
    }

    /// 応答用構造体へ変換
    pub fn to_response(&self) -> ApiKeyResponse {
        ApiKeyResponse {
            id: self.id,
            name: self.name.clone(),
            permissions: self.get_permissions(),
            user_id: self.user_id,
            created_at: self.created_at,
            expires_at: self.expires_at,
            last_used_at: self.last_used_at,
        }
    }

    /// ログ出力や監査向けにキー本体を暴露しない短縮表示
    pub fn mask_raw(raw: &str) -> String {
        if raw.len() <= 10 { return "***".into(); }
        format!("{}…{}", &raw[..6], &raw[raw.len()-4..])
    }
}
