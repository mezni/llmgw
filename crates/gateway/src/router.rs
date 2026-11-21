use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth;
use crate::provider_registry::ProviderRegistry;
use crate::routes;
use crate::usage::UsageLog;

#[derive(Clone)]
pub struct AppState {
    pub providers: ProviderRegistry,
    pub usage: UsageLog,
    pub pool: SqlitePool,
}

pub fn router(
    providers: ProviderRegistry,
    usage: UsageLog,
    max_body_bytes: usize,
    pool: SqlitePool,
) -> Router {
    let state = AppState {
        providers,
        usage,
        pool,
    };
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route(
            "/v1/chat/completions",
            axum::routing::post(routes::chat::handler),
        )
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware,
        ))
        .layer(middleware::from_fn(crate::request_id::middleware))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn ready() -> Json<serde_json::Value> {
    Json(json!({ "status": "ready" }))
}
