//! HTTP ミドルウェア層（Phase 4新構造統合）
//!
//! # モジュール構成
//!
//! ## Phase 4新構造（推奨）
//! - `core`: 統合ミドルウェア（require_auth, rate_limit, request_logging）
//!
//! ## レガシー（段階廃止予定）
//! - `api_key`: 実験的APIキー認証
//! - `auth`: 既存認証ミドルウェア
//! - `rate_limiting`: 既存レート制限
//! - その他: 共通機能

// Phase 4新構造: 統合ミドルウェア（最優先）
pub mod core;
pub use core::{rate_limit, request_logging, require_auth};

// レガシーモジュール（Phase 5で段階廃止）
// TODO: これらのモジュールは新AppStateに対応していないため一時的に無効化
// Phase 5+ でサービス統合後に再実装予定
#[cfg(feature = "legacy_middlewares")]
pub mod api_key;
#[cfg(feature = "legacy_middlewares")]
pub mod auth;
pub mod common;
pub mod compression;
#[cfg(feature = "legacy_middlewares")]
pub mod csrf;
pub mod deprecation;
pub mod logging;
pub mod permission;
pub mod rate_limit_backend;
#[cfg(feature = "legacy_middlewares")]
pub mod rate_limiting;
pub mod security;
