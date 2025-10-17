//! カテゴリドメインモデル (Category Domain Model)
//!
//! Entity + Value Objects 統合パターン（Phase 2）
//!
//! このファイルには以下が含まれます：
//! - Value Objects: CategoryId, CategoryName, CategorySlug, CategoryDescription
//! - Entity: Category
//! - ビジネスルール実装
//!
//! 設計原則：
//! - Value Objects は検証ロジックを内包
//! - Category Entity は不変条件を struct フィールドの private 化で保証
//! - カテゴリは投稿の分類に使用（タグより広い概念）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::common::types::DomainError;

// ============================================================================
// Value Objects
// ============================================================================

/// カテゴリID（NewType Pattern）
///
/// # 不変条件
/// - 内部のUUIDは常に有効
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CategoryId(Uuid);

impl CategoryId {
    /// 新しいカテゴリIDを生成
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 既存のUUIDからカテゴリIDを作成
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

impl Default for CategoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CategoryId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<CategoryId> for Uuid {
    fn from(id: CategoryId) -> Self {
        id.0
    }
}

/// カテゴリ名（検証済み）
///
/// # 不変条件
/// - 空でない
/// - 100文字以下
/// - ASCII英数字、ハイフン、スペース、アンダースコアのみ
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CategoryName(String);

impl CategoryName {
    /// カテゴリ名を検証して作成
    ///
    /// # エラー
    /// - 空文字列
    /// - 100文字超過
    /// - 許可されない文字を含む
    pub fn new(name: String) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::InvalidCategoryName(
                "Cannot be empty".to_string(),
            ));
        }

        if name.len() > 100 {
            return Err(DomainError::InvalidCategoryName(format!(
                "Exceeds 100 chars: {} chars",
                name.len()
            )));
        }

        // ASCII英数字、ハイフン、スペース、アンダースコアのみ許可
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ')
        {
            return Err(DomainError::InvalidCategoryName(
                "Contains invalid characters (only alphanumeric, dash, space, underscore allowed)"
                    .to_string(),
            ));
        }

        Ok(Self(name))
    }

    /// カテゴリ名の文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CategoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CategoryName> for String {
    fn from(name: CategoryName) -> Self {
        name.0
    }
}

/// カテゴリスラッグ（URL用識別子）
///
/// # 不変条件
/// - 空でない
/// - 50文字以下
/// - 小文字英数字とハイフンのみ
/// - ハイフンで開始/終了しない
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CategorySlug(String);

impl CategorySlug {
    /// カテゴリスラッグを検証して作成
    ///
    /// # エラー
    /// - 空文字列
    /// - 50文字超過
    /// - 大文字を含む
    /// - 許可されない文字を含む
    /// - ハイフンで開始/終了
    pub fn new(slug: String) -> Result<Self, DomainError> {
        if slug.is_empty() {
            return Err(DomainError::InvalidCategorySlug(
                "Cannot be empty".to_string(),
            ));
        }

        if slug.len() > 50 {
            return Err(DomainError::InvalidCategorySlug(format!(
                "Exceeds 50 chars: {} chars",
                slug.len()
            )));
        }

        // 小文字英数字とハイフンのみ
        if !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(DomainError::InvalidCategorySlug(
                "Contains invalid characters (only lowercase alphanumeric and dash allowed)"
                    .to_string(),
            ));
        }

        // ハイフンで開始/終了しない
        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(DomainError::InvalidCategorySlug(
                "Cannot start or end with dash".to_string(),
            ));
        }

        Ok(Self(slug))
    }

    /// スラッグの文字列参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CategorySlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CategorySlug> for String {
    fn from(slug: CategorySlug) -> Self {
        slug.0
    }
}

/// カテゴリ説明（検証済み）
///
/// # 不変条件
/// - 空でない
/// - 1000文字以下
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CategoryDescription(String);

