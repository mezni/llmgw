use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use thiserror::Error;

pub mod usage;

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
    use super::*;

    #[tokio::test]
    async fn migration_creates_request_log_schema() {
        let path = std::env::temp_dir().join(format!(
            "llmgw-schema-{}-{}.db",
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

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'requests'",
        )
        .fetch_one(&pool)
        .await
        .expect("table count");
        assert_eq!(count, 1);
        let columns = sqlx::query("PRAGMA table_info(requests)")
            .fetch_all(&pool)
            .await
            .expect("columns");
        assert_eq!(columns.len(), 21);
        let indexes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'requests'",
        )
        .fetch_one(&pool)
        .await
        .expect("index count");
        assert_eq!(indexes, 5);

        pool.close().await;
        let _ = std::fs::remove_file(path);
    }
}
