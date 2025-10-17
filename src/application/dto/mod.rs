// src/application/dto/mod.rs
//! Data Transfer Objects (DTOs)
//!
//! HTTPリクエスト/レスポンスとドメインエンティティの変換を担います。
//!
//! ## 原則
//! - ドメインエンティティを直接公開しない
//! - シリアライズ/デシリアライズの責務を担う
//! - バリデーションはValue Objectsで実施済み

pub mod common;
pub mod user;
pub mod post;
pub mod comment;

pub use common::*;
pub use user::*;
pub use post::*;
pub use comment::*;