impl CategoryDescription {
    /// カテゴリ説明を検証して作成
    ///
    /// # エラー
    /// - 空文字列
    /// - 1000文字超過
    pub fn new(description: String) -> Result<Self, DomainError> {
        if description.is_empty() {
            return Err(DomainError::InvalidCategoryDescription(
                "Cannot be empty".to_string(),
            ));
        }

        if description.len() > 1000 {
            return Err(DomainError::InvalidCategoryDescription(format!(
                "Exceeds 1,000 chars: {} chars",
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

impl fmt::Display for CategoryDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CategoryDescription> for String {
    fn from(desc: CategoryDescription) -> Self {
        desc.0
    }
}

// ============================================================================
// Entity
// ============================================================================

/// カテゴリエンティティ
///
/// カテゴリは投稿を大きく分類するための概念。
/// タグより階層的で、通常は少数である。
///
/// # 不変条件
/// - idは有効なUUID
/// - nameは100文字以下かつ有効な文字のみ
/// - slugは50文字以下、小文字英数字とハイフンのみ
/// - descriptionは1000文字以下
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    id: CategoryId,
    name: CategoryName,
    slug: CategorySlug,
    description: CategoryDescription,
    post_count: i64,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Category {
    /// 新しいカテゴリを作成
    ///
    /// # 引数
    /// - `name`: カテゴリ名（検証済み）
    /// - `slug`: URL用スラッグ（検証済み）
    /// - `description`: カテゴリ説明（検証済み）
    ///
    /// # 戻り値
    /// - 成功時：新しいCategoryエンティティ
    /// - 失敗時：DomainError
    ///
    /// # 例
    /// ```ignore
    /// let category = Category::new(
    ///     CategoryName::new("Technology".to_string())?,
    ///     CategorySlug::new("technology".to_string())?,
    ///     CategoryDescription::new("Articles about technology".to_string())?,
    /// )?;
    /// ```
    pub fn new(
        name: CategoryName,
        slug: CategorySlug,
        description: CategoryDescription,
    ) -> Result<Self, DomainError> {
        let now = Utc::now();
        Ok(Self {
            id: CategoryId::new(),
            name,
            slug,
            description,
            post_count: 0,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// データベースから Category を復元（ID保持）
    ///
    /// Repository での使用を想定。`new()` はビジネスロジック用に新しいIDを生成するが、
    /// `restore()` はデータベースから取得した既存のIDを保持する。
    ///
    /// # 引数
    /// - `id`: データベースから取得したカテゴリID
    /// - `name`: カテゴリ名（検証済み）
    /// - `slug`: URL用スラッグ（検証済み）
    /// - `description`: カテゴリ説明（検証済み）
    /// - `post_count`: 投稿数
    /// - `is_active`: アクティブ状態
    /// - `created_at`: 作成日時
    /// - `updated_at`: 更新日時
    ///
    /// # 戻り値
    /// 復元されたCategoryエンティティ
    #[must_use]
    pub fn restore(
        id: CategoryId,
        name: CategoryName,
        slug: CategorySlug,
        description: CategoryDescription,
        post_count: i64,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            slug,
            description,
            post_count,
            is_active,
            created_at,
            updated_at,
        }
    }

    /// カテゴリIDを取得
    #[must_use]
    pub fn id(&self) -> CategoryId {
        self.id
    }

    /// カテゴリ名を取得
    #[must_use]
    pub fn name(&self) -> &CategoryName {
        &self.name
    }

    /// スラッグを取得
    #[must_use]
    pub fn slug(&self) -> &CategorySlug {
        &self.slug
    }

    /// 説明を取得
    #[must_use]
    pub fn description(&self) -> &CategoryDescription {
        &self.description
    }

    /// 投稿数を取得
    #[must_use]
    pub fn post_count(&self) -> i64 {
        self.post_count
    }

    /// アクティブ状態を取得
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.is_active
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

    /// 投稿数をインクリメント
    pub fn increment_post_count(&mut self) {
        self.post_count += 1;
        self.updated_at = Utc::now();
    }

    /// 投稿数をデクリメント
    ///
    /// # エラー
    /// - 投稿数が既に0の場合
    pub fn decrement_post_count(&mut self) -> Result<(), DomainError> {
        if self.post_count == 0 {
            return Err(DomainError::InvalidCategoryStatus(
                "Cannot decrement post count below 0".to_string(),
            ));
        }
        self.post_count -= 1;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// カテゴリを有効化
    pub fn activate(&mut self) {
        if !self.is_active {
            self.is_active = true;
            self.updated_at = Utc::now();
        }
    }

    /// カテゴリを無効化
    pub fn deactivate(&mut self) {
        if self.is_active {
            self.is_active = false;
            self.updated_at = Utc::now();
        }
    }

    /// カテゴリ名を更新
    ///
    /// # 引数
    /// - `new_name`: 新しい名前（検証済み）
    pub fn update_name(&mut self, new_name: CategoryName) {
        self.name = new_name;
        self.updated_at = Utc::now();
    }

    /// スラッグを更新
    ///
    /// # 引数
    /// - `new_slug`: 新しいスラッグ（検証済み）
    pub fn update_slug(&mut self, new_slug: CategorySlug) {
        self.slug = new_slug;
        self.updated_at = Utc::now();
    }

    /// 説明を更新
    ///
    /// # 引数
    /// - `new_description`: 新しい説明（検証済み）
    pub fn update_description(&mut self, new_description: CategoryDescription) {
        self.description = new_description;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // CategoryId Tests
    // ========================================================================

    #[test]
    fn test_category_id_generation() {
        let id1 = CategoryId::new();
        let id2 = CategoryId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_category_id_display() {
        let id = CategoryId::new();
        let s = id.to_string();
        assert!(!s.is_empty());
        assert_eq!(s.len(), 36); // UUID format length
    }

    // ========================================================================
    // CategoryName Tests
    // ========================================================================

    #[test]
    fn test_category_name_valid() {
        let name = CategoryName::new("Technology and Science".to_string()).unwrap();
        assert_eq!(name.as_str(), "Technology and Science");
    }

    #[test]
    fn test_category_name_with_dash() {
        let name = CategoryName::new("Web-Development".to_string()).unwrap();
        assert_eq!(name.as_str(), "Web-Development");
    }

    #[test]
    fn test_category_name_empty() {
        let result = CategoryName::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCategoryName(_))));
    }

    #[test]
    fn test_category_name_too_long() {
        let long_name = "a".repeat(101);
        let result = CategoryName::new(long_name);
        assert!(matches!(result, Err(DomainError::InvalidCategoryName(_))));
    }

    #[test]
    fn test_category_name_boundary_100() {
        let exact_100 = "a".repeat(100);
        let name = CategoryName::new(exact_100).unwrap();
        assert_eq!(name.as_str().len(), 100);
    }

    #[test]
    fn test_category_name_invalid_chars() {
        let result = CategoryName::new("Tech@Science".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCategoryName(_))));
    }

    // ========================================================================
    // CategorySlug Tests
    // ========================================================================

    #[test]
    fn test_category_slug_valid() {
        let slug = CategorySlug::new("technology-science".to_string()).unwrap();
        assert_eq!(slug.as_str(), "technology-science");
    }

    #[test]
    fn test_category_slug_empty() {
        let result = CategorySlug::new("".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCategorySlug(_))));
    }

    #[test]
    fn test_category_slug_too_long() {
        let long_slug = "a".repeat(51);
        let result = CategorySlug::new(long_slug);
        assert!(matches!(result, Err(DomainError::InvalidCategorySlug(_))));
    }

    #[test]
    fn test_category_slug_boundary_50() {
        let exact_50 = "a".repeat(50);
        let slug = CategorySlug::new(exact_50).unwrap();
        assert_eq!(slug.as_str().len(), 50);
    }

    #[test]
    fn test_category_slug_uppercase_rejected() {
        let result = CategorySlug::new("TechScience".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCategorySlug(_))));
    }

    #[test]
    fn test_category_slug_starts_with_dash() {
        let result = CategorySlug::new("-technology".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCategorySlug(_))));
    }

