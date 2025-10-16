// Shared utilities and types used across layers
// This facade allows incremental migration of common utilities into `src/shared`.

pub mod time {
    pub use chrono::*;
}

pub mod prelude {
    // Re-export commonly used items to ease migration
    pub use crate::common::*;
}
