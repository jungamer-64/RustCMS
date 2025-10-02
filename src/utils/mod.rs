//! ユーティリティ集
//!
//! ハンドラやサービス層から再利用される共通ロジックをまとめたモジュールです。
//! 主な下位モジュール：
//! - `api_types`/`response_ext`: API レスポンスの型とヘルパ
//! - `cache_*`: キャッシュキー生成・TTL・キャッシュ抽象化
//! - `paginate`: ページング汎用処理（DB 取得関数と件数関数を受け取る）
//! - `auth_response`/`common_types`: 認証レスポンスの統一表現・共通 DTO
//! - `security_validation`: 入力バリデーション・サニタイズ
//! - `sort`: ソート指定の解析（許可カラムの安全な選択）
//! - `search_index`: 検索インデックス連携の薄いヘルパ
//! - `deprecation`: 互換 API の段階的廃止をユーザに通知
//!
//! これらはビジネスロジックを肥大化させないための補助であり、
//! できる限り副作用を限定しテストしやすい関数に分割されています。

pub mod api_types;
pub mod auth_response;
pub mod bin_utils;
pub mod cache_helpers;
pub mod cache_key;
pub mod cache_ttl;
pub mod common_types;
pub mod crud;
pub mod date;
pub mod deprecation;
pub mod dto;
pub mod dup;
pub mod error;
pub mod file;
pub mod hash;
pub mod init;
pub mod paginate;
pub mod password;
pub mod response_ext;
pub mod search_index;
pub mod security_validation;
pub mod sort;
pub mod text;
pub mod url_encoding;
pub mod validation;
pub mod vec_helpers; // Security validation utilities
