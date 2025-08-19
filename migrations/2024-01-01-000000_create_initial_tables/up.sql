-- Create the users table
-- pgcrypto extension creation skipped: managed Postgres may disallow creating extensions
-- CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    email VARCHAR NOT NULL UNIQUE,
    password_hash VARCHAR,
    first_name VARCHAR,
    last_name VARCHAR,
    role VARCHAR NOT NULL DEFAULT 'subscriber',
    is_active BOOLEAN NOT NULL DEFAULT true,
    email_verified BOOLEAN NOT NULL DEFAULT false,
    last_login TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create the posts table
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    slug VARCHAR NOT NULL UNIQUE,
    content TEXT NOT NULL,
    excerpt TEXT,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR NOT NULL DEFAULT 'draft',
    featured_image_id UUID,
    tags TEXT[] DEFAULT '{}',
    categories TEXT[] DEFAULT '{}',
    meta_title VARCHAR,
    meta_description TEXT,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create the pages table
CREATE TABLE pages (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    slug VARCHAR NOT NULL UNIQUE,
    content TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR NOT NULL DEFAULT 'draft',
    meta_title VARCHAR,
    meta_description TEXT,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create the media_files table
CREATE TABLE media_files (
    id UUID PRIMARY KEY,
    filename VARCHAR NOT NULL,
    original_filename VARCHAR NOT NULL,
    mime_type VARCHAR NOT NULL,
    file_size BIGINT NOT NULL,
    file_path VARCHAR NOT NULL,
    alt_text TEXT,
    caption TEXT,
    uploaded_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create the api_keys table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    key_hash VARCHAR NOT NULL UNIQUE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permissions JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ
);

-- Create the settings table
CREATE TABLE settings (
    id UUID PRIMARY KEY,
    key VARCHAR NOT NULL UNIQUE,
    value JSONB NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_posts_author_id ON posts(author_id);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_published_at ON posts(published_at);
CREATE INDEX idx_posts_slug ON posts(slug);

CREATE INDEX idx_pages_author_id ON pages(author_id);
CREATE INDEX idx_pages_status ON pages(status);
CREATE INDEX idx_pages_slug ON pages(slug);

CREATE INDEX idx_media_files_uploaded_by ON media_files(uploaded_by);
CREATE INDEX idx_media_files_mime_type ON media_files(mime_type);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);

CREATE INDEX idx_settings_key ON settings(key);

-- Add foreign key constraint for featured_image_id
ALTER TABLE posts ADD CONSTRAINT fk_posts_featured_image 
    FOREIGN KEY (featured_image_id) REFERENCES media_files(id) ON DELETE SET NULL;
