//! Shared Validation Utilities
//!
//! Input validation and sanitization functions.

// Note: validation.rs の内容を直接ここに含めるか、
// または validation.rs を別名にリネームする必要があります。
// 今回は validation.rs の内容を再エクスポートします。

// Module inception を避けるため、validation.rs を validators.rs にリネームする
#[path = "validation.rs"]
mod validators;

// Re-exports for convenience
pub use validators::*;
