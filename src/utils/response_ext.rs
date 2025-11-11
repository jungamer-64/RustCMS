//! Response helpers (deprecated shim).
//!
//! Retains the historical `crate::utils::response_ext` API surface while
//! delegating to shared-core implementations.

use shared_core::types::api_types::{ok_message, ApiOk};

/// Convenience helper returning `ApiOk({"message": msg})`.
#[must_use]
pub fn ok_message_value(msg: &str) -> ApiOk<serde_json::Value> {
    ok_message(msg)
}

/// Generic helper for delete style endpoints that only need to run an async
/// operation and then return a standard message payload.
pub async fn delete_with<F>(op: F, message: &str) -> crate::Result<ApiOk<serde_json::Value>>
where
    F: std::future::Future<Output = crate::Result<()>>,
{
    op.await?;
    Ok(ok_message_value(message))
}
