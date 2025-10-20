//! Unit of Work パターン実装
//!
//! ## 責務
//! - トランザクション境界の管理
//! - 複数の Repository 操作を単一のトランザクションでラップ
//! - 自動ロールバック（エラー時）および明示的コミット
//!
//! ## 設計方針
//! - クロージャベースAPI（自動クリーンアップ）
//! - Diesel の同期 API を tokio::task::spawn_blocking でラップ
//! - セーブポイント対応（ネストトランザクション）
//!
//! ## Phase 3 実装範囲
//! - 基本的なトランザクション管理
//! - エラーハンドリングと変換
//! - Phase 4 でより高度な機能（分散トランザクション等）を追加予定

use crate::application::ports::repositories::RepositoryError;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;

/// Diesel ベースの Unit of Work 実装
///
/// PostgreSQL のトランザクション管理を提供します。
///
/// # Examples
///
/// ```rust,ignore
/// let uow = DieselUnitOfWork::new(pool);
///
/// uow.execute_in_transaction(|conn| {
///     user_repo.save_with_connection(conn, user)?;
///     post_repo.save_with_connection(conn, post)?;
///     Ok(())
/// }).await?;
/// ```
pub struct DieselUnitOfWork {
    pool: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl DieselUnitOfWork {
    /// 新しい Unit of Work を作成
    ///
    /// # Arguments
    ///
    /// * `pool` - Diesel コネクションプール
    #[must_use]
    pub fn new(pool: Arc<Pool<ConnectionManager<PgConnection>>>) -> Self {
        Self { pool }
    }

    /// トランザクション内でクロージャを実行
    ///
    /// クロージャが成功（Ok）を返した場合はコミット、
    /// エラー（Err）を返した場合は自動的にロールバックします。
    ///
    /// # Type Parameters
    ///
    /// * `F` - トランザクション内で実行する関数
    /// * `R` - 関数の戻り値型
    ///
    /// # Arguments
    ///
    /// * `f` - PgConnection を受け取る関数
    ///
    /// # Errors
    ///
    /// - コネクション取得失敗
    /// - トランザクション開始失敗
    /// - クロージャ内でエラーが発生
    /// - コミット/ロールバック失敗
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let result = uow.execute_in_transaction(|conn| {
    ///     diesel::insert_into(users::table)
    ///         .values(&new_user)
    ///         .execute(conn)?;
    ///     Ok(user_id)
    /// }).await?;
    /// ```
    pub async fn execute_in_transaction<F, R>(&self, f: F) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut PgConnection) -> Result<R, RepositoryError> + Send + 'static,
        R: Send + 'static,
    {
        let pool = Arc::clone(&self.pool);

        // Diesel の同期 API を spawn_blocking でラップ
        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            // トランザクション開始
            conn.transaction::<R, RepositoryError, _>(|conn| f(conn))
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {e}")))?
    }

    /// ネストトランザクション（セーブポイント）を実行
    ///
    /// 既存のトランザクション内でセーブポイントを作成し、
    /// クロージャ内でエラーが発生した場合はそのセーブポイントまでロールバックします。
    ///
    /// # Type Parameters
    ///
    /// * `F` - セーブポイント内で実行する関数
    /// * `R` - 関数の戻り値型
    ///
    /// # Arguments
    ///
    /// * `conn` - トランザクション中の PgConnection
    /// * `f` - セーブポイント内で実行する関数
    ///
    /// # Errors
    ///
    /// - セーブポイント作成失敗
    /// - クロージャ内でエラーが発生
    /// - ロールバック失敗
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// conn.transaction::<_, RepositoryError, _>(|conn| {
    ///     user_repo.save_with_connection(conn, user)?;
    ///     
    ///     // ネストトランザクション（失敗しても user は保存される）
    ///     let _ = DieselUnitOfWork::with_savepoint(conn, |conn| {
    ///         risky_operation(conn)
    ///     });
    ///     
    ///     Ok(())
    /// })
    /// ```
    pub fn with_savepoint<F, R>(conn: &mut PgConnection, f: F) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut PgConnection) -> Result<R, RepositoryError>,
    {
        // Diesel の build_transaction() API を使用してセーブポイントを作成
        conn.build_transaction()
            .run::<R, RepositoryError, _>(|conn| f(conn))
    }

    /// トランザクション内で複数の操作を実行（タプルで結果を返す）
    ///
    /// 2つの操作を同じトランザクション内で実行し、両方の結果を返します。
    /// どちらかが失敗した場合は両方ロールバックされます。
    ///
    /// # Type Parameters
    ///
    /// * `F1`, `F2` - 実行する関数
    /// * `R1`, `R2` - 各関数の戻り値型
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let (user, post) = uow.execute_two_in_transaction(
    ///     |conn| user_repo.find_by_id_with_connection(conn, user_id),
    ///     |conn| post_repo.find_by_id_with_connection(conn, post_id),
    /// ).await?;
    /// ```
    pub async fn execute_two_in_transaction<F1, F2, R1, R2>(
        &self,
        f1: F1,
        f2: F2,
    ) -> Result<(R1, R2), RepositoryError>
    where
        F1: FnOnce(&mut PgConnection) -> Result<R1, RepositoryError> + Send + 'static,
        F2: FnOnce(&mut PgConnection) -> Result<R2, RepositoryError> + Send + 'static,
        R1: Send + 'static,
        R2: Send + 'static,
    {
        self.execute_in_transaction(move |conn| {
            let r1 = f1(conn)?;
            let r2 = f2(conn)?;
            Ok((r1, r2))
        })
        .await
    }

    /// トランザクション内で3つの操作を実行（タプルで結果を返す）
    pub async fn execute_three_in_transaction<F1, F2, F3, R1, R2, R3>(
        &self,
        f1: F1,
        f2: F2,
        f3: F3,
    ) -> Result<(R1, R2, R3), RepositoryError>
    where
        F1: FnOnce(&mut PgConnection) -> Result<R1, RepositoryError> + Send + 'static,
        F2: FnOnce(&mut PgConnection) -> Result<R2, RepositoryError> + Send + 'static,
        F3: FnOnce(&mut PgConnection) -> Result<R3, RepositoryError> + Send + 'static,
        R1: Send + 'static,
        R2: Send + 'static,
        R3: Send + 'static,
    {
        self.execute_in_transaction(move |conn| {
            let r1 = f1(conn)?;
            let r2 = f2(conn)?;
            let r3 = f3(conn)?;
            Ok((r1, r2, r3))
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
    use crate::infrastructure::database::schema::users;
    use diesel::r2d2::{self, ConnectionManager};

    /// テスト用のコネクションプールを作成
    fn create_test_pool() -> Arc<Pool<ConnectionManager<PgConnection>>> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/cms_test".to_string());

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder()
            .max_size(1) // テストでは1接続で十分
            .build(manager)
            .expect("Failed to create pool");

        Arc::new(pool)
    }

    #[tokio::test]
    #[ignore] // 実際の DB 接続が必要
    async fn test_execute_in_transaction_commit() {
        let pool = create_test_pool();
        let uow = DieselUnitOfWork::new(pool);

        let result = uow
            .execute_in_transaction(|conn| {
                // トランザクション内でクエリを実行
                let count: i64 = users::table.count().get_result(conn)?;
                Ok(count)
            })
            .await;

        assert!(result.is_ok(), "Transaction should succeed");
    }

    #[tokio::test]
    #[ignore] // 実際の DB 接続が必要
    async fn test_execute_in_transaction_rollback() {
        let pool = create_test_pool();
        let uow = DieselUnitOfWork::new(pool);

        let result: Result<(), RepositoryError> = uow
            .execute_in_transaction(|_conn| {
                // エラーを返すとロールバックされる
                Err(RepositoryError::DatabaseError(
                    "Intentional error".to_string(),
                ))
            })
            .await;

        assert!(result.is_err(), "Transaction should rollback on error");
    }

    #[test]
    #[ignore] // Requires a running Postgres instance; ignore in local/dev CI
    fn test_unit_of_work_creation() {
        let pool = create_test_pool();
        let _uow = DieselUnitOfWork::new(pool);
        // 作成できることを確認
    }

    #[tokio::test]
    #[ignore] // 実際の DB 接続が必要
    async fn test_execute_two_in_transaction() {
        let pool = create_test_pool();
        let uow = DieselUnitOfWork::new(pool);

        let result = uow
            .execute_two_in_transaction(
                |conn| {
                    let count: i64 = users::table.count().get_result(conn)?;
                    Ok(count)
                },
                |conn| {
                    let count: i64 = users::table.count().get_result(conn)?;
                    Ok(count + 1)
                },
            )
            .await;

        assert!(result.is_ok(), "Two operations should succeed");
        if let Ok((r1, r2)) = result {
            assert_eq!(
                r1 + 1,
                r2,
                "Second operation should return incremented value"
            );
        }
    }

    #[tokio::test]
    #[ignore] // 実際の DB 接続が必要
    async fn test_execute_three_in_transaction() {
        let pool = create_test_pool();
        let uow = DieselUnitOfWork::new(pool);

        let result = uow
            .execute_three_in_transaction(
                |conn| {
                    let count: i64 = users::table.count().get_result(conn)?;
                    Ok(count)
                },
                |conn| {
                    let count: i64 = users::table.count().get_result(conn)?;
                    Ok(count + 1)
                },
                |conn| {
                    let count: i64 = users::table.count().get_result(conn)?;
                    Ok(count + 2)
                },
            )
            .await;

        assert!(result.is_ok(), "Three operations should succeed");
    }
}
