use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use providers::Provider;
use serde_json::json;

use crate::routes;

#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn Provider>,
}

pub fn router(provider: Arc<dyn Provider>, max_body_bytes: usize) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/v1/chat/completions",
            axum::routing::post(routes::chat::handler),
        )
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .layer(middleware::from_fn(crate::request_id::middleware))
        .with_state(AppState { provider })
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
