use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

#[derive(Clone, Default)]
pub struct MockState {
    requests: Arc<Mutex<Vec<Value>>>,
}

impl MockState {
    pub fn recorded(&self) -> Vec<Value> {
        self.requests
            .lock()
            .map(|values| values.clone())
            .unwrap_or_default()
    }

    fn record(&self, request: &Value) {
        if let Ok(mut values) = self.requests.lock() {
            values.push(request.clone());
        }
    }
}

pub fn router(state: MockState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat))
        .route("/__mock/requests", get(recorded))
        .with_state(state)
}

async fn recorded(State(state): State<MockState>) -> Json<Value> {
    Json(json!({ "requests": state.recorded() }))
}

async fn chat(
    State(state): State<MockState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    state.record(&request);

    let scenario = headers
        .get("x-mock-scenario")
        .and_then(|value| value.to_str().ok())
        .or_else(|| request.get("x-mock-scenario").and_then(Value::as_str))
        .unwrap_or("ok")
        .to_owned();

    match scenario.as_str() {
        "delay" => {
            tokio::time::sleep(Duration::from_millis(400)).await;
            success(&request)
        }
        "rate_limited" => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(error_body(
                "rate_limit_error",
                "rate_limit_exceeded",
                "slow down",
            )),
        )
            .into_response(),
        "server_error" => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": {
                    "message": "upstream failed with key sk-hostile-fake",
                    "type": "internal_error",
                    "code": "internal_error"
                }
            })),
        )
            .into_response(),
        "bad_request" => (
            StatusCode::BAD_REQUEST,
            Json(error_body(
                "invalid_request_error",
                "invalid_request",
                "bad request",
            )),
        )
            .into_response(),
        _ => success(&request),
    }
}

fn success(request: &Value) -> Response {
    let content = request
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| messages.last())
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let alias = request
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("fast")
        .to_owned();

    (
        StatusCode::OK,
        Json(json!({
            "id": "chatcmpl-mock-001",
            "object": "chat.completion",
            "created": 1763664000,
            "model": alias,
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": format!("echo: {content}") },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 5,
                "total_tokens": 17
            }
        })),
    )
        .into_response()
}

fn error_body(error_type: &str, code: &str, message: &str) -> Value {
    json!({ "error": { "message": message, "type": error_type, "code": code } })
}
