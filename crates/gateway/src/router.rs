use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::provider_registry::ProviderRegistry;
use crate::routes;
use crate::usage::UsageLog;

#[derive(Clone)]
pub struct AppState {
    pub providers: ProviderRegistry,
    pub usage: UsageLog,
}

pub fn router(providers: ProviderRegistry, usage: UsageLog, max_body_bytes: usize) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/v1/chat/completions",
            axum::routing::post(routes::chat::handler),
        )
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .layer(middleware::from_fn(crate::request_id::middleware))
        .with_state(AppState { providers, usage })
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
