// Integration test: verify Postgres connectivity and basic query.
#![cfg(feature = "database")]

#[cfg(test)]
mod tests {
    use diesel::PgConnection;
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sql_query;
    use std::env;

    #[test]
    fn db_connectivity_smoke() {
        // If DATABASE_URL is not set, skip (useful for local dev where integration infra isn't available)
        let Ok(database_url) = env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL not set; skipping DB connectivity integration test");
            return;
        };

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(2)
            .build(manager)
            .expect("failed to build pool");
        let mut conn = pool.get().expect("failed to get connection from pool");

        // Simple smoke query to assert DB is reachable
        let res = sql_query("SELECT 1").execute(&mut conn);
        assert!(res.is_ok(), "expected SELECT 1 to succeed against the DB");
    }
}
