mod config;

use std::env;
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

    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("key") {
        return key_cli(&config.database_url, &args[1..]).await;
    }
    if !args.is_empty() {
        return Err(format!("unknown command '{}'", args[0]).into());
    }

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
    let writer = UsageWriter::new(pool.clone(), 100, Duration::from_millis(500));
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
    let app: Router = gateway::router(providers, usage_log, config.max_body_bytes, pool);
    let listener = TcpListener::bind(&config.data_listen).await?;
    tracing::info!(address = %config.data_listen, "data plane listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn key_cli(database_url: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let pool = storage::connect(database_url).await?;
    storage::migrate(&pool).await?;

    match args {
        [command, flag, project]
            if command == "create" && flag == "--project" && !project.is_empty() =>
        {
            let project = storage::ensure_project(&pool, project).await?;
            let key = storage::create_key(&pool, &project.id).await?;
            println!("id: {}", key.id);
            println!("project: {}", project.name);
            println!("prefix: {}", key.prefix);
            println!("secret: {}", key.secret);
            Ok(())
        }
        [command, flag, project]
            if command == "list" && flag == "--project" && !project.is_empty() =>
        {
            let Some(project) = storage::find_project(&pool, project).await? else {
                return Err(format!("project '{project}' does not exist").into());
            };
            let keys = storage::list_keys(&pool, &project.id).await?;
            if keys.is_empty() {
                println!("no keys for project {}", project.name);
                return Ok(());
            }
            for key in keys {
                let status = if key.revoked_at.is_some() {
                    "revoked"
                } else if key.expires_at.is_some() {
                    "expires"
                } else {
                    "active"
                };
                println!("{}\t{}\t{}\t{}", key.id, key.prefix, status, key.created_at);
            }
            Ok(())
        }
        [command, project_flag, project, key_flag, key_id]
            if command == "revoke"
                && project_flag == "--project"
                && key_flag == "--key-id"
                && !project.is_empty()
                && !key_id.is_empty() =>
        {
            let Some(project) = storage::find_project(&pool, project).await? else {
                return Err(format!("project '{project}' does not exist").into());
            };
            let revoked = storage::revoke_project_key(&pool, &project.id, key_id).await?;
            if !revoked {
                return Err(format!("key '{key_id}' is not active in project '{}'", project.name).into());
            }
            println!("revoked: {key_id}");
            Ok(())
        }
        _ => Err(
            "usage: key create --project NAME | key list --project NAME | key revoke --project NAME --key-id ID"
                .into(),
        ),
    }
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
