-- Drop all tables in reverse dependency order
DROP TABLE IF EXISTS api_keys;
DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS posts;
DROP TABLE IF EXISTS pages;
DROP TABLE IF EXISTS media_files;
DROP TABLE IF EXISTS users;

-- Drop the UUID extension if it exists
-- Drop pgcrypto extension if it was created by the migration
DROP EXTENSION IF EXISTS pgcrypto;
