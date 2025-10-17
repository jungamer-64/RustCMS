//! タグドメインモデル (Tag Domain Model)
//!
//! Entity + Value Objects 統合パターン（Phase 2）
//!
//! このファイルには以下が含まれます：
//! - Value Objects: TagId, TagName, TagDescription
//! - Entity: Tag
//! - ビジネスルール実装
//!
//! 設計原則：
//! - Value Objects は検証ロジックを内包
//! - Tag Entity は不変条件を struct フィールドの private 化で保証
//! - タグはメタデータとしてのみ機能（状態遷移なし）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::common::types::DomainError;

// ============================================================================
// Value Objects
// ============================================================================

/// タグID（NewType Pattern）
///
/// # 不変条件
/// - 内部のUUIDは常に有効
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagId(Uuid);

impl TagId {
    /// 新しいタグIDを生成
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 既存のUUIDからタグIDを作成
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// 内部のUUIDへの参照を取得
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// UUIDを消費して取得
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for TagId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TagId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<TagId> for Uuid {
    fn from(id: TagId) -> Self {
        id.0
    }
}

/// タグ名（検証済み）
///
/// # 不変条件
/// - 空でない
/// - 50文字以下
/// - ASCII英数字、ハイフン、アンダースコアのみ
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagName(String);

impl TagName {
    /// タグ名を検証して作成
    ///
    /// # エラー
    /// - 空文字列
    /// - 50文字超過
    /// - 許可されない文字を含む
    pub fn new(name: String) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::InvalidTagName("Cannot be empty".to_string()));
        }

        if name.len() > 50 {
            return Err(DomainError::InvalidTagName(format!(
                "Exceeds 50 chars: {} chars",
                name.len()
            )));
        }

        // ASCII英数字、ハイフン、アンダースコアのみ許可
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidTagName(
                "Contains invalid characters (only alphanumeric, dash, underscore allowed)"
                    .to_string(),
            ));
        }

        Ok(Self(name))
    }

    /// タグ名の文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TagName> for String {
    fn from(name: TagName) -> Self {
        name.0
    }
}

/// タグ説明（検証済み）
///
/// # 不変条件
/// - 空でない
/// - 500文字以下
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagDescription(String);

impl TagDescription {
    /// タグ説明を検証して作成
    ///
    /// # エラー
    /// - 空文字列
    /// - 500文字超過
    pub fn new(description: String) -> Result<Self, DomainError> {
        if description.is_empty() {
            return Err(DomainError::InvalidTagDescription(
                "Cannot be empty".to_string(),
            ));
        }

        if description.len() > 500 {
            return Err(DomainError::InvalidTagDescription(format!(
                "Exceeds 500 chars: {} chars",
                description.len()
            )));
        }

        Ok(Self(description))
    }

    /// 説明の文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TagDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TagDescription> for String {
    fn from(desc: TagDescription) -> Self {
        desc.0
    }
}

// ============================================================================
// Entity
// ============================================================================

