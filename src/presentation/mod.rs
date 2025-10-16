// Presentation layer facade for HTTP handlers, CLI, and other adapters
// Re-export existing handlers and router for compatibility during migration
// When not using the new restructure_presentation feature, expose existing
// handlers/routes through the presentation::http facade to preserve imports.
#[cfg(not(feature = "restructure_presentation"))]
pub mod http {
    pub use crate::handlers::*;
    pub use crate::routes::*;
}

pub mod cli {
    // Placeholder for future CLI presentation adapters
}

// Keep file small: specific modules live in `src/handlers` and `src/routes`
// Presentation layer (facade) for HTTP handlers, CLI, and other adapters

#[cfg(feature = "restructure_presentation")]
pub mod http;

/// Presentation Layer prelude: 共通型のインポート
pub mod prelude {
    // Phase 4 で実装予定
}
