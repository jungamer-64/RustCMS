//! Category Application Layer - CQRS統合
//!
//! Commands + Queries + DTOs を単一ファイルに統合

use serde::{Deserialize, Serialize};

use domain::category::{
    Category, CategoryDescription, CategoryId, CategoryName, CategorySlug,
};

use crate::ports::repositories::{CategoryRepository, RepositoryError};

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCategoryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub post_count: i64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Category> for CategoryDto {
    fn from(category: Category) -> Self {
        Self {
            id: category.id().to_string(),
            name: category.name().to_string(),
            slug: category.slug().to_string(),
            description: category.description().to_string(),
            post_count: category.post_count(),
            is_active: category.is_active(),
            created_at: category.created_at().to_rfc3339(),
            updated_at: category.updated_at().to_rfc3339(),
        }
    }
}

// ============================================================================
// Commands
// ============================================================================

pub struct CreateCategory<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> CreateCategory<'a> {
    pub const fn new(repo: &'a dyn CategoryRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        request: CreateCategoryRequest,
    ) -> Result<CategoryDto, RepositoryError> {
        let name = CategoryName::new(request.name)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let slug = if let Some(slug_str) = request.slug {
            CategorySlug::new(slug_str)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?
        } else {
            CategorySlug::from_name(name.as_str())
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?
        };

        let description = CategoryDescription::new(request.description)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // スラッグ重複チェック
        if self.repo.find_by_slug(&slug).await?.is_some() {
            return Err(RepositoryError::Duplicate(format!(
                "Category slug '{}' already exists",
                slug
            )));
        }

        let category = Category::new(name, slug, description)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(category.clone()).await?;

        Ok(CategoryDto::from(category))
    }
}

pub struct UpdateCategory<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> UpdateCategory<'a> {
    pub const fn new(repo: &'a dyn CategoryRepository) -> Self {
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
            .ok_or_else(|| RepositoryError::NotFound(format!("Category {category_id}")))?;

        if let Some(new_name) = request.name {
            let name = CategoryName::new(new_name)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            category.update_name(name);
        }

        if let Some(new_description) = request.description {
            let desc = CategoryDescription::new(new_description)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            category.update_description(desc);
        }

        self.repo.save(category.clone()).await?;

        Ok(CategoryDto::from(category))
    }
}

pub struct DeactivateCategory<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> DeactivateCategory<'a> {
    pub const fn new(repo: &'a dyn CategoryRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, category_id: CategoryId) -> Result<CategoryDto, RepositoryError> {
        let mut category = self
            .repo
            .find_by_id(category_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Category {category_id}")))?;

        // deactivate is a void method per domain model
        category.deactivate();

        self.repo.save(category.clone()).await?;

        Ok(CategoryDto::from(category))
    }
}

// ============================================================================
// Queries
// ============================================================================

pub struct GetCategoryById<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> GetCategoryById<'a> {
    pub const fn new(repo: &'a dyn CategoryRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        category_id: CategoryId,
    ) -> Result<Option<CategoryDto>, RepositoryError> {
        Ok(self
            .repo
            .find_by_id(category_id)
            .await?
            .map(CategoryDto::from))
    }
}

pub struct ListCategories<'a> {
    repo: &'a dyn CategoryRepository,
}

impl<'a> ListCategories<'a> {
    pub const fn new(repo: &'a dyn CategoryRepository) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::repositories::MockCategoryRepository;

    #[tokio::test]
    async fn test_create_category_success() {
        let mut mock_repo = MockCategoryRepository::new();

        mock_repo.expect_find_by_slug().returning(|_| Ok(None));
        mock_repo.expect_save().returning(|_| Ok(()));

        let use_case = CreateCategory::new(&mock_repo);

        let request = CreateCategoryRequest {
            name: "Technology".into(),
            description: "Tech articles".into(),
            slug: Some("tech".into()),
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

        let existing = Category::new(
            CategoryName::new("Existing".into()).unwrap(),
            CategorySlug::new("tech".into()).unwrap(),
            CategoryDescription::new("Description".into()).unwrap(),
        )
        .unwrap();

        mock_repo
            .expect_find_by_slug()
            .returning(move |_| Ok(Some(existing.clone())));

        let use_case = CreateCategory::new(&mock_repo);

        let request = CreateCategoryRequest {
            name: "Technology".into(),
            description: "New description".into(),
            slug: Some("tech".into()),
        };

        let result = use_case.execute(request).await;
        assert!(matches!(result, Err(RepositoryError::Duplicate(_))));
    }
}
