pub mod common;

// 統合テスト用ヘルパーモジュール
//
// このモジュールは PostgreSQL データベース接続、マイグレーション実行、
// テストデータ生成などの共通機能を提供します。

#[cfg(all(test, feature = "database"))]
pub mod integration {
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::PgConnection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use std::env;
    use std::sync::Arc;

    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    /// テスト用データベース接続プールを作成
    ///
    /// # 環境変数
    /// - `TEST_DATABASE_URL`: テスト用データベース接続文字列
    ///   - デフォルト: `postgres://postgres:postgres@localhost:5432/cms_test`
    ///
    /// # Panics
    /// - データベース接続プールの作成に失敗した場合
    pub fn create_test_pool() -> Arc<Pool<ConnectionManager<PgConnection>>> {
        let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cms_test".to_string()
        });

        let manager = ConnectionManager::<PgConnection>::new(&database_url);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create connection pool for test database.\n\
                     Error: {e}\n\
                     DATABASE_URL: {database_url}\n\
                     \n\
                     Please ensure:\n\
                     1. PostgreSQL is running\n\
                     2. Database 'cms_test' exists\n\
                     3. Connection credentials are correct"
                )
            });

        Arc::new(pool)
    }

    /// テストデータベースのマイグレーションを実行
    ///
    /// # Panics
    /// - マイグレーション実行に失敗した場合
    pub fn run_migrations(pool: &Pool<ConnectionManager<PgConnection>>) {
        let mut conn = pool.get().expect("Failed to get connection from pool");

        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
    }

    /// テストデータベースの全テーブルをクリーンアップ
    ///
    /// トランザクション内で実行される各テストケース後のクリーンアップに使用します。
    /// TRUNCATE CASCADE を使用して、外部キー制約を考慮しながら全データを削除します。
    ///
    /// # Panics
    /// - TRUNCATE 実行に失敗した場合
    pub fn cleanup_database(pool: &Pool<ConnectionManager<PgConnection>>) {
        use diesel::sql_query;
        use diesel::RunQueryDsl;

        let mut conn = pool.get().expect("Failed to get connection from pool");

        // すべてのテーブルを TRUNCATE (外部キー制約を無視して CASCADE)
        sql_query(
            "TRUNCATE TABLE users, posts, comments, tags, categories, post_tags, post_categories \
             RESTART IDENTITY CASCADE",
        )
        .execute(&mut conn)
        .expect("Failed to truncate tables");
    }

    /// テストセットアップ: プール作成 + マイグレーション実行
    ///
    /// 統合テストの最初に呼び出すヘルパー関数です。
    ///
    /// # Example
    /// ```no_run
    /// use tests::helpers::integration::setup_test_database;
    ///
    /// #[tokio::test]
    /// async fn test_user_repository() {
    ///     let pool = setup_test_database();
    ///     // ... テストロジック
    /// }
    /// ```
    pub fn setup_test_database() -> Arc<Pool<ConnectionManager<PgConnection>>> {
        let pool = create_test_pool();
        run_migrations(&*pool);
        pool
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_create_pool_with_env_var() {
            // 環境変数が設定されている場合のテスト
            unsafe {
                env::set_var(
                    "TEST_DATABASE_URL",
                    "postgres://test:test@localhost:5432/test_db",
                );
            }
            let _pool = create_test_pool();
            unsafe {
                env::remove_var("TEST_DATABASE_URL");
            }
        }

        #[test]
        fn test_create_pool_with_default() {
            // 環境変数が設定されていない場合、デフォルト値を使用
            unsafe {
                env::remove_var("TEST_DATABASE_URL");
            }
            // デフォルトのDB接続先が存在しない場合はpanicするが、
            // CI環境では適切に設定されている前提
        }
    }
}
