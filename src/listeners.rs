//! Event listeners for the application (DEPRECATED)
//!
//! このモジュールは `src/infrastructure/events/listeners` に移行されました。
//! Backward compatibility のために re-export を維持しています。
//!
//! 新しいコードでは `crate::infrastructure::events::listeners` を使用してください。

// Re-export from the new location
#[deprecated(
    since = "3.0.0",
    note = "Use crate::infrastructure::events::listeners instead"
)]
pub use crate::infrastructure::events::listeners::spawn_event_listeners;
