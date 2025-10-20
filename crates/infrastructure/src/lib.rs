//! Infrastructure layer providing adapters for database, events, and shared application state.

pub mod app_state;
#[cfg(feature = "database")]
pub mod database;
pub mod events;
#[cfg(feature = "database")]
pub mod use_cases;

pub mod common;
pub mod config;
pub mod auth;

pub use app_state::AppState;
