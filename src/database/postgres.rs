use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool, PoolError};
use r2d2_diesel::ConnectionManager as R2D2ConnectionManager;
use std::env;

use crate::AppError;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Database {
    pub pool: DbPool,
}

impl Database {
    pub fn new() -> Result<Self, AppError> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| AppError::Configuration("DATABASE_URL must be set".to_string()))?;
        
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(20)
            .min_idle(Some(5))
            .connection_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(Some(std::time::Duration::from_secs(600)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .build(manager)
            .map_err(|e| AppError::Database(format!("Failed to create connection pool: {}", e)))?;

        Ok(Database { pool })
    }

    pub fn get_conn(&self) -> Result<DbConnection, AppError> {
        self.pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        use diesel::sql_query;
        
        let mut conn = self.get_conn()?;
        sql_query("SELECT 1")
            .execute(&mut conn)
            .map_err(|e| AppError::Database(format!("Health check failed: {}", e)))?;
        
        Ok(())
    }

    pub fn run_migrations(&self) -> Result<(), AppError> {
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
        let mut conn = self.get_conn()?;
        
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| AppError::Database(format!("Migration failed: {}", e)))?;
        
        Ok(())
    }
}
