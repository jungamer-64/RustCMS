// src/application/dto/mod.rs
//! Data Transfer Objects (DTOs)
//!
//! HTTPリクエスト/レスポンスとドメインエンティティの変換を担います。
//!
//! ## 原則
//! - ドメインエンティティを直接公開しない
//! - シリアライズ/デシリアライズの責務を担う
//! - バリデーションはValue Objectsで実施済み

pub mod comment;
pub mod common;
pub mod post;
pub mod user;

pub use comment::*;
pub use common::*;
pub use post::*;
pub use user::*;
