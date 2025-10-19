//! HTTP ミドルウェア層 (Phase 5.2 - 新AppState対応完了)
//!
//! # モジュール構成
//!
//! ## Phase 4新構造(推奨)
//! - `core`: 統合ミドルウェア(require_auth, rate_limit, request_logging)
//!
//! ## Phase 5.2 更新済みモジュール
//! - `api_key`: APIキー認証 (Arc<AppState>対応、Phase 5.3で完全実装予定)
//! - `auth`: JWT/Biscuit認証 (Arc<AppState>対応、Phase 5.3で完全実装予定)
//! - `csrf`: CSRF保護 (Arc<AppState>対応、Phase 5.3でキャッシュ統合予定)
//! - `rate_limiting`: レート制限 (Arc<AppState>対応、Phase 5.3でキャッシュ統合予定)
//!
//! ## その他
//! - `common`: 共通ユーティリティ
//! - `compression`: 圧縮ミドルウェア
//! - `deprecation`: 非推奨機能の警告
//! - `logging`: ロギング
//! - `permission`: 権限チェック
//! - `security`: セキュリティヘルパー

// Phase 4新構造: 統合ミドルウェア(最優先)
pub mod core;
pub use core::{rate_limit, request_logging, require_auth};

// Phase 5.2: 新AppState対応済みモジュール
pub mod api_key;
pub mod auth;
pub mod common;
pub mod compression;
pub mod csrf;
pub mod deprecation;
pub mod logging;
pub mod permission;
pub mod rate_limit_backend;
pub mod rate_limiting;
pub mod security;