    #[test]
    fn test_category_slug_ends_with_dash() {
        let result = CategorySlug::new("technology-".to_string());
        assert!(matches!(result, Err(DomainError::InvalidCategorySlug(_))));
    }

    #[test]
    fn test_category_slug_with_numbers() {
        let slug = CategorySlug::new("web2-tech".to_string()).unwrap();
        assert_eq!(slug.as_str(), "web2-tech");
    }

    // ========================================================================
    // CategoryDescription Tests
    // ========================================================================

    #[test]
    fn test_category_description_valid() {
        let desc = CategoryDescription::new("Technology and science articles".to_string()).unwrap();
        assert_eq!(desc.as_str(), "Technology and science articles");
    }

    #[test]
    fn test_category_description_empty() {
        let result = CategoryDescription::new("".to_string());
        assert!(matches!(
            result,
            Err(DomainError::InvalidCategoryDescription(_))
        ));
    }

    #[test]
    fn test_category_description_too_long() {
        let long_desc = "a".repeat(1001);
        let result = CategoryDescription::new(long_desc);
        assert!(matches!(
            result,
            Err(DomainError::InvalidCategoryDescription(_))
        ));
    }

    #[test]
    fn test_category_description_boundary_1000() {
        let exact_1000 = "a".repeat(1000);
        let desc = CategoryDescription::new(exact_1000).unwrap();
        assert_eq!(desc.as_str().len(), 1000);
    }

    // ========================================================================
    // Category Entity Tests
    // ========================================================================

    #[test]
    fn test_category_creation() {
        let name = CategoryName::new("Technology".to_string()).unwrap();
        let slug = CategorySlug::new("technology".to_string()).unwrap();
        let desc = CategoryDescription::new("Technology articles".to_string()).unwrap();
        let category = Category::new(name, slug, desc).unwrap();

        assert_eq!(category.name().as_str(), "Technology");
        assert_eq!(category.slug().as_str(), "technology");
        assert_eq!(category.post_count(), 0);
        assert!(category.is_active());
    }

