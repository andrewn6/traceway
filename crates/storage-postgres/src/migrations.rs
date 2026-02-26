//! Postgres schema migrations for the auth layer.

use auth::AuthStoreError;
use sqlx::PgPool;
use tracing::info;

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_auth_tables",
        r#"
	CREATE TABLE IF NOT EXISTS organizations (
            id          UUID PRIMARY KEY,
            name        TEXT NOT NULL,
            slug        TEXT NOT NULL UNIQUE,
            plan        TEXT NOT NULL DEFAULT 'free',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS users (
            id              UUID PRIMARY KEY,
            email           TEXT NOT NULL UNIQUE,
            name            TEXT,
            password_hash   TEXT,
            org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
            role            TEXT NOT NULL DEFAULT 'member',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        CREATE INDEX IF NOT EXISTS idx_users_org_id ON users(org_id);

        CREATE TABLE IF NOT EXISTS api_keys (
            id              UUID PRIMARY KEY,
            org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
            name            TEXT NOT NULL,
            key_prefix      TEXT NOT NULL,
            key_hash        TEXT NOT NULL,
            scopes          JSONB NOT NULL DEFAULT '[]',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_used_at    TIMESTAMPTZ,
            expires_at      TIMESTAMPTZ
        );
        CREATE INDEX IF NOT EXISTS idx_api_keys_org_id ON api_keys(org_id);
        CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix);

        CREATE TABLE IF NOT EXISTS invites (
            id              UUID PRIMARY KEY,
            org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
            email           TEXT NOT NULL,
            role            TEXT NOT NULL DEFAULT 'member',
            invited_by      UUID NOT NULL REFERENCES users(id),
            token_hash      TEXT NOT NULL UNIQUE,
            expires_at      TIMESTAMPTZ NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_invites_org_id ON invites(org_id);
        CREATE INDEX IF NOT EXISTS idx_invites_token ON invites(token_hash);

        -- Migration tracking
        CREATE TABLE IF NOT EXISTS _auth_migrations (
            name        TEXT PRIMARY KEY,
            applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    ),
    (
        "002_password_reset_tokens",
        r#"
        CREATE TABLE IF NOT EXISTS password_reset_tokens (
            id          UUID PRIMARY KEY,
            user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token_hash  TEXT NOT NULL UNIQUE,
            expires_at  TIMESTAMPTZ NOT NULL,
            used        BOOLEAN NOT NULL DEFAULT FALSE,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_password_reset_token ON password_reset_tokens(token_hash);
        CREATE INDEX IF NOT EXISTS idx_password_reset_user ON password_reset_tokens(user_id);
        "#,
    ),
    (
        "003_projects",
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            id          UUID PRIMARY KEY,
            org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
            name        TEXT NOT NULL,
            slug        TEXT NOT NULL,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(org_id, slug)
        );
        CREATE INDEX IF NOT EXISTS idx_projects_org_id ON projects(org_id);

        -- Add project_id column to api_keys (nullable for backwards compat during migration)
        ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS project_id UUID REFERENCES projects(id) ON DELETE CASCADE;
        CREATE INDEX IF NOT EXISTS idx_api_keys_project_id ON api_keys(project_id);

        -- Create a default project for every existing org
        INSERT INTO projects (id, org_id, name, slug, created_at, updated_at)
        SELECT gen_random_uuid(), id, 'Default', 'default', NOW(), NOW()
        FROM organizations
        WHERE NOT EXISTS (
            SELECT 1 FROM projects WHERE projects.org_id = organizations.id
        );

        -- Backfill api_keys with their org's default project
        UPDATE api_keys SET project_id = (
            SELECT p.id FROM projects p WHERE p.org_id = api_keys.org_id AND p.slug = 'default' LIMIT 1
        ) WHERE project_id IS NULL;
        "#,
    ),
];

/// Run pending migrations.
pub async fn run(pool: &PgPool) -> Result<(), AuthStoreError> {
    // Ensure migration table exists (it's created in first migration,
    // but we need it to check what's been applied)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _auth_migrations (
            name TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthStoreError::Database(e.to_string()))?;

    for (name, sql) in MIGRATIONS {
        let applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _auth_migrations WHERE name = $1)",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| AuthStoreError::Database(e.to_string()))?;

        if !applied {
            // Use raw_sql to support multi-statement migrations
            sqlx::raw_sql(sql)
                .execute(pool)
                .await
                .map_err(|e| AuthStoreError::Database(format!("Migration {}: {}", name, e)))?;

            sqlx::query("INSERT INTO _auth_migrations (name) VALUES ($1)")
                .bind(name)
                .execute(pool)
                .await
                .map_err(|e| AuthStoreError::Database(e.to_string()))?;

            info!(migration = name, "Applied auth migration");
        }
    }

    Ok(())
}
