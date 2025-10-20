//! 統合認証コンテキスト (Phase 5.3)
//!
//! JWT (認証) と Biscuit (認可) を統合した認証コンテキストを提供します。
//!
//! # 設計思想
//! - **JWT**: "このユーザーは誰か?" (Who are you?)
//!   - ユーザーID、ユーザー名、ロールなどのアイデンティティ情報
//!   - セッション管理
//!   - 短命のアクセストークン
//!
//! - **Biscuit**: "このユーザーは何ができるか?" (What can you do?)
//!   - リソースベースの権限チェック
//!   - 細粒度のアクセス制御
//!   - 委譲可能な権限
//!   - ポリシー評価
//!
//! # 使用フロー
//! 1. ログイン時: JWT トークンペアを生成
//! 2. リクエスト時: JWT でユーザーを認証
//! 3. 権限チェック時: Biscuit でリソースアクセスを認可
//! 4. リフレッシュ時: JWT リフレッシュトークンで新しいアクセストークンを取得

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "restructure_domain")]
use crate::domain::user::UserRole;

#[cfg(not(feature = "restructure_domain"))]
use crate::models::UserRole;

use crate::common::type_utils::common_types::SessionId;

/// 統合認証コンテキスト (JWT + Biscuit)
///
/// このコンテキストはリクエストごとに生成され、
/// ミドルウェアによって request extensions に格納されます。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAuthContext {
    // ===== JWT 認証情報 =====
    /// ユーザーID (JWT sub クレーム)
    pub user_id: Uuid,

    /// ユーザー名 (JWT カスタムクレーム)
    pub username: String,

    /// ユーザーロール (JWT カスタムクレーム)
    pub role: UserRole,

    /// セッションID (JWT カスタムクレーム)
    pub session_id: SessionId,

    /// JWT トークンの有効期限 (Unix timestamp)
    pub jwt_exp: i64,

    // ===== Biscuit 認可情報 =====
    /// Biscuit トークンから抽出された権限リスト
    pub permissions: Vec<String>,

    /// Biscuit トークン (オプション - 追加の権限チェック用)
    pub biscuit_token: Option<String>,
}

impl UnifiedAuthContext {
    /// JWT クレームから認証コンテキストを作成
    ///
    /// 初期状態では Biscuit 権限は空です。
    /// 必要に応じて `with_biscuit_permissions` で権限を追加します。
    pub fn from_jwt(claims: &crate::auth::jwt::JwtClaims) -> Result<Self, crate::auth::AuthError> {
        let user_id =
            Uuid::parse_str(&claims.sub).map_err(|_| crate::auth::AuthError::InvalidTokenFormat)?;

        let session_id = SessionId::from(claims.session_id.clone());

        Ok(Self {
            user_id,
            username: claims.username.clone(),
            role: claims.role.clone(),
            session_id,
            jwt_exp: claims.exp,
            permissions: Vec::new(),
            biscuit_token: None,
        })
    }

    /// Biscuit 権限を追加
    ///
    /// Biscuit トークンから抽出した権限をコンテキストに追加します。
    pub fn with_biscuit_permissions(
        mut self,
        permissions: Vec<String>,
        token: Option<String>,
    ) -> Self {
        self.permissions = permissions;
        self.biscuit_token = token;
        self
    }

    /// 特定の権限を持っているかチェック
    ///
    /// # Arguments
    /// * `permission` - チェックする権限文字列 (例: "posts:write", "users:read")
    ///
    /// # Returns
    /// 権限を持っている場合は `true`
    pub fn has_permission(&self, permission: &str) -> bool {
        // Admin ロールはすべての権限を持つ
        if matches!(self.role, UserRole::Admin) {
            return true;
        }

        self.permissions.iter().any(|p| p == permission)
    }

    /// 複数の権限のいずれかを持っているかチェック
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        if matches!(self.role, UserRole::Admin) {
            return true;
        }

        permissions.iter().any(|&p| self.has_permission(p))
    }

    /// すべての権限を持っているかチェック
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        if matches!(self.role, UserRole::Admin) {
            return true;
        }

        permissions.iter().all(|&p| self.has_permission(p))
    }

    /// 管理者かどうかチェック
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }

    /// JWT トークンの有効期限が切れているかチェック
    pub fn is_jwt_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.jwt_exp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::{JwtClaims, TokenType};

    fn create_test_claims() -> JwtClaims {
        JwtClaims {
            sub: Uuid::new_v4().to_string(),
            username: "testuser".to_string(),
            role: UserRole::Editor,
            session_id: SessionId::new().as_ref().to_string(), // .to_string() を .as_ref().to_string() に修正
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            iat: chrono::Utc::now().timestamp(),
            token_type: TokenType::Access,
        }
    }

    #[test]
    fn test_from_jwt() {
        let claims = create_test_claims();
        let context = UnifiedAuthContext::from_jwt(&claims).expect("Failed to create context");

        assert_eq!(context.username, "testuser");
        assert_eq!(context.role, UserRole::Editor);
        assert!(context.permissions.is_empty());
    }

    #[test]
    fn test_with_biscuit_permissions() {
        let claims = create_test_claims();
        let context = UnifiedAuthContext::from_jwt(&claims)
            .expect("Failed to create context")
            .with_biscuit_permissions(
                vec!["posts:read".to_string(), "posts:write".to_string()],
                Some("biscuit_token".to_string()),
            );

        assert_eq!(context.permissions.len(), 2);
        assert!(context.biscuit_token.is_some());
    }

    #[test]
    fn test_has_permission() {
        let claims = create_test_claims();
        let context = UnifiedAuthContext::from_jwt(&claims)
            .expect("Failed to create context")
            .with_biscuit_permissions(vec!["posts:read".to_string()], None);

        assert!(context.has_permission("posts:read"));
        assert!(!context.has_permission("posts:write"));
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let mut claims = create_test_claims();
        claims.role = UserRole::Admin;

        let context = UnifiedAuthContext::from_jwt(&claims).expect("Failed to create context");

        // Admin はどんな権限もチェックでパスする
        assert!(context.has_permission("any:permission"));
        assert!(context.has_any_permission(&["posts:write", "users:delete"]));
        assert!(context.has_all_permissions(&["posts:write", "users:delete"]));
        assert!(context.is_admin());
    }

    #[test]
    fn test_has_any_permission() {
        let claims = create_test_claims();
        let context = UnifiedAuthContext::from_jwt(&claims)
            .expect("Failed to create context")
            .with_biscuit_permissions(vec!["posts:read".to_string()], None);

        assert!(context.has_any_permission(&["posts:read", "posts:write"]));
        assert!(!context.has_any_permission(&["posts:write", "users:delete"]));
    }

    #[test]
    fn test_has_all_permissions() {
        let claims = create_test_claims();
        let context = UnifiedAuthContext::from_jwt(&claims)
            .expect("Failed to create context")
            .with_biscuit_permissions(
                vec!["posts:read".to_string(), "posts:write".to_string()],
                None,
            );

        assert!(context.has_all_permissions(&["posts:read", "posts:write"]));
        assert!(!context.has_all_permissions(&["posts:read", "users:delete"]));
    }
}
