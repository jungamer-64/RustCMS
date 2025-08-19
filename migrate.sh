#!/bin/bash

# データベースマイグレーションスクリプト
# PostgreSQL用

set -e  # エラー時に停止

# 環境変数の設定
DB_NAME=${DB_NAME:-"production_cms"}
DB_USER=${DB_USER:-"cms_user"}
DB_PASSWORD=${DB_PASSWORD:-"cms_password"}
DB_HOST=${DB_HOST:-"localhost"}
DB_PORT=${DB_PORT:-"5432"}

echo "🗄️  PostgreSQL Migration Script"
echo "================================"

# PostgreSQLサービスの確認
echo "📋 Checking PostgreSQL connection..."
if ! pg_isready -h $DB_HOST -p $DB_PORT -U $DB_USER; then
    echo "❌ PostgreSQL is not ready. Please check your connection settings."
    exit 1
fi

echo "✅ PostgreSQL connection confirmed"

# データベースが存在しない場合は作成
echo "📋 Checking if database exists..."
DB_EXISTS=$(PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -t -c "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'" 2>/dev/null | grep -c 1 || true)

if [ "$DB_EXISTS" -eq "0" ]; then
    echo "🔨 Creating database: $DB_NAME"
    PGPASSWORD=$DB_PASSWORD createdb -h $DB_HOST -p $DB_PORT -U $DB_USER $DB_NAME
    echo "✅ Database created successfully"
else
    echo "ℹ️  Database already exists"
fi

# マイグレーションファイルのディレクトリ
MIGRATIONS_DIR="./migrations"

if [ ! -d "$MIGRATIONS_DIR" ]; then
    echo "❌ Migrations directory not found: $MIGRATIONS_DIR"
    exit 1
fi

# マイグレーションテーブルの作成（実行済みマイグレーションを追跡）
echo "📋 Setting up migration tracking..."
PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME <<EOF
CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(255) PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
EOF

echo "✅ Migration tracking setup complete"

# マイグレーションファイルを順番に実行
echo "🔄 Running migrations..."
for migration_file in $(ls $MIGRATIONS_DIR/*.sql | sort); do
    # ファイル名からバージョンを取得
    version=$(basename "$migration_file" .sql)
    
    # 既に実行済みかチェック
    APPLIED=$(PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -t -c "SELECT COUNT(*) FROM schema_migrations WHERE version='$version'" 2>/dev/null | xargs)
    
    if [ "$APPLIED" -eq "0" ]; then
        echo "▶️  Applying migration: $version"
        
        # マイグレーションを実行
        if PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f "$migration_file"; then
            # 成功した場合、記録を追加
            PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "INSERT INTO schema_migrations (version) VALUES ('$version')"
            echo "✅ Migration $version applied successfully"
        else
            echo "❌ Migration $version failed"
            exit 1
        fi
    else
        echo "⏭️  Migration $version already applied, skipping"
    fi
done

echo ""
echo "🎉 All migrations completed successfully!"
echo ""

# データベース統計を表示
echo "📊 Database Statistics:"
echo "======================"
PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME <<EOF
SELECT 
    schemaname,
    tablename,
    n_tup_ins as inserts,
    n_tup_upd as updates,
    n_tup_del as deletes
FROM pg_stat_user_tables 
ORDER BY schemaname, tablename;

SELECT 
    'Tables' as type,
    COUNT(*) as count
FROM information_schema.tables 
WHERE table_schema = 'public'
UNION ALL
SELECT 
    'Indexes' as type,
    COUNT(*) as count
FROM pg_indexes 
WHERE schemaname = 'public'
UNION ALL
SELECT 
    'Constraints' as type,
    COUNT(*) as count
FROM information_schema.table_constraints 
WHERE table_schema = 'public';
EOF

echo ""
echo "🔗 Connection Info:"
echo "=================="
echo "Host: $DB_HOST:$DB_PORT"
echo "Database: $DB_NAME"
echo "User: $DB_USER"
echo ""
echo "🚀 Database is ready for the Production CMS!"
