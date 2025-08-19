@echo off
REM Database Migration Script - Windows版
REM PostgreSQL schema initialization and data migration

setlocal enabledelayedexpansion

echo 🗄️  PostgreSQL Migration Starting...
echo ===================================

REM 環境変数チェック
if "%DATABASE_URL%"=="" (
    echo ❌ DATABASE_URL environment variable is not set
    echo Please set it like: set "DATABASE_URL=postgresql://cms_user:secure_password@localhost:5432/production_cms"
    goto error
)

echo 📋 Checking PostgreSQL connection...
REM Dockerized PostgreSQLへの接続チェック
docker-compose exec -T postgres pg_isready -U cms_user -d production_cms >nul 2>&1
if errorlevel 1 (
    echo ❌ PostgreSQL is not accessible
    echo Make sure PostgreSQL service is running with: docker-compose up -d postgres
    goto error
)
echo ✅ PostgreSQL connection successful

echo 🔄 Running database migrations...

echo 📝 Creating initial schema...
docker-compose exec -T postgres psql %DATABASE_URL% << EOF
-- Initial Schema Migration
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_login TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Webauthn credentials table
CREATE TABLE IF NOT EXISTS webauthn_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA UNIQUE NOT NULL,
    public_key BYTEA NOT NULL,
    counter BIGINT NOT NULL DEFAULT 0,
    backup_eligible BOOLEAN NOT NULL DEFAULT false,
    backup_state BOOLEAN NOT NULL DEFAULT false,
    device_type VARCHAR(50) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used TIMESTAMP
);

-- Posts table
CREATE TABLE IF NOT EXISTS posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(500) NOT NULL,
    content TEXT NOT NULL,
    summary TEXT,
    author_id UUID NOT NULL REFERENCES users(id),
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    published_at TIMESTAMP,
    featured BOOLEAN NOT NULL DEFAULT false,
    view_count INTEGER NOT NULL DEFAULT 0,
    like_count INTEGER NOT NULL DEFAULT 0,
    comment_count INTEGER NOT NULL DEFAULT 0,
    seo_title VARCHAR(255),
    seo_description TEXT,
    slug VARCHAR(255) UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Categories table
CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    slug VARCHAR(255) UNIQUE NOT NULL,
    parent_id UUID REFERENCES categories(id),
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Tags table
CREATE TABLE IF NOT EXISTS tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    slug VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    color VARCHAR(7),
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Many-to-many relationships
CREATE TABLE IF NOT EXISTS post_categories (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, category_id)
);

CREATE TABLE IF NOT EXISTS post_tags (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- Comments table
CREATE TABLE IF NOT EXISTS comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    author_id UUID REFERENCES users(id),
    parent_id UUID REFERENCES comments(id),
    content TEXT NOT NULL,
    author_name VARCHAR(255),
    author_email VARCHAR(255),
    is_approved BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Media table
CREATE TABLE IF NOT EXISTS media (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename VARCHAR(500) NOT NULL,
    original_filename VARCHAR(500) NOT NULL,
    mime_type VARCHAR(255) NOT NULL,
    file_size BIGINT NOT NULL,
    width INTEGER,
    height INTEGER,
    alt_text TEXT,
    caption TEXT,
    uploaded_by UUID NOT NULL REFERENCES users(id),
    upload_path TEXT NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token TEXT UNIQUE NOT NULL,
    csrf_token TEXT,
    ip_address INET,
    user_agent TEXT,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_activity TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- API keys table
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    key_hash TEXT UNIQUE NOT NULL,
    permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    rate_limit INTEGER,
    expires_at TIMESTAMP,
    last_used TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Settings table
CREATE TABLE IF NOT EXISTS settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(255) UNIQUE NOT NULL,
    value JSONB NOT NULL,
    category VARCHAR(255) NOT NULL DEFAULT 'general',
    description TEXT,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
EOF

if errorlevel 1 (
    echo ❌ Schema creation failed
    goto error
)
echo ✅ Schema created successfully

echo 🔍 Creating indexes for performance...
docker-compose exec -T postgres psql %DATABASE_URL% << EOF
-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_posts_author_id ON posts(author_id);
CREATE INDEX IF NOT EXISTS idx_posts_status ON posts(status);
CREATE INDEX IF NOT EXISTS idx_posts_published_at ON posts(published_at);
CREATE INDEX IF NOT EXISTS idx_posts_featured ON posts(featured);
CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts(slug);
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at);

CREATE INDEX IF NOT EXISTS idx_comments_post_id ON comments(post_id);
CREATE INDEX IF NOT EXISTS idx_comments_author_id ON comments(author_id);
CREATE INDEX IF NOT EXISTS idx_comments_parent_id ON comments(parent_id);
CREATE INDEX IF NOT EXISTS idx_comments_is_approved ON comments(is_approved);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(session_token);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

CREATE INDEX IF NOT EXISTS idx_webauthn_user_id ON webauthn_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_webauthn_credential_id ON webauthn_credentials(credential_id);

CREATE INDEX IF NOT EXISTS idx_media_uploaded_by ON media(uploaded_by);
CREATE INDEX IF NOT EXISTS idx_media_mime_type ON media(mime_type);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);

CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);

CREATE INDEX IF NOT EXISTS idx_tags_slug ON tags(slug);
CREATE INDEX IF NOT EXISTS idx_settings_key ON settings(key);
CREATE INDEX IF NOT EXISTS idx_settings_category ON settings(category);
EOF

if errorlevel 1 (
    echo ❌ Index creation failed
    goto error
)
echo ✅ Indexes created successfully

echo 🔧 Creating triggers for updated_at columns...
docker-compose exec -T postgres psql %DATABASE_URL% << EOF
-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply triggers to tables with updated_at columns
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_posts_updated_at BEFORE UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_categories_updated_at BEFORE UPDATE ON categories FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_comments_updated_at BEFORE UPDATE ON comments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_settings_updated_at BEFORE UPDATE ON settings FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
EOF

if errorlevel 1 (
    echo ❌ Trigger creation failed
    goto error
)
echo ✅ Triggers created successfully

echo 📊 Inserting initial data...
docker-compose exec -T postgres psql %DATABASE_URL% << EOF
-- Insert default admin user (password: admin123)
INSERT INTO users (id, username, email, password_hash, role, is_active)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'admin',
    'admin@example.com',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj3Q6Q2kWzNK',
    'admin',
    true
) ON CONFLICT (username) DO NOTHING;

-- Insert sample categories
INSERT INTO categories (name, description, slug, sort_order) VALUES
    ('Technology', 'Technology related posts', 'technology', 1),
    ('Programming', 'Programming tutorials and tips', 'programming', 2),
    ('Web Development', 'Web development articles', 'web-development', 3),
    ('Rust', 'Rust programming language', 'rust', 4)
ON CONFLICT (name) DO NOTHING;

-- Insert sample tags
INSERT INTO tags (name, slug, description, color) VALUES
    ('rust', 'rust', 'Rust programming language', '#CE422B'),
    ('web', 'web', 'Web development', '#61DAFB'),
    ('api', 'api', 'API development', '#FF6B6B'),
    ('tutorial', 'tutorial', 'Tutorial content', '#4ECDC4'),
    ('cms', 'cms', 'Content Management System', '#45B7D1')
ON CONFLICT (name) DO NOTHING;

-- Insert default settings
INSERT INTO settings (key, value, category, description, is_public) VALUES
    ('site_title', '"Production CMS"', 'general', 'Site title', true),
    ('site_description', '"A powerful CMS built with Rust"', 'general', 'Site description', true),
    ('posts_per_page', '10', 'content', 'Number of posts per page', true),
    ('allow_registration', 'false', 'auth', 'Allow user registration', false),
    ('require_email_verification', 'true', 'auth', 'Require email verification', false),
    ('max_file_size', '10485760', 'media', 'Maximum file size in bytes (10MB)', false)
