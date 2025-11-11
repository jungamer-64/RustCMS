//! Utility modules (Phase 7 minimal set - legacy removed)

pub mod api_types;
pub mod cache_key;
pub mod cache_ttl;
pub mod deprecation;
// Phase 9: dto module removed (legacy code deleted in Phase 7)
// pub mod dto;
// pub mod error; // Removed in Phase 7
// Phase 9: search_index module removed (legacy code deleted in Phase 7)
// pub mod search_index;
pub mod security_validation;
pub mod url_encoding;
pub mod validation;

pub use shared_core::helpers::cache_helpers as cache_helpers;
pub use shared_core::helpers::date as date;
pub use shared_core::helpers::hash as hash;
pub use shared_core::helpers::text as text;
pub use shared_core::helpers::vec_helpers as vec_helpers;
pub use shared_core::security::password as password;
pub use shared_core::types::sort as sort;
