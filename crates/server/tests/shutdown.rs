use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gateway::{router, ProviderRegistry, UsageLog};
use providers::mock::{router as mock_router, MockState};
use providers::OpenAiProvider;
use serde_json::json;
use sqlx::SqlitePool;
use storage::UsageWriter;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

async fn spawn_mock() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener");
    let address = listener.local_addr().expect("mock address");
    let app = mock_router(MockState::default());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{address}/v1")
}

async fn fresh_pool() -> (SqlitePool, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "llmgw-shutdown-{}-{}.db",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let pool = storage::connect(&format!("sqlite://{}", path.display()))
        .await
        .expect("connect");
    storage::migrate(&pool).await.expect("migrate");
    (pool, path)
}

#[tokio::test]
async fn graceful_shutdown_drains_all_usage_events() {
    let (pool, path) = fresh_pool().await;
    let project = storage::ensure_project(&pool, "shutdown")
        .await
        .expect("project");
    let key = storage::create_key(&pool, &project.id).await.expect("key");
    let base_url = spawn_mock().await;
    let openai = Arc::new(OpenAiProvider::new(
        base_url,
        "test-key",
        Duration::from_secs(2),
    ));
    let (usage, receiver) = UsageLog::channel(16);
    let writer = UsageWriter::new(pool.clone(), 100, Duration::from_millis(500));
    let writer_handle = writer.spawn(receiver);
    let app = router(
        ProviderRegistry::new(openai, None),
        usage.clone(),
        1024 * 1024,
        pool.clone(),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("gateway listener");
    let address = listener.local_addr().expect("gateway address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("server");
    });

    let response = reqwest::Client::new()
        .post(format!("http://{address}/v1/chat/completions"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", key.secret))
        .json(&json!({
            "model": "fast",
            "messages": [{ "role": "user", "content": "shutdown smoke" }]
        }))
        .send()
        .await
        .expect("request");
    assert!(response.status().is_success());

    shutdown_tx.send(()).expect("shutdown signal");
    server.await.expect("server drains requests");
    usage.close();
    writer_handle.await.expect("writer drains channel");

    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests")
        .fetch_one(&pool)
        .await
        .expect("row count");
    assert_eq!(rows, 1);
    let project_id: String = sqlx::query_scalar("SELECT project_id FROM requests LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("project id");
    assert_eq!(project_id, project.id);

    pool.close().await;
    let _ = std::fs::remove_file(path);
}
