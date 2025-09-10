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

    pub fn get(&self) -> Result<PooledConnection, crate::AppError> {
        self.pool.get().map_err(|e| {
            crate::AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            ))
        })
    }

    pub async fn health_check(&self) -> Result<(), crate::AppError> {
        use diesel::prelude::*;
        use diesel::sql_query;

        let mut conn = self.get()?;
        sql_query("SELECT 1").execute(&mut conn)?;
        Ok(())
    }
}
