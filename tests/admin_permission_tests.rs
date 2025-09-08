use cms_backend::auth::{AuthContext, require_admin_permission};
use cms_backend::models::UserRole;
use uuid::Uuid;

#[test]
fn test_admin_permission_super_admin() {
    let auth_context = AuthContext {
        user_id: Uuid::new_v4(),
        username: "superadmin".to_string(),
        role: UserRole::SuperAdmin,
        session_id: "test_session".to_string(),
        permissions: vec!["admin".to_string(), "read".to_string(), "write".to_string(), "delete".to_string()],
    };

    let result = require_admin_permission(&auth_context);
    assert!(result.is_ok(), "SuperAdmin should have admin permission");
}

#[test]
fn test_admin_permission_admin() {
    let auth_context = AuthContext {
        user_id: Uuid::new_v4(),
        username: "admin".to_string(),
        role: UserRole::Admin,
        session_id: "test_session".to_string(),
        permissions: vec!["read".to_string(), "write".to_string(), "delete".to_string()],
    };

    let result = require_admin_permission(&auth_context);
    assert!(result.is_err(), "Regular Admin (without admin permission) should be rejected");
}

#[test]
fn test_admin_permission_admin_with_permission() {
    // This test shows that if we manually grant admin permission to an Admin role user, it works
    let auth_context = AuthContext {
        user_id: Uuid::new_v4(),
        username: "admin".to_string(),
        role: UserRole::Admin,
        session_id: "test_session".to_string(),
        permissions: vec!["admin".to_string(), "read".to_string(), "write".to_string(), "delete".to_string()],
    };

    let result = require_admin_permission(&auth_context);
    assert!(result.is_ok(), "Admin with admin permission should pass");
}

#[test]
fn test_admin_permission_editor() {
    let auth_context = AuthContext {
        user_id: Uuid::new_v4(),
        username: "editor".to_string(),
        role: UserRole::Editor,
        session_id: "test_session".to_string(),
        permissions: vec!["read".to_string(), "write".to_string()],
    };

    let result = require_admin_permission(&auth_context);
    assert!(result.is_err(), "Editor should not have admin permission");
}

#[test]
fn test_admin_permission_author() {
    let auth_context = AuthContext {
        user_id: Uuid::new_v4(),
        username: "author".to_string(),
        role: UserRole::Author,
        session_id: "test_session".to_string(),
        permissions: vec!["read".to_string(), "write_own".to_string()],
    };

    let result = require_admin_permission(&auth_context);
    assert!(result.is_err(), "Author should not have admin permission");
}