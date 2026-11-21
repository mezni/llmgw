use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use thiserror::Error;

pub mod keys;
pub mod usage;

pub use keys::*;
pub use usage::UsageWriter;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

pub async fn connect(url: &str) -> Result<SqlitePool, StorageError> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), StorageError> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::Row;

    use super::*;
    use crate::keys::{
        create_key, create_project, ensure_project, find_active_key, hash_key, list_keys,
        revoke_project_key,
    };

    async fn test_pool(label: &str) -> (sqlx::SqlitePool, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "llmgw-{}-{}-{}.db",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let pool = connect(&format!("sqlite://{}", path.display()))
            .await
            .expect("connect");
        migrate(&pool).await.expect("migrate");
        (pool, path)
    }

    #[tokio::test]
    async fn migration_creates_schema_and_enables_wal() {
        let (pool, path) = test_pool("schema").await;

        for table in ["projects", "api_keys", "requests"] {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .expect("table count");
            assert_eq!(count, 1, "missing {table}");
        }
        let key_columns = sqlx::query("PRAGMA table_info(api_keys)")
            .fetch_all(&pool)
            .await
            .expect("api_keys columns");
        assert_eq!(key_columns.len(), 9);
        let key_indexes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'api_keys'",
        )
        .fetch_one(&pool)
        .await
        .expect("index count");
        assert_eq!(key_indexes, 3);
        let requests_columns = sqlx::query("PRAGMA table_info(requests)")
            .fetch_all(&pool)
            .await
            .expect("requests columns");
        assert_eq!(requests_columns.len(), 21);
        let requests_indexes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'requests'",
        )
        .fetch_one(&pool)
        .await
        .expect("index count");
        assert_eq!(requests_indexes, 5);

        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .expect("journal mode");
        assert_eq!(journal_mode, "wal");
        let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&pool)
            .await
            .expect("busy timeout");
        assert_eq!(busy_timeout, 5000);
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .expect("foreign keys");
        assert_eq!(foreign_keys, 1);

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn key_lifecycle_stores_only_hash_and_prefix() {
        let (pool, path) = test_pool("keys").await;
        let project = create_project(&pool, "demo").await.expect("project");
        let key = create_key(&pool, &project.id).await.expect("key");

        assert!(key.secret.starts_with("gw-"));
        assert!(key.secret.starts_with(&key.prefix));
        let row = sqlx::query("SELECT prefix, key_hash FROM api_keys WHERE id = ?")
            .bind(&key.id)
            .fetch_one(&pool)
            .await
            .expect("key row");
        let stored_prefix: String = row.get("prefix");
        let stored_hash: Vec<u8> = row.get("key_hash");
        assert_eq!(stored_prefix, key.prefix);
        assert_eq!(stored_hash, hash_key(&key.secret));
        let stored_secret: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM api_keys WHERE CAST(key_hash AS TEXT) LIKE '%gw-%'",
        )
        .fetch_one(&pool)
        .await
        .expect("secret scan");
        assert_eq!(stored_secret, 0);

        let active = find_active_key(&pool, &key.secret)
            .await
            .expect("lookup")
            .expect("active key");
        assert_eq!(active.project_id, project.id);
        assert_eq!(active.key_id, key.id);
        assert!(find_active_key(&pool, "gw-not-the-key")
            .await
            .expect("invalid lookup")
            .is_none());

        let keys = list_keys(&pool, &project.id).await.expect("list");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].prefix, key.prefix);
        assert!(keys[0].revoked_at.is_none());

        let revoked = revoke_project_key(&pool, &project.id, &key.id)
            .await
            .expect("revoke");
        assert!(revoked);
        assert!(find_active_key(&pool, &key.secret)
            .await
            .expect("revoked lookup")
            .is_none());
        assert!(!revoke_project_key(&pool, &project.id, &key.id)
            .await
            .expect("second revoke"));

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn duplicate_key_hashes_are_rejected() {
        let (pool, path) = test_pool("duplicate").await;
        let project = create_project(&pool, "demonstration")
            .await
            .expect("project");
        let key = create_key(&pool, &project.id).await.expect("key");
        let duplicate = sqlx::query(
            "INSERT INTO api_keys (id, project_id, prefix, key_hash, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&project.id)
        .bind(&key.prefix)
        .bind(hash_key(&key.secret))
        .bind("2026-11-20T00:00:00Z")
        .execute(&pool)
        .await;
        assert!(duplicate.is_err());

        let reused = ensure_project(&pool, "demonstration").await.expect("reuse");
        assert_eq!(reused.id, project.id);

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }
}
