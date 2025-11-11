//! Common Layer
//!
//! このモジュールはshared-coreクレートからre-exportし、後方互換性を保ちます。
//! 新しいコードはshared_coreを直接使用することを推奨します。

// Re-export everything from shared-core for backward compatibility
pub use shared_core::error as error_types;
pub use shared_core::helpers;
pub use shared_core::security;
pub use shared_core::types as type_utils;

// Validation module (not yet migrated to shared-core)
pub mod validation;

// Convenience re-exports (PRIMARY error types)
pub use shared_core::error::{
    AppError, ApplicationError, ApplicationResult, DomainError, DomainResult,
    InfrastructureError, InfrastructureResult, Result,
};

// types モジュール用のエイリアス (backward compatibility)
pub mod types {
    pub use shared_core::error::*;
}

// Prelude for ease of use
pub mod prelude {
    pub use shared_core::prelude::*;
}
