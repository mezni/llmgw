use std::time::Duration;

use gateway_core::UsageEvent;
use sqlx::sqlite::SqlitePool;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct UsageWriter {
    pool: SqlitePool,
    batch_size: usize,
    interval: Duration,
}

impl UsageWriter {
    pub fn new(pool: SqlitePool, batch_size: usize, interval: Duration) -> Self {
        Self {
            pool,
            batch_size,
            interval,
        }
    }

    pub fn spawn(self, mut receiver: mpsc::Receiver<UsageEvent>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(self.batch_size);
            let mut ticker = tokio::time::interval(self.interval);
            loop {
                if batch.len() >= self.batch_size {
                    self.flush(&mut batch).await;
                    continue;
                }
                tokio::select! {
                    event = receiver.recv() => match event {
                        Some(event) => batch.push(event),
                        None => {
                            self.flush(&mut batch).await;
                            break;
                        }
                    },
                    _ = ticker.tick() => self.flush(&mut batch).await,
                }
            }
        })
    }

    async fn flush(&self, batch: &mut Vec<UsageEvent>) {
        if batch.is_empty() {
            return;
        }
        match self.insert(batch).await {
            Ok(()) => batch.clear(),
            Err(error) => {
                tracing::error!(event_count = batch.len(), error = %error, "usage batch write failed");
                batch.clear();
            }
        }
    }

    async fn insert(&self, batch: &[UsageEvent]) -> Result<(), sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        for event in batch {
            sqlx::query(
                "INSERT INTO requests
                 (id, ts, project_id, key_id, alias, tag, provider_id, model, stream,
                  input_tokens, output_tokens, cached_tokens, cost_usd, latency_ms, ttft_ms,
                  status, http_status, error_kind)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&event.id)
            .bind(&event.ts)
            .bind(&event.project_id)
            .bind(&event.key_id)
            .bind(&event.alias)
            .bind(&event.tag)
            .bind(&event.provider_id)
            .bind(&event.model)
            .bind(event.stream)
            .bind(event.input_tokens.map(i64::from))
            .bind(event.output_tokens.map(i64::from))
            .bind(event.cached_tokens.map(i64::from))
            .bind(event.cost_usd)
            .bind(event.latency_ms)
            .bind(event.ttft_ms)
            .bind(event.status.as_str())
            .bind(event.http_status.map(i64::from))
            .bind(&event.error_kind)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use gateway_core::UsageStatus;

    use super::*;
    use crate::{connect, migrate};

    fn event(id: &str) -> UsageEvent {
        UsageEvent {
            id: id.to_owned(),
            ts: "2026-11-20T12:00:00Z".to_owned(),
            project_id: "local".to_owned(),
            key_id: "local-key".to_owned(),
            alias: "fast".to_owned(),
            tag: None,
            provider_id: Some("openai".to_owned()),
            model: Some("gpt-test".to_owned()),
            stream: false,
            input_tokens: Some(12),
            output_tokens: Some(5),
            cached_tokens: None,
            cost_usd: None,
            latency_ms: Some(42),
            ttft_ms: None,
            status: UsageStatus::Ok,
            http_status: Some(200),
            error_kind: None,
        }
    }

    async fn fresh_pool() -> (SqlitePool, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "llmgw-writer-{}-{}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let pool = connect(&format!("sqlite://{}", path.display()))
            .await
            .expect("connect");
        migrate(&pool).await.expect("migrate");
        (pool, path)
    }

    async fn row_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM requests")
            .fetch_one(pool)
            .await
            .expect("count")
    }

    #[tokio::test]
    async fn flushes_when_batch_is_full() {
        let (pool, path) = fresh_pool().await;
        let (sender, receiver) = mpsc::channel(4);
        let handle = UsageWriter::new(pool.clone(), 2, Duration::from_secs(60)).spawn(receiver);

        sender.send(event("one")).await.expect("send");
        sender.send(event("two")).await.expect("send");

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if row_count(&pool).await == 2 {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("rows appear after a full batch");

        drop(sender);
        handle.await.expect("writer stops");
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn flushes_on_interval_and_on_channel_close() {
        let (pool, path) = fresh_pool().await;
        let (sender, receiver) = mpsc::channel(4);
        let handle =
            UsageWriter::new(pool.clone(), 100, Duration::from_millis(20)).spawn(receiver);

        sender.send(event("tick")).await.expect("send");
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if row_count(&pool).await == 1 {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("rows appear after the flush interval");

        sender.send(event("final")).await.expect("send");
        drop(sender);
        handle.await.expect("writer drains on close");
        assert_eq!(row_count(&pool).await, 2);
        pool.close().await;
        let _ = std::fs::remove_file(path);
    }
}
