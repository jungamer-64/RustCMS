//! 実用的なCMSエラー処理
//! 本番環境でも安全なエラーレスポンス

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,

/// ページング付き成功レスポンス
// Compatibility shim: forward to canonical `utils::error` module
pub use crate::utils::error::*;
