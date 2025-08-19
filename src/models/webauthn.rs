use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::schema::webauthn_credentials;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Associations, ToSchema)]
#[diesel(belongs_to(User))]
#[diesel(table_name = webauthn_credentials)]
pub struct WebAuthnCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub counter: i64,
    pub name: String,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, ToSchema)]
#[diesel(table_name = webauthn_credentials)]
pub struct NewWebAuthnCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub counter: i64,
    pub name: String,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = webauthn_credentials)]
pub struct UpdateWebAuthnCredential {
    pub counter: Option<i64>,
    pub last_used: Option<DateTime<Utc>>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebAuthnCredentialResponse {
    pub id: Uuid,
    pub name: String,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

impl From<WebAuthnCredential> for WebAuthnCredentialResponse {
    fn from(cred: WebAuthnCredential) -> Self {
        Self {
            id: cred.id,
            name: cred.name,
            backup_eligible: cred.backup_eligible,
            backup_state: cred.backup_state,
            created_at: cred.created_at,
            last_used: cred.last_used,
        }
    }
}

impl WebAuthnCredential {
    pub fn new(
        user_id: Uuid,
        credential_id: Vec<u8>,
        public_key: Vec<u8>,
        name: String,
        backup_eligible: bool,
        backup_state: bool,
    ) -> NewWebAuthnCredential {
        NewWebAuthnCredential {
            id: Uuid::new_v4(),
            user_id,
            credential_id,
            public_key,
            counter: 0,
            name,
            backup_eligible,
            backup_state,
            created_at: Utc::now(),
        }
    }
}

use crate::models::user::User;
