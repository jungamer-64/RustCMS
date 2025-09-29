use cms_backend::{Result, models::User};
use ring::rand::{SecureRandom, SystemRandom};
use secrecy::SecretString;
use tokio::task;

use crate::backend::AdminBackend;

pub const PASSWORD_CHARSET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";

pub fn generate_random_password() -> Result<SecretString> {
    generate_random_password_with_len(16)
}

pub fn generate_random_password_with_len(len: usize) -> Result<SecretString> {
    let charset_len = PASSWORD_CHARSET.len() as u16;
    let threshold: u16 = 256u16 - (256u16 % charset_len);
    let rng = SystemRandom::new();

    let mut password = String::with_capacity(len);
    let mut byte = [0u8; 1];
    while password.len() < len {
        rng.fill(&mut byte).map_err(|_| {
            cms_backend::AppError::Internal(
                "Failed to read from system's entropy source".to_string(),
            )
        })?;
        let value = byte[0] as u16;
        if value < threshold {
            let idx = (value % charset_len) as usize;
            password.push(PASSWORD_CHARSET[idx] as char);
        }
    }
    Ok(SecretString::new(password.into_boxed_str()))
}

pub fn prompt_password(prompt: &str) -> Result<SecretString> {
    let password = rpassword::prompt_password(prompt)
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;

    if password.is_empty() {
        return Err(cms_backend::AppError::BadRequest(
            "Password cannot be empty".to_string(),
        ));
    }

    Ok(SecretString::new(password.into_boxed_str()))
}

pub async fn prompt_password_async(prompt: String) -> Result<SecretString> {
    task::spawn_blocking(move || prompt_password(&prompt))
        .await
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?
}

pub async fn find_user_by_id_or_username<B: AdminBackend + ?Sized>(
    backend: &B,
    identifier: &str,
) -> Result<User> {
    let result = if let Ok(id) = uuid::Uuid::parse_str(identifier) {
        backend.get_user_by_id(id).await
    } else {
        backend.get_user_by_username(identifier).await
    };

    match result {
        Ok(user) => Ok(user),
        Err(err) => {
            tracing::debug!(identifier = %identifier, backend_error = %format!("{err}"), "User lookup failed");
            match err {
                cms_backend::AppError::NotFound(_) => Err(cms_backend::AppError::NotFound(
                    format!("User '{}' not found", identifier),
                )),
                other => Err(other),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn generate_password_uses_charset() -> Result<()> {
        for _ in 0..8 {
            let password = generate_random_password()?;
            let secret = password.expose_secret();
            assert_eq!(secret.len(), 16);
            for ch in secret.chars() {
                let byte = ch as u8;
                assert!(
                    PASSWORD_CHARSET.contains(&byte),
                    "password contains invalid char: {}",
                    ch
                );
            }
        }
        Ok(())
    }
}
