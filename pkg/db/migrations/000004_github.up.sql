ALTER TABLE users ADD COLUMN github_id TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_github_id ON users(github_id);
