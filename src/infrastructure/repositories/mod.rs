//! Repository implementations (DEPRECATED)
//!
//! Phase 9: All legacy implementations moved to infrastructure/database/repositories.rs
//! This module is kept for backward compatibility but will be removed in Phase 10.

// ============================================================================
// Phase 9 Migration Notice
// ============================================================================
//
// Legacy repository implementations (2,373 lines) have been replaced with
// new implementations following the audited structure pattern:
//
// Old location: src/infrastructure/repositories/*.rs
// New location: src/infrastructure/database/repositories.rs (single file)
//
// Benefits of new structure:
// - Single file consolidation (< 1000 lines total)
// - Consistent with RESTRUCTURE_EXAMPLES.md audited pattern
// - Direct Pool usage (no Database wrapper dependency)
// - New trait interface (find_by_id, save, delete)
// - Async wrapping with tokio::task::spawn_blocking
//
// ============================================================================

#[cfg(feature = "restructure_domain")]
pub mod error_helpers;

// All other legacy modules disabled

