use chrono::{SecondsFormat, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::StorageError;

use super::{generate_key, hash_key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedKey {
    pub id: String,
    pub project_id: String,
    pub prefix: String,
    pub secret: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySummary {
    pub id: String,
    pub project_id: String,
    pub prefix: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedKey {
    pub project_id: String,
    pub key_id: String,
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub async fn create_project(pool: &SqlitePool, name: &str) -> Result<Project, StorageError> {
    let id = Uuid::new_v4().to_string();
    let created_at = now();
    let spend_period = Utc::now().format("%Y-%m").to_string();
    sqlx::query("INSERT INTO projects (id, name, spend_period, created_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(spend_period)
        .bind(&created_at)
        .execute(pool)
        .await?;
    Ok(Project {
        id,
        name: name.to_owned(),
        created_at,
    })
}

pub async fn ensure_project(pool: &SqlitePool, name: &str) -> Result<Project, StorageError> {
    let id = Uuid::new_v4().to_string();
    let created_at = now();
    let spend_period = Utc::now().format("%Y-%m").to_string();
    sqlx::query(
        "INSERT INTO projects (id, name, spend_period, created_at) VALUES (?, ?, ?, ?) \
         ON CONFLICT(name) DO NOTHING",
    )
    .bind(&id)
    .bind(name)
    .bind(spend_period)
    .bind(&created_at)
    .execute(pool)
    .await?;
    let row = sqlx::query("SELECT id, name, created_at FROM projects WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(Project {
        id: row.get("id"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    })
}

pub async fn create_key(pool: &SqlitePool, project_id: &str) -> Result<CreatedKey, StorageError> {
    let generated = generate_key();
    let id = Uuid::new_v4().to_string();
    let created_at = now();
    sqlx::query(
        "INSERT INTO api_keys (id, project_id, prefix, key_hash, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(project_id)
    .bind(&generated.prefix)
    .bind(&generated.hash)
    .bind(&created_at)
    .execute(pool)
    .await?;
    Ok(CreatedKey {
        id,
        project_id: project_id.to_owned(),
        prefix: generated.prefix,
        secret: generated.secret,
        created_at,
    })
}

pub async fn find_active_key(
    pool: &SqlitePool,
    secret: &str,
) -> Result<Option<AuthenticatedKey>, StorageError> {
    let hash = hash_key(secret);
    let now = now();
    let row = sqlx::query(
        "SELECT id, project_id FROM api_keys \
         WHERE key_hash = ? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > ?)",
    )
    .bind(hash)
    .bind(now)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| AuthenticatedKey {
        project_id: row.get("project_id"),
        key_id: row.get("id"),
    }))
}

pub async fn find_project(pool: &SqlitePool, name: &str) -> Result<Option<Project>, StorageError> {
    let row = sqlx::query("SELECT id, name, created_at FROM projects WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| Project {
        id: row.get("id"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    }))
}

pub async fn list_keys(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<KeySummary>, StorageError> {
    let rows = sqlx::query(
        "SELECT id, project_id, prefix, expires_at, revoked_at, created_at \
         FROM api_keys WHERE project_id = ? ORDER BY created_at, id",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| KeySummary {
            id: row.get("id"),
            project_id: row.get("project_id"),
            prefix: row.get("prefix"),
            expires_at: row.get("expires_at"),
            revoked_at: row.get("revoked_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}

pub async fn revoke_key(pool: &SqlitePool, key_id: &str) -> Result<bool, StorageError> {
    let result =
        sqlx::query("UPDATE api_keys SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(now())
            .bind(key_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn revoke_project_key(
    pool: &SqlitePool,
    project_id: &str,
    key_id: &str,
) -> Result<bool, StorageError> {
    let result = sqlx::query(
        "UPDATE api_keys SET revoked_at = ? \
         WHERE id = ? AND project_id = ? AND revoked_at IS NULL",
    )
    .bind(now())
    .bind(key_id)
    .bind(project_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}
