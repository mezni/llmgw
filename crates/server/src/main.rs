mod config;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use config::Config;
use gateway::{ProviderRegistry, UsageLog, USAGE_CHANNEL_CAPACITY};
use providers::mock::{router as mock_router, MockState};
use providers::{AnthropicProvider, OpenAiProvider};
use storage::UsageWriter;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    init_tracing(&config);

    let (base_url, api_key, upstream_is_mock) = match config.upstream_base_url.as_deref() {
        Some(url) => (
            url.to_owned(),
            config.upstream_api_key.clone().unwrap_or_default(),
            false,
        ),
        None => (spawn_mock().await?, "mock-key".to_owned(), true),
    };

    tracing::info!(
        data_listen = %config.data_listen,
        admin_listen = %config.admin_listen,
        log_mode = %config.log_mode,
        max_body_bytes = config.max_body_bytes,
        timeout_ms = config.default_timeout_ms,
        mock_upstream = upstream_is_mock,
        "gateway starting"
    );

    let pool = storage::connect(&config.database_url).await?;
    storage::migrate(&pool).await?;
    let (usage_log, receiver) = UsageLog::channel(USAGE_CHANNEL_CAPACITY);
    let writer = UsageWriter::new(pool, 100, Duration::from_millis(500));
    let _writer_handle = writer.spawn(receiver);

    let timeout = Duration::from_millis(config.default_timeout_ms);
    let openai = Arc::new(OpenAiProvider::new(base_url, api_key, timeout));
    let anthropic = config.anthropic_api_key.clone().map(|key| {
        Arc::new(AnthropicProvider::new(
            config.anthropic_base_url.clone(),
            key,
            timeout,
        )) as Arc<_>
    });
    let providers = ProviderRegistry::new(openai, anthropic);
    let app: Router = gateway::router(providers, usage_log, config.max_body_bytes);
    let listener = TcpListener::bind(&config.data_listen).await?;
    tracing::info!(address = %config.data_listen, "data plane listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing(config: &Config) {
    let filter = tracing_subscriber::EnvFilter::new(&config.log_level);
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false);
    if config.log_json {
        tracing::subscriber::set_global_default(subscriber.json().finish())
            .expect("tracing subscriber");
    } else {
        tracing::subscriber::set_global_default(subscriber.finish()).expect("tracing subscriber");
    }
}

async fn spawn_mock() -> Result<String, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = mock_router(MockState::default());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(format!("http://{address}/v1"))
}
