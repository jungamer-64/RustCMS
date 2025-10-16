// src/infrastructure/database/repositories/diesel_comment_repository.rs
//! Diesel ベースの Comment Repository 実装（Phase 3 Step 4）

use crate::common::types::ApplicationError;

/// Diesel ベースの Comment Repository 実装（Phase 3 Step 4）
#[derive(Clone)]
pub struct DieselCommentRepository {
    // TODO: Phase 3.3 - DB コネクションプール注入
}

impl DieselCommentRepository {
    pub fn new() -> Self {
        Self {}
    }

    pub fn find_by_id(&self, _id_placeholder: ()) -> Result<(), ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "Phase 3.3 で実装予定".to_string(),
        ))
    }

    pub fn find_by_post(&self, _post_id_placeholder: ()) -> Result<(), ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "Phase 3.3 で実装予定".to_string(),
        ))
    }

    pub fn save(&self, _comment_placeholder: ()) -> Result<(), ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "Phase 3.3 で実装予定".to_string(),
        ))
    }

    pub fn delete(&self, _id_placeholder: ()) -> Result<(), ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "Phase 3.3 で実装予定".to_string(),
        ))
    }
}

impl Default for DieselCommentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diesel_comment_repository_creation() {
        let repo = DieselCommentRepository::new();
        let _ = repo;
    }
}
