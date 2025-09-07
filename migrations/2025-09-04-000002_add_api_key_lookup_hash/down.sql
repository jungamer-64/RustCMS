-- Revert deterministic lookup hash column
ALTER TABLE api_keys DROP COLUMN IF EXISTS api_key_lookup_hash;
DROP INDEX IF EXISTS idx_api_keys_lookup_hash;
