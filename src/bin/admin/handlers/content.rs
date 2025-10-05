// src/bin/admin/handlers/content.rs
use crate::cli::ContentAction;
use cms_backend::{AppState, Result};
use tracing::warn;

pub fn handle_content_action(action: ContentAction, _state: &AppState) -> Result<()> {
    warn!(
        "'Content' command invoked but not implemented: {:?}. Returning NotImplemented.",
        action
    );
    println!(
        "Content commands are not yet available in this CLI build. Refer to CLI.md for the roadmap."
    );
    Err(cms_backend::AppError::NotImplemented(
        "Content commands are not implemented in this build".into(),
    ))
}
