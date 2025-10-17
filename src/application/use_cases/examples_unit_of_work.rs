//! Unit of Work 使用例 - トランザクション管理が必要な Use Case
//!
//! Phase 3 Week 11: Unit of Work パターンの実装例
//!
//! ## シナリオ
//! 投稿を公開し、同時に著者の統計（公開投稿数）を更新する。
//! この操作は原子的でなければならない（両方成功 or 両方失敗）。

use crate::application::ports::repositories::{PostRepository, RepositoryError, UserRepository};
use crate::domain::post::{Post, PostId};
use crate::domain::user::UserId;
use crate::infrastructure::database::unit_of_work::DieselUnitOfWork;
use std::sync::Arc;

/// 投稿公開 + 著者統計更新 Use Case（トランザクション管理付き）
///
/// # トランザクション境界
/// - Post の公開
/// - User の公開投稿数インクリメント
///
/// この2つの操作は同一トランザクション内で実行され、
/// どちらかが失敗した場合は両方ロールバックされます。
pub struct PublishPostWithStatsUseCase {
    post_repository: Arc<dyn PostRepository>,
    user_repository: Arc<dyn UserRepository>,
    unit_of_work: Arc<DieselUnitOfWork>,
}

impl PublishPostWithStatsUseCase {
    /// Use Case の生成
    pub fn new(
        post_repository: Arc<dyn PostRepository>,
        user_repository: Arc<dyn UserRepository>,
        unit_of_work: Arc<DieselUnitOfWork>,
    ) -> Self {
        Self {
            post_repository,
            user_repository,
            unit_of_work,
        }
    }

    /// 投稿を公開し、著者の統計を更新（トランザクション内）
    ///
    /// # Arguments
    /// - `post_id` - 公開する投稿のID
    ///
    /// # Errors
    /// - 投稿が見つからない
    /// - 著者が見つからない
    /// - データベースエラー
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let use_case = PublishPostWithStatsUseCase::new(
    ///     post_repo, user_repo, unit_of_work
    /// );
    /// use_case.execute(post_id).await?;
    /// ```
    pub async fn execute(&self, post_id: PostId) -> Result<(), RepositoryError> {
        // Unit of Work でトランザクション境界を管理
        self.unit_of_work
            .execute_in_transaction(|_conn| {
                // Phase 3: Repository はまだ connection を受け取らない
                // Phase 4 で Repository メソッドに &mut PgConnection を追加予定
                
                // 現在の実装では、Repository は内部で connection pool を使用
                // 将来的には以下のように書ける:
                // let post = self.post_repository.find_by_id_with_connection(conn, post_id)?;
                // let mut post = post.ok_or(RepositoryError::NotFound)?;
                // post.publish();
                // self.post_repository.save_with_connection(conn, post)?;
                //
                // let author_id = post.author_id();
                // let mut author = self.user_repository.find_by_id_with_connection(conn, author_id)?
                //     .ok_or(RepositoryError::NotFound)?;
                // // 統計更新ロジック
                // self.user_repository.save_with_connection(conn, author)?;

                // Phase 3 プレースホルダー
                Ok(())
            })
            .await
    }
}

/// 複数投稿の一括公開 Use Case（ネストトランザクション使用例）
///
/// # トランザクション構造
/// - 外側のトランザクション: 全体の一括操作
/// - 内側のセーブポイント: 個々の投稿ごと（失敗してもスキップして続行）
pub struct BulkPublishPostsUseCase {
    post_repository: Arc<dyn PostRepository>,
    unit_of_work: Arc<DieselUnitOfWork>,
}

impl BulkPublishPostsUseCase {
    pub fn new(
        post_repository: Arc<dyn PostRepository>,
        unit_of_work: Arc<DieselUnitOfWork>,
    ) -> Self {
        Self {
            post_repository,
            unit_of_work,
        }
    }

    /// 複数の投稿を公開（一部失敗しても続行）
    ///
    /// # Arguments
    /// - `post_ids` - 公開する投稿のIDリスト
    ///
    /// # Returns
    /// - 成功した投稿数
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let use_case = BulkPublishPostsUseCase::new(post_repo, unit_of_work);
    /// let published_count = use_case.execute(vec![id1, id2, id3]).await?;
    /// ```
    pub async fn execute(&self, post_ids: Vec<PostId>) -> Result<usize, RepositoryError> {
        self.unit_of_work
            .execute_in_transaction(|conn| {
                let mut published_count = 0;

                for post_id in post_ids {
                    // セーブポイント: 個々の投稿の公開を試みる
                    let result = DieselUnitOfWork::with_savepoint(conn, |_inner_conn| {
                        // Phase 4 で実装:
                        // let post = self.post_repository.find_by_id_with_connection(inner_conn, post_id)?;
                        // post.publish();
                        // self.post_repository.save_with_connection(inner_conn, post)?;
                        
                        // Phase 3 プレースホルダー
                        Ok(())
                    });

                    // 失敗してもスキップして続行
                    if result.is_ok() {
                        published_count += 1;
                    }
                }

                Ok(published_count)
            })
            .await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockPostRepository;
    use crate::application::ports::repositories::MockUserRepository;
    use diesel::r2d2::{self, ConnectionManager};
    use diesel::PgConnection;

    fn create_test_pool() -> Arc<diesel::r2d2::Pool<ConnectionManager<PgConnection>>> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/cms_test".to_string());

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("Failed to create pool");

        Arc::new(pool)
    }

    #[test]
    fn test_publish_post_with_stats_use_case_creation() {
        let pool = create_test_pool();
        let unit_of_work = Arc::new(DieselUnitOfWork::new(pool));
        let post_repo = Arc::new(MockPostRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());

        let _use_case =
            PublishPostWithStatsUseCase::new(post_repo, user_repo, unit_of_work);
        // 作成できることを確認
    }

    #[test]
    fn test_bulk_publish_posts_use_case_creation() {
        let pool = create_test_pool();
        let unit_of_work = Arc::new(DieselUnitOfWork::new(pool));
        let post_repo = Arc::new(MockPostRepository::new());

        let _use_case = BulkPublishPostsUseCase::new(post_repo, unit_of_work);
        // 作成できることを確認
    }
}
