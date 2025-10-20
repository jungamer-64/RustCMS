use crate::auth::error::AuthError;
use crate::common::type_utils::common_types::SessionId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

#[cfg(feature = "restructure_domain")]
use domain::user::UserRole;

#[cfg(not(feature = "restructure_domain"))]
use crate::models::UserRole;

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub refresh_version: u32, // 現在有効な refresh token version
}

type SessionResult<T> = std::result::Result<T, AuthError>;

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn insert(&self, id: SessionId, data: SessionData);
    async fn remove(&self, id: SessionId);
    async fn count(&self) -> usize;
    async fn cleanup_expired(&self, now: DateTime<Utc>);
    async fn validate_access(
        &self,
        id: SessionId,
        version: u32,
        now: DateTime<Utc>,
    ) -> SessionResult<()>;
    async fn validate_and_bump_refresh(
        &self,
        id: SessionId,
        expected_version: u32,
        now: DateTime<Utc>,
    ) -> SessionResult<u32>;
    #[cfg(test)]
    async fn clear(&self);
}

pub struct InMemorySessionStore {
    inner: RwLock<HashMap<SessionId, SessionData>>,
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl InMemorySessionStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(clippy::significant_drop_tightening)]
#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn insert(&self, id: SessionId, data: SessionData) {
        self.inner.write().await.insert(id, data);
    }
    async fn remove(&self, id: SessionId) {
        self.inner.write().await.remove(&id);
    }
    async fn count(&self) -> usize {
        self.inner.read().await.len()
    }
    async fn cleanup_expired(&self, now: DateTime<Utc>) {
        self.inner.write().await.retain(|_, s| s.expires_at > now);
    }
    async fn validate_access(
        &self,
        id: SessionId,
        version: u32,
        now: DateTime<Utc>,
    ) -> SessionResult<()> {
        let mut map = self.inner.write().await;
        let sess = map.get_mut(&id).ok_or(AuthError::SessionNotFound)?;
        if sess.expires_at < now {
            return Err(AuthError::SessionExpired);
        }
        if version > sess.refresh_version {
            return Err(AuthError::SessionVersionMismatch);
        }
        sess.last_accessed = now;
        Ok(())
    }
    async fn validate_and_bump_refresh(
        &self,
        id: SessionId,
        expected_version: u32,
        now: DateTime<Utc>,
    ) -> SessionResult<u32> {
        let mut map = self.inner.write().await;
        let sess = map.get_mut(&id).ok_or(AuthError::SessionNotFound)?;
        if sess.expires_at < now {
            return Err(AuthError::SessionExpired);
        }
        if sess.refresh_version != expected_version {
            return Err(AuthError::SessionVersionMismatch);
        }
        sess.refresh_version += 1;
        sess.last_accessed = now;
        Ok(sess.refresh_version)
    }
    #[cfg(test)]
    async fn clear(&self) {
        self.inner.write().await.clear();
    }
}