ON CONFLICT (key) DO NOTHING;
EOF

if errorlevel 1 (
    echo ❌ Initial data insertion failed
    goto error
)
echo ✅ Initial data inserted successfully

echo 🔍 Verifying migration...
echo Checking table counts:
docker-compose exec -T postgres psql %DATABASE_URL% -c "SELECT tablename FROM pg_tables WHERE schemaname = 'public';"

echo.
echo ✅ Database migration completed successfully!
echo.
echo 📊 Database Schema Summary:
echo   • Users: Authentication and user management
echo   • Posts: Content with full metadata
echo   • Categories & Tags: Content organization
echo   • Comments: User engagement
echo   • Media: File management
echo   • Sessions: Session management
echo   • API Keys: API access control
echo   • Settings: System configuration
echo   • WebAuthn: Passwordless authentication
echo.
echo 🔐 Default Admin Account:
echo   Username: admin
echo   Email: admin@example.com
echo   Password: admin123
echo   ⚠️  Please change the default password after first login!
echo.
goto end

:error
echo ❌ Migration failed
pause
exit /b 1

:end
echo Migration completed
pause
    echo ❌ Migrations directory not found: %MIGRATIONS_DIR%
    pause
    exit /b 1
)

REM マイグレーションテーブルの作成
echo 📋 Setting up migration tracking...
psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -c "CREATE TABLE IF NOT EXISTS schema_migrations (version VARCHAR(255) PRIMARY KEY, applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW());"
if errorlevel 1 (
    echo ❌ Failed to setup migration tracking
    pause
    exit /b 1
)

echo ✅ Migration tracking setup complete

REM マイグレーションファイルを順番に実行
echo 🔄 Running migrations...

for %%f in ("%MIGRATIONS_DIR%\*.sql") do (
    set "migration_file=%%f"
    set "filename=%%~nf"
    set "version=!filename!"
    
    REM 既に実行済みかチェック
    for /f %%a in ('psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -t -c "SELECT COUNT(*) FROM schema_migrations WHERE version='!version!'" 2^>nul') do set APPLIED=%%a
    
    if "!APPLIED!"=="0" (
        echo ▶️  Applying migration: !version!
        
        REM マイグレーションを実行
        psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -f "!migration_file!"
        if errorlevel 1 (
            echo ❌ Migration !version! failed
            pause
            exit /b 1
        )
        
        REM 成功した場合、記録を追加
        psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -c "INSERT INTO schema_migrations (version) VALUES ('!version!')"
        echo ✅ Migration !version! applied successfully
    ) else (
        echo ⏭️  Migration !version! already applied, skipping
    )
)

echo.
echo 🎉 All migrations completed successfully!
echo.

REM データベース統計を表示
echo 📊 Database Statistics:
echo ======================
psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -c "SELECT schemaname, tablename, n_tup_ins as inserts, n_tup_upd as updates, n_tup_del as deletes FROM pg_stat_user_tables ORDER BY schemaname, tablename;"

psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -c "SELECT 'Tables' as type, COUNT(*) as count FROM information_schema.tables WHERE table_schema = 'public' UNION ALL SELECT 'Indexes' as type, COUNT(*) as count FROM pg_indexes WHERE schemaname = 'public' UNION ALL SELECT 'Constraints' as type, COUNT(*) as count FROM information_schema.table_constraints WHERE table_schema = 'public';"

echo.
echo 🔗 Connection Info:
echo ==================
echo Host: %DB_HOST%:%DB_PORT%
echo Database: %DB_NAME%
echo User: %DB_USER%
echo.
echo 🚀 Database is ready for the Production CMS!
echo.
pause
