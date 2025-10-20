// src/application/category.rs
//! Category Application Layer - CQRS統合
//!
//! Phase 6-D: Legacy application layer (disabled with restructure_domain)
//! Commands + Queries + DTOs を単一ファイルに統合（監査推奨パターン）
#![cfg(not(feature = "restructure_domain"))]

use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::category::{Category, CategoryDescription, CategoryId, CategoryName, CategorySlug};

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{CategoryRepository, RepositoryError};

// ============================================================================
// DTOs
// ============================================================================

/// カテゴリ作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: String, // Made mandatory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

/// カテゴリ更新リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCategoryRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// カテゴリレスポンス
#[derive(Debug, Clone, Serialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub post_count: i64, // Changed from i32 to i64
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "restructure_domain")]
impl From<Category> for CategoryDto {
    fn from(category: Category) -> Self {
        Self {
            id: category.id().to_string(),
            name: category.name().to_string(),
            slug: category.slug().to_string(),
            description: category.description().to_string(), // to_string() instead of map()
            post_count: category.post_count(), // Already i64
            is_active: category.is_active(),
            created_at: category.created_at().to_rfc3339(),
            updated_at: category.updated_at().to_rfc3339(),
        }
    }
}

// ============================================================================
// Commands
// ============================================================================

/// カテゴリ作成コマンド
#[cfg(feature = "restructure_domain")]
pub struct CreateCategory {
    repo: Arc<dyn CategoryRepository>,
}

#[cfg(feature = "restructure_domain")]
impl CreateCategory {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        request: CreateCategoryRequest,
    ) -> Result<CategoryDto, RepositoryError> {
        // 1. Value Objects 作成
        let name = CategoryName::new(request.name)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let slug = if let Some(slug_str) = request.slug {
            CategorySlug::new(slug_str)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?
        } else {
            CategorySlug::from_name(name.as_str())
        };

        let description = CategoryDescription::new(request.description)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 2. スラッグ重複チェック
        if self.repo.find_by_slug(&slug).await?.is_some() {
            return Err(RepositoryError::Duplicate(format!(
                "Category slug '{}' already exists",
                slug
            )));
        }

        // 3. ドメインエンティティ作成
        let category = Category::new(name, slug, description)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 4. 永続化
        self.repo.save(category.clone()).await?;

        Ok(CategoryDto::from(category))
    }
}

/// カテゴリ更新コマンド
#[cfg(feature = "restructure_domain")]
pub struct UpdateCategory {
    repo: Arc<dyn CategoryRepository>,
}

#[cfg(feature = "restructure_domain")]
impl UpdateCategory {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        category_id: CategoryId,
        request: UpdateCategoryRequest,
    ) -> Result<CategoryDto, RepositoryError> {
        let mut category = self
            .repo
            .find_by_id(category_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Category {}", category_id)))?;

        if let Some(new_name) = request.name {
            let name = CategoryName::new(new_name)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            category
                .update_name(name)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
        }

        if let Some(new_description) = request.description {
            category.update_description(Some(new_description));
        }

        self.repo.save(category.clone()).await?;

        Ok(CategoryDto::from(category))
    }
}

/// カテゴリ非アクティブ化コマンド
#[cfg(feature = "restructure_domain")]
pub struct DeactivateCategory {
    repo: Arc<dyn CategoryRepository>,
}

#[cfg(feature = "restructure_domain")]
impl DeactivateCategory {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, category_id: CategoryId) -> Result<CategoryDto, RepositoryError> {
        let mut category = self
            .repo
            .find_by_id(category_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Category {}", category_id)))?;

        category
            .deactivate()
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(category.clone()).await?;

        Ok(CategoryDto::from(category))
    }
}

// ============================================================================
// Queries
// ============================================================================

/// カテゴリ取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct GetCategoryById {
    repo: Arc<dyn CategoryRepository>,
}

#[cfg(feature = "restructure_domain")]
impl GetCategoryById {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        category_id: CategoryId,
    ) -> Result<Option<CategoryDto>, RepositoryError> {
        let category = self.repo.find_by_id(category_id).await?;
        Ok(category.map(CategoryDto::from))
    }
}

/// カテゴリ一覧取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct ListCategories {
    repo: Arc<dyn CategoryRepository>,
}

#[cfg(feature = "restructure_domain")]
impl ListCategories {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CategoryDto>, RepositoryError> {
        let categories = self.repo.list_all(limit, offset).await?;
        Ok(categories.into_iter().map(CategoryDto::from).collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "restructure_domain"))]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::application::ports::repositories::{MockCategoryRepository, RepositoryError};

    #[tokio::test]
    async fn test_create_category_success() {
        let mut mock_repo = MockCategoryRepository::new();

        // スラッグ重複チェック: None
        mock_repo
            .expect_find_by_slug()
            .returning(|_| Box::pin(async { Ok(None) }));

        mock_repo
            .expect_save()
            .returning(|_| Box::pin(async { Ok(()) }));

        let use_case = CreateCategory::new(Arc::new(mock_repo));

        let request = CreateCategoryRequest {
            name: "Technology".to_string(),
            description: "Tech articles".to_string(),
            slug: Some("tech".to_string()),
        };

        let result = use_case.execute(request).await;
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert_eq!(dto.name, "Technology");
        assert_eq!(dto.slug, "tech");
    }

    #[tokio::test]
    async fn test_create_category_duplicate_slug() {
        let mut mock_repo = MockCategoryRepository::new();

        let existing_cat = Category::new(
            CategoryName::new("Existing".to_string()).unwrap(),
            CategorySlug::new("tech".to_string()).unwrap(),
            CategoryDescription::new("Description".to_string()).unwrap(),
        )
        .unwrap();

        mock_repo
            .expect_find_by_slug()
            .returning(move |_| Box::pin(async move { Ok(Some(existing_cat.clone())) }));

        let use_case = CreateCategory::new(Arc::new(mock_repo));

        let request = CreateCategoryRequest {
            name: "Technology".to_string(),
            description: "New description".to_string(),
            slug: Some("tech".to_string()),
        };

        let result = use_case.execute(request).await;
        assert!(matches!(result, Err(RepositoryError::Duplicate(_))));
    }
}