/// タグエンティティ
///
/// タグは投稿やコンテンツを分類するためのメタデータ。
/// 状態遷移なしで、作成と削除のみサポート。
///
/// # 不変条件
/// - idは有効なUUID
/// - nameは50文字以下かつ有効な文字のみ
/// - descriptionは500文字以下
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    id: TagId,
    name: TagName,
    description: TagDescription,
    usage_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Tag {
    /// 新しいタグを作成
    ///
    /// # 引数
    /// - `name`: タグ名（検証済み）
    /// - `description`: タグ説明（検証済み）
    ///
    /// # 戻り値
    /// - 成功時：新しいTagエンティティ
    /// - 失敗時：DomainError
    ///
    /// # 例
    /// ```ignore
    /// let tag = Tag::new(
    ///     TagName::new("rust".to_string())?,
    ///     TagDescription::new("Rust programming language".to_string())?,
    /// )?;
    /// ```
    pub fn new(name: TagName, description: TagDescription) -> Result<Self, DomainError> {
        let now = Utc::now();
        Ok(Self {
            id: TagId::new(),
            name,
            description,
            usage_count: 0,
            created_at: now,
            updated_at: now,
        })
    }

    /// データベースから Tag を復元（ID保持）
    ///
    /// Repository での使用を想定。`new()` はビジネスロジック用に新しいIDを生成するが、
    /// `restore()` はデータベースから取得した既存のIDを保持する。
    ///
    /// # 引数
    /// - `id`: データベースから取得したTagId
    /// - `name`: タグ名（検証済み）
    /// - `description`: タグ説明（検証済み）
    /// - `usage_count`: 使用数
    /// - `created_at`: 作成日時
    /// - `updated_at`: 更新日時
    ///
    /// # 戻り値
    /// 既存のIDを持つTagエンティティ
    #[must_use]
    pub fn restore(
        id: TagId,
        name: TagName,
        description: TagDescription,
        usage_count: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            usage_count,
            created_at,
            updated_at,
        }
    }

    /// タグIDを取得
    #[must_use]
    pub fn id(&self) -> TagId {
        self.id
    }

    /// タグ名を取得
    #[must_use]
    pub fn name(&self) -> &TagName {
        &self.name
    }

    /// タグ説明を取得
    #[must_use]
    pub fn description(&self) -> &TagDescription {
        &self.description
    }

    /// 使用数を取得
    #[must_use]
    pub fn usage_count(&self) -> i64 {
        self.usage_count
    }

    /// 作成日時を取得
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// 更新日時を取得
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// 使用数をインクリメント
    ///
    /// 投稿がこのタグを使用した時に呼ぶ
    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
        self.updated_at = Utc::now();
    }

    /// 使用数をデクリメント
    ///
    /// 投稿がこのタグの使用を停止した時に呼ぶ
    ///
    /// # エラー
    /// - 使用数が既に0の場合
    pub fn decrement_usage(&mut self) -> Result<(), DomainError> {
        if self.usage_count == 0 {
            return Err(DomainError::InvalidTagStatus(
                "Cannot decrement usage count below 0".to_string(),
            ));
        }
        self.usage_count -= 1;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// タグが使用中か判定
    #[must_use]
    pub fn is_in_use(&self) -> bool {
        self.usage_count > 0
    }

    /// タグの説明を更新
    ///
    /// # 引数
    /// - `new_description`: 新しい説明（検証済み）
    pub fn update_description(&mut self, new_description: TagDescription) {
        self.description = new_description;
        self.updated_at = Utc::now();
    }

    /// タグ名を更新
    ///
    /// # 引数
    /// - `new_name`: 新しい名前（検証済み）
    pub fn update_name(&mut self, new_name: TagName) {
        self.name = new_name;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // TagId Tests
    // ========================================================================

    #[test]
    fn test_tag_id_generation() {
        let id1 = TagId::new();
        let id2 = TagId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_tag_id_display() {
        let id = TagId::new();
        let s = id.to_string();
        assert!(!s.is_empty());
        assert_eq!(s.len(), 36); // UUID format length
    }

    // ========================================================================
    // TagName Tests
    // ========================================================================

    #[test]
    fn test_tag_name_valid() {
        let name = TagName::new("rust-lang".to_string()).unwrap();
        assert_eq!(name.as_str(), "rust-lang");
    }

    #[test]
    fn test_tag_name_empty() {
        let result = TagName::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidTagName(_))));
    }

    #[test]
    fn test_tag_name_too_long() {
        let long_name = "a".repeat(51);
        let result = TagName::new(long_name);
        assert!(matches!(result, Err(DomainError::InvalidTagName(_))));
    }

    #[test]
    fn test_tag_name_boundary_50() {
        let exact_50 = "a".repeat(50);
        let name = TagName::new(exact_50).unwrap();
        assert_eq!(name.as_str().len(), 50);
    }

    #[test]
    fn test_tag_name_invalid_chars() {
        let result = TagName::new("rust@lang".to_string());
        assert!(matches!(result, Err(DomainError::InvalidTagName(_))));
    }

    #[test]
    fn test_tag_name_with_underscore() {
        let name = TagName::new("rust_lang".to_string()).unwrap();
        assert_eq!(name.as_str(), "rust_lang");
    }

    // ========================================================================
    // TagDescription Tests
    // ========================================================================

    #[test]
    fn test_tag_description_valid() {
        let desc = TagDescription::new("Rust programming language".to_string()).unwrap();
        assert_eq!(desc.as_str(), "Rust programming language");
    }

    #[test]
    fn test_tag_description_empty() {
        let result = TagDescription::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidTagDescription(_))));
    }

    #[test]
    fn test_tag_description_too_long() {
        let long_desc = "a".repeat(501);
        let result = TagDescription::new(long_desc);
        assert!(matches!(result, Err(DomainError::InvalidTagDescription(_))));
    }

    #[test]
    fn test_tag_description_boundary_500() {
        let exact_500 = "a".repeat(500);
        let desc = TagDescription::new(exact_500).unwrap();
        assert_eq!(desc.as_str().len(), 500);
    }

    // ========================================================================
    // Tag Entity Tests
    // ========================================================================

    #[test]
    fn test_tag_creation() {
        let name = TagName::new("rust".to_string()).unwrap();
        let desc = TagDescription::new("Rust language".to_string()).unwrap();
        let tag = Tag::new(name, desc).unwrap();

        assert_eq!(tag.name().as_str(), "rust");
        assert_eq!(tag.description().as_str(), "Rust language");
        assert_eq!(tag.usage_count(), 0);
        assert!(!tag.is_in_use());
    }

    #[test]
    fn test_tag_increment_usage() {
        let name = TagName::new("python".to_string()).unwrap();
        let desc = TagDescription::new("Python language".to_string()).unwrap();
        let mut tag = Tag::new(name, desc).unwrap();

        tag.increment_usage();
        assert_eq!(tag.usage_count(), 1);
        assert!(tag.is_in_use());

        tag.increment_usage();
        assert_eq!(tag.usage_count(), 2);
    }

    #[test]
    fn test_tag_decrement_usage() {
        let name = TagName::new("golang".to_string()).unwrap();
        let desc = TagDescription::new("Go language".to_string()).unwrap();
        let mut tag = Tag::new(name, desc).unwrap();

        tag.increment_usage();
        tag.increment_usage();
        assert_eq!(tag.usage_count(), 2);

        tag.decrement_usage().unwrap();
        assert_eq!(tag.usage_count(), 1);
    }

    #[test]
    fn test_tag_decrement_usage_below_zero() {
        let name = TagName::new("javascript".to_string()).unwrap();
        let desc = TagDescription::new("JavaScript".to_string()).unwrap();
        let mut tag = Tag::new(name, desc).unwrap();

        let result = tag.decrement_usage();
        assert!(matches!(result, Err(DomainError::InvalidTagStatus(_))));
        assert_eq!(tag.usage_count(), 0);
    }

    #[test]
    fn test_tag_update_description() {
        let name = TagName::new("webdev".to_string()).unwrap();
        let desc = TagDescription::new("Web development".to_string()).unwrap();
        let mut tag = Tag::new(name, desc).unwrap();

        let old_updated = tag.updated_at();

        // 意図的に時間を進める
        std::thread::sleep(std::time::Duration::from_millis(10));

        let new_desc = TagDescription::new("Web development updated".to_string()).unwrap();
        tag.update_description(new_desc);

        assert_eq!(tag.description().as_str(), "Web development updated");
        assert!(tag.updated_at() > old_updated);
    }

    #[test]
    fn test_tag_update_name() {
        let name = TagName::new("db".to_string()).unwrap();
        let desc = TagDescription::new("Database".to_string()).unwrap();
        let mut tag = Tag::new(name, desc).unwrap();

        let old_updated = tag.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let new_name = TagName::new("database".to_string()).unwrap();
        tag.update_name(new_name);

        assert_eq!(tag.name().as_str(), "database");
        assert!(tag.updated_at() > old_updated);
    }

    #[test]
    fn test_tag_usage_flow() {
        let name = TagName::new("test".to_string()).unwrap();
        let desc = TagDescription::new("Test tag".to_string()).unwrap();
        let mut tag = Tag::new(name, desc).unwrap();

        // 初期状態
        assert_eq!(tag.usage_count(), 0);
        assert!(!tag.is_in_use());

        // 使用開始
        tag.increment_usage();
        tag.increment_usage();
        tag.increment_usage();
        assert_eq!(tag.usage_count(), 3);
        assert!(tag.is_in_use());

        // 使用縮小
        tag.decrement_usage().unwrap();
        assert_eq!(tag.usage_count(), 2);
        assert!(tag.is_in_use());

        tag.decrement_usage().unwrap();
        tag.decrement_usage().unwrap();
        assert_eq!(tag.usage_count(), 0);
        assert!(!tag.is_in_use());
    }

    #[test]
    fn test_tag_timestamps_initialized() {
        let name = TagName::new("timestamp-test".to_string()).unwrap();
        let desc = TagDescription::new("Testing timestamps".to_string()).unwrap();
        let tag = Tag::new(name, desc).unwrap();

        assert!(tag.created_at() <= tag.updated_at());
    }

    #[test]
    fn test_tag_equality() {
        let name = TagName::new("eq-test".to_string()).unwrap();
        let desc = TagDescription::new("Equality test".to_string()).unwrap();

        let tag1 = Tag::new(name.clone(), desc.clone()).unwrap();
        let tag2 = Tag::new(name, desc).unwrap();

        // 異なるIDを持つので等しくない
        assert_ne!(tag1, tag2);
    }

    #[test]
    fn test_tag_serialization() {
        let name = TagName::new("serde-test".to_string()).unwrap();
        let desc = TagDescription::new("Serialization test".to_string()).unwrap();
        let tag = Tag::new(name, desc).unwrap();

        let json = serde_json::to_string(&tag).unwrap();
        let deserialized: Tag = serde_json::from_str(&json).unwrap();

        assert_eq!(tag.id(), deserialized.id());
        assert_eq!(tag.name().as_str(), deserialized.name().as_str());
    }
}
