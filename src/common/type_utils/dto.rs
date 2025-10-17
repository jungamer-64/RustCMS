//! DTO conversion utilities
//!
//! Provides a small macro to eliminate repeated `impl From<&Model> for Dto` plus
//! the boilerplate owned `impl From<Model> for Dto` that simply delegates.
//!
//! # Macro
//!
//! ```ignore
//! dto_from_model!(PostDto, Post, |m| PostDto { id: m.id, ... });
//! ```
//! Expands to both reference and owned conversions.
//!
//! Keeps mapping logic in a single expression block and encourages consistent
//! construction patterns. The macro purposefully keeps a simple signature to
//! avoid over‑engineering while still removing duplication clusters.
//!
//! Added as part of the dedup initiative (see `deduplicated_logic_report.csv`).

#[macro_export]
macro_rules! dto_from_model {
    ($dto:ty, $model:ty, |$m:ident| $body:expr) => {
        impl From<&$model> for $dto {
            #[inline]
            fn from($m: &$model) -> Self {
                $body
            }
        }
        impl From<$model> for $dto {
            #[inline]
            fn from($m: $model) -> Self {
                Self::from(&$m)
            }
        }
    };
}

// Re-export macro under utils namespace when imported as module (not strictly
// necessary for #[macro_export] but convenient for discoverability).
pub use crate::dto_from_model; // re-export
