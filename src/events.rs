//! Application-wide event system (DEPRECATED)
//!
//! このモジュールは `src/infrastructure/events/bus` に移行されました。
//! Backward compatibility のために re-export を維持しています。
//!
//! 新しいコードでは `crate::infrastructure::events::bus` を使用してください。

// Re-export from the new location
pub use crate::infrastructure::events::bus::*;

#[deprecated(since = "3.0.0", note = "Use crate::infrastructure::events::bus instead")]
pub use crate::infrastructure::events::bus::AppEvent;

#[deprecated(since = "3.0.0", note = "Use crate::infrastructure::events::bus instead")]
pub use crate::infrastructure::events::bus::EventBus;

#[deprecated(since = "3.0.0", note = "Use crate::infrastructure::events::bus instead")]
pub use crate::infrastructure::events::bus::create_event_bus;