    #[test]
    fn test_category_increment_post_count() {
        let name = CategoryName::new("Science".to_string()).unwrap();
        let slug = CategorySlug::new("science".to_string()).unwrap();
        let desc = CategoryDescription::new("Science articles".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        category.increment_post_count();
        assert_eq!(category.post_count(), 1);

        category.increment_post_count();
        assert_eq!(category.post_count(), 2);
    }

    #[test]
    fn test_category_decrement_post_count() {
        let name = CategoryName::new("Art".to_string()).unwrap();
        let slug = CategorySlug::new("art".to_string()).unwrap();
        let desc = CategoryDescription::new("Art articles".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        category.increment_post_count();
        category.increment_post_count();
        assert_eq!(category.post_count(), 2);

        category.decrement_post_count().unwrap();
        assert_eq!(category.post_count(), 1);
    }

    #[test]
    fn test_category_decrement_post_count_below_zero() {
        let name = CategoryName::new("Music".to_string()).unwrap();
        let slug = CategorySlug::new("music".to_string()).unwrap();
        let desc = CategoryDescription::new("Music articles".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        let result = category.decrement_post_count();
        assert!(matches!(result, Err(DomainError::InvalidCategoryStatus(_))));
        assert_eq!(category.post_count(), 0);
    }

    #[test]
    fn test_category_activate_deactivate() {
        let name = CategoryName::new("History".to_string()).unwrap();
        let slug = CategorySlug::new("history".to_string()).unwrap();
        let desc = CategoryDescription::new("History articles".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        assert!(category.is_active());

        category.deactivate();
        assert!(!category.is_active());

        category.activate();
        assert!(category.is_active());
    }

    #[test]
    fn test_category_update_name() {
        let name = CategoryName::new("Old Name".to_string()).unwrap();
        let slug = CategorySlug::new("old-name".to_string()).unwrap();
        let desc = CategoryDescription::new("Description".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        let old_updated = category.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let new_name = CategoryName::new("New Name".to_string()).unwrap();
        category.update_name(new_name);

        assert_eq!(category.name().as_str(), "New Name");
        assert!(category.updated_at() > old_updated);
    }

    #[test]
    fn test_category_update_slug() {
        let name = CategoryName::new("Category".to_string()).unwrap();
        let slug = CategorySlug::new("old-slug".to_string()).unwrap();
        let desc = CategoryDescription::new("Description".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        let old_updated = category.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let new_slug = CategorySlug::new("new-slug".to_string()).unwrap();
        category.update_slug(new_slug);

        assert_eq!(category.slug().as_str(), "new-slug");
        assert!(category.updated_at() > old_updated);
    }

    #[test]
    fn test_category_update_description() {
        let name = CategoryName::new("Test".to_string()).unwrap();
        let slug = CategorySlug::new("test".to_string()).unwrap();
        let desc = CategoryDescription::new("Old description".to_string()).unwrap();
        let mut category = Category::new(name, slug, desc).unwrap();

        let old_updated = category.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let new_desc = CategoryDescription::new("New description".to_string()).unwrap();
        category.update_description(new_desc);

        assert_eq!(category.description().as_str(), "New description");
        assert!(category.updated_at() > old_updated);
    }

    #[test]
    fn test_category_timestamps_initialized() {
        let name = CategoryName::new("Timestamp Test".to_string()).unwrap();
        let slug = CategorySlug::new("timestamp-test".to_string()).unwrap();
        let desc = CategoryDescription::new("Testing timestamps".to_string()).unwrap();
        let category = Category::new(name, slug, desc).unwrap();

        assert!(category.created_at() <= category.updated_at());
    }

    #[test]
    fn test_category_serialization() {
        let name = CategoryName::new("Serde Test".to_string()).unwrap();
        let slug = CategorySlug::new("serde-test".to_string()).unwrap();
        let desc = CategoryDescription::new("Serialization test".to_string()).unwrap();
        let category = Category::new(name, slug, desc).unwrap();

        let json = serde_json::to_string(&category).unwrap();
        let deserialized: Category = serde_json::from_str(&json).unwrap();

        assert_eq!(category.id(), deserialized.id());
        assert_eq!(category.name().as_str(), deserialized.name().as_str());
        assert_eq!(category.slug().as_str(), deserialized.slug().as_str());
    }

    #[test]
    fn test_category_equality() {
        let name = CategoryName::new("Equality".to_string()).unwrap();
        let slug = CategorySlug::new("equality".to_string()).unwrap();
        let desc = CategoryDescription::new("Equality test".to_string()).unwrap();

        let cat1 = Category::new(name.clone(), slug.clone(), desc.clone()).unwrap();
        let cat2 = Category::new(name, slug, desc).unwrap();

        // 異なるIDを持つので等しくない
        assert_ne!(cat1, cat2);
    }
}
