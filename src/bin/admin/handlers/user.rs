// src/bin/admin/handlers/user.rs
use cms_backend::{
    AppState, Result,
    models::{CreateUserRequest, UpdateUserRequest},
};
use comfy_table::{Cell, Table};
use secrecy::{ExposeSecret, SecretString};
use tokio::task;
use tracing::{info, warn};

use crate::backend::AdminBackend;
use crate::cli::{UserAction, UserRoleArg};
use crate::util::{
    find_user_by_id_or_username, generate_random_password_with_len, prompt_password_async,
};

pub async fn handle_user_action<B: AdminBackend + ?Sized>(
    action: UserAction,
    backend: &B,
) -> Result<()> {
    match action {
        UserAction::List { role, active_only } => list(backend, &role, active_only).await?,
        UserAction::Create {
            username,
            email,
            role,
            generate_password,
        } => create(backend, username, email, role, generate_password).await?,
        UserAction::Update {
            user,
            email,
            role,
            active,
        } => update(backend, user, email, role, active).await?,
        UserAction::Delete { user, force } => delete(backend, user, force).await?,
        UserAction::ResetPassword {
            user,
            password,
            generate_password,
        } => reset_password(backend, user, password, generate_password).await?,
    }
    Ok(())
}

async fn list<B: AdminBackend + ?Sized>(
    backend: &B,
    role: &Option<UserRoleArg>,
    active_only: bool,
) -> Result<()> {
    info!("📊 Listing users...");
    let role_filter: Option<&str> = role.as_ref().map(|r| match r {
        UserRoleArg::Admin => "admin",
        UserRoleArg::Editor => "editor",
        UserRoleArg::Subscriber => "subscriber",
    });
    let active_filter = if active_only { Some(true) } else { None };
    let users = backend.list_users(role_filter, active_filter).await?;

    if users.is_empty() {
        println!("No users found matching the criteria.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["ID", "Username", "Email", "Role", "Active"]);

    // Phase 2: User fields are private, use getters
    for user in users {
        table.add_row(vec![
            Cell::new(user.id().to_string()),
            Cell::new(user.username().to_string()),
            Cell::new(user.email().to_string()),
            Cell::new(format!("{:?}", user.role())),
            Cell::new(if user.is_active() { "Yes" } else { "No" }),
        ]);
    }

    println!("{table}");

    Ok(())
}

async fn create<B: AdminBackend + ?Sized>(
    backend: &B,
    username: String,
    email: String,
    _role: UserRoleArg, // Phase 2: CreateUserRequest doesn't accept role
    generate_password: bool,
) -> Result<()> {
    let password = if generate_password {
        let len = std::env::var("ADMIN_PW_LENGTH")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .map(|v| v.clamp(8, 128))
            .unwrap_or(16);
        generate_random_password_with_len(len)?
    } else {
        prompt_password_async("Enter password for new user: ".to_string()).await?
    };

    let password_for_request = password.expose_secret().to_owned();

    // Phase 2: CreateUserRequest simplified (no first_name, last_name, role)
    let user = CreateUserRequest {
        username: username.clone(),
        email,
        password: password_for_request,
    };

    let created_user = backend.create_user(user).await?;

    // Phase 2: User fields are private, use getters
    info!("✅ User created successfully:");
    println!("  ID: {}", created_user.id());
    println!("  Username: {}", created_user.username());
    println!("  Email: {}", created_user.email());
    println!("  Role: {:?}", created_user.role());

    if generate_password {
        warn!("🔑 A new random password has been generated.");
        warn!("⚠️  Please save this password securely - it will not be shown again!");
        println!("Generated password: {}", password.expose_secret());
    }

    Ok(())
}

async fn update<B: AdminBackend + ?Sized>(
    backend: &B,
    user: String,
    email: Option<String>,
    _role: Option<UserRoleArg>, // Phase 2: UpdateUserRequest API changed
    _active: Option<bool>,      // Phase 2: UpdateUserRequest API changed
) -> Result<()> {
    let existing_user = find_user_by_id_or_username(backend, &user).await?;

    // Phase 2: UpdateUserRequest simplified
    let update = UpdateUserRequest {
        username: None,
        email: email.clone(),
        password: None,
    };

    // Phase 2: User ID is UserId type, use getter
    let updated_user = backend.update_user(existing_user.id().into(), update).await?;

    // Phase 2: User fields are private, use getters
    info!("✅ User updated successfully:");
    println!("  ID: {}", updated_user.id());
    println!("  Username: {}", updated_user.username());
    println!("  Email: {}", updated_user.email());
    println!("  Role: {:?}", updated_user.role());
    println!("  Active: {}", updated_user.is_active());

    Ok(())
}

async fn delete<B: AdminBackend + ?Sized>(backend: &B, user: String, force: bool) -> Result<()> {
    let existing_user = find_user_by_id_or_username(backend, &user).await?;

    // Phase 2: User fields are private, use getters
    if !force {
        warn!(
            "⚠️  You are about to delete user: {} ({})",
            existing_user.username(), existing_user.email()
        );
        warn!("⚠️  This action cannot be undone!");

        let confirmed = task::spawn_blocking(move || -> Result<bool> {
            use std::io::{self, Write};
            print!("Type 'DELETE' to confirm: ");
            io::stdout()
                .flush()
                .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;
            Ok(input.trim().eq_ignore_ascii_case("DELETE"))
        })
        .await
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))??;

        if !confirmed {
            info!("❌ User deletion cancelled");
            return Ok(());
        }
    }

    // Phase 2: User ID is UserId type, use getter and convert
    backend.delete_user(existing_user.id().into()).await?;
    info!("✅ User deleted successfully");

    Ok(())
}

async fn reset_password<B: AdminBackend + ?Sized>(
    backend: &B,
    user: String,
    password: Option<String>,
    generate_password: bool,
) -> Result<()> {
    let existing_user = find_user_by_id_or_username(backend, &user).await?;

    let new_password = match (password, generate_password) {
        (Some(p), false) => Ok(SecretString::new(p.into_boxed_str())),
        (None, true) => {
            let len = std::env::var("ADMIN_PW_LENGTH")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .map(|v| v.clamp(8, 128))
                .unwrap_or(16);
            generate_random_password_with_len(len)
        }
        (None, false) => prompt_password_async("Enter new password: ".to_string()).await,
        (Some(_), true) => unreachable!(),
    }?;

    // Phase 2: User ID is UserId type, use getter and convert
    backend
        .reset_user_password(existing_user.id().into(), new_password.expose_secret())
        .await?;

    // Phase 2: User fields are private, use getters
    info!(
        "✅ Password reset successfully for user: {}",
        existing_user.username()
    );
    if generate_password {
        warn!(
            "🔑 A new random password has been generated for user: {}",
            existing_user.username()
        );
        println!("Generated password: {}", new_password.expose_secret());
    }

    Ok(())
}

pub async fn handle_user_action_state(action: UserAction, state: &AppState) -> Result<()> {
    handle_user_action(action, state).await
}
