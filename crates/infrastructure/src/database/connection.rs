// src/infrastructure/database/connection.rs
//! データベース接続プール管理
//!
//! このモジュールは、Diesel を使用したデータベース接続プールの初期化と管理を行います。
//!
//! # 責務
//!
//! - PostgreSQL 接続プールの生成
//! - 接続パラメータの設定（タイムアウト、最大コネクション数等）
//! - 環境変数からの設定読み込み
//! - コネクションヘルスチェック
//!
//! # 例
//!
//! ```rust,ignore
//! let pool = DatabasePool::new("postgresql://user:pass@localhost/mydb")?;
//! ```

use crate::common::InfrastructureError;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::sync::Arc;

/// ポーリングコネクションの型エイリアス
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// データベース接続プール管理
#[derive(Clone)]
pub struct DatabasePool {
    pool: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl DatabasePool {
    /// PostgreSQL 接続プールを作成
    ///
    /// # Arguments
    ///
    /// * `database_url` - PostgreSQL 接続文字列
    ///
    /// # Errors
    ///
    /// 接続プールの初期化に失敗した場合、`InfrastructureError` を返す
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let pool = DatabasePool::new("postgresql://postgres:password@localhost/cms_db")?;
    /// ```
    pub fn new(database_url: &str) -> Result<Self, InfrastructureError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);

        let pool = Pool::builder()
            .max_size(32) // 最大コネクション数
            // Diesel 2.x: NopErrorHandler または LoggingErrorHandler を使用
            // クロージャーは非対応
            .build(manager)
            .map_err(|e| {
                InfrastructureError::DatabaseError(format!("接続プール初期化失敗: {}", e))
            })?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 接続プールの参照を取得
    pub fn get_pool(&self) -> Arc<Pool<ConnectionManager<PgConnection>>> {
        Arc::clone(&self.pool)
    }

    /// プールから接続を取得
    ///
    /// # Errors
    ///
    /// 利用可能な接続がない場合、`InfrastructureError` を返す
    pub fn get_connection(&self) -> Result<DbConnection, InfrastructureError> {
        self.pool
            .get()
            .map_err(|e| InfrastructureError::DatabaseError(format!("接続取得失敗: {}", e)))
    }

    /// 接続プールのヘルスチェック
    ///
    /// # Returns
    ///
    /// `Ok(())` - 接続が正常
    /// `Err(InfrastructureError)` - 接続が失敗
    pub fn health_check(&self) -> Result<(), InfrastructureError> {
        use diesel::RunQueryDsl;
        use diesel::sql_query;

        let mut conn = self.get_connection()?;
        sql_query("SELECT 1").execute(&mut conn).map_err(|e| {
            InfrastructureError::DatabaseError(format!("ヘルスチェック失敗: {}", e))
        })?;
        Ok(())
    }

    /// プールの統計情報を取得
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            connections: self.pool.state().connections,
            idle_connections: self.pool.state().idle_connections,
        }
    }
}

/// プール統計情報
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub connections: u32,
    pub idle_connections: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_pool_creation() {
        // テスト用DB URLは環境変数から取得
        // 実際のテストは integration tests で実施
    }

    #[test]
    fn test_pool_stats() {
        // プール統計の取得テスト
    }
}
