// src/database/pool.rs
use diesel::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use std::sync::Arc;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone, Debug)]
pub struct DatabasePool {
    pool: Arc<Pool>,
}

impl DatabasePool {
    /// 接続プールを作成します。
    ///
    /// # Errors
    ///
    /// 接続プールの初期化に失敗した場合にエラーを返します。
    pub fn new(database_url: &str, max_connections: u32) -> Result<Self, crate::AppError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder()
            .max_size(max_connections)
            .build(manager)
            .map_err(|e| {
                crate::AppError::Database(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UnableToSendCommand,
                    Box::new(e.to_string()),
                ))
            })?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// プールから接続を取得します。
    ///
    /// # Errors
    ///
    /// 利用可能な接続が無い、もしくは取得に失敗した場合にエラーを返します。
    pub fn get(&self) -> Result<PooledConnection, crate::AppError> {
        self.pool.get().map_err(|e| {
            crate::AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            ))
        })
    }

    #[allow(clippy::unused_async)] // kept async for future async driver compatibility and API consistency
    /// ヘルスチェック用に簡単なクエリを実行します。
    ///
    /// # Errors
    ///
    /// クエリの実行に失敗した場合にエラーを返します。
    pub async fn health_check(&self) -> Result<(), crate::AppError> {
        use diesel::prelude::*;
        use diesel::sql_query;

        let mut conn = self.get()?;
        sql_query("SELECT 1").execute(&mut conn)?;
        Ok(())
    }
}
