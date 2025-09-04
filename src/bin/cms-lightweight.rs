//! Lightweight CMS server binary (recreated)
//! Provides a minimal startup harness mainly for benchmark / smoke tests.

use cms_backend::{utils::init, AppState};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let config = init::init_logging_and_config().await?;
	let _state = AppState::from_config(config).await?;
	info!("lightweight server initialized; no HTTP listener started");
	// Prevent immediate exit in real usage; here we just return Ok for CI/lint.
	Ok(())
}
