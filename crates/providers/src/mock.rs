use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::unfold;
use serde_json::{json, Value};

const STREAM_HELLO: &str = r#"data: {"id":"chatcmpl-mock-001","object":"chat.completion.chunk","created":1763664000,"model":"fast","choices":[{"index":0,"delta":{"role":"assistant","content":"He"},"finish_reason":null}]}

"#;

const STREAM_WORLD: &str = r#"data: {"id":"chatcmpl-mock-001","object":"chat.completion.chunk","created":1763664000,"model":"fast","choices":[{"index":0,"delta":{"content":"llo"},"finish_reason":null}]}

"#;

const STREAM_FINISH: &str = r#"data: {"id":"chatcmpl-mock-001","object":"chat.completion.chunk","created":1763664000,"model":"fast","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}}

"#;

const STREAM_DONE: &str = "data: [DONE]\n\n";

#[derive(Clone, Default)]
pub struct MockState {
    requests: Arc<Mutex<Vec<Value>>>,
    cancellations: Arc<AtomicUsize>,
}

impl MockState {
    pub fn recorded(&self) -> Vec<Value> {
        self.requests
            .lock()
            .map(|values| values.clone())
            .unwrap_or_default()
    }

    pub fn cancellations(&self) -> usize {
        self.cancellations.load(Ordering::SeqCst)
    }

    fn record(&self, request: &Value) {
        if let Ok(mut values) = self.requests.lock() {
            values.push(request.clone());
        }
    }
}

struct StreamGuard {
    cancellations: Arc<AtomicUsize>,
    completed: Arc<AtomicBool>,
}

impl Drop for StreamGuard {
    fn drop(&mut self) {
        if !self.completed.load(Ordering::SeqCst) {
            self.cancellations.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub fn router(state: MockState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat))
        .route("/__mock/requests", get(recorded))
        .route("/__mock/cancellations", get(cancellations))
        .with_state(state)
}

async fn recorded(State(state): State<MockState>) -> Json<Value> {
    Json(json!({ "requests": state.recorded() }))
}

async fn cancellations(State(state): State<MockState>) -> Json<Value> {
    Json(json!({ "count": state.cancellations() }))
}

async fn chat(
    State(state): State<MockState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    state.record(&request);

    let mut scenario = headers
        .get("x-mock-scenario")
        .and_then(|value| value.to_str().ok())
        .or_else(|| request.get("x-mock-scenario").and_then(Value::as_str))
        .unwrap_or("ok")
        .to_owned();
    if scenario == "ok" && request.get("stream").and_then(Value::as_bool) == Some(true) {
        scenario = "stream_ok".to_owned();
    }

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
        "stream_ok" | "stream_cut" | "stream_malformed" | "client_cancel" => {
            stream_response(&scenario, state)
        }
        _ => success(&request),
    }
}

fn stream_response(scenario: &str, state: MockState) -> Response {
    let frames: Vec<&'static str> = match scenario {
        "stream_ok" => vec![STREAM_HELLO, STREAM_WORLD, STREAM_FINISH, STREAM_DONE],
        "stream_cut" => vec![STREAM_HELLO],
        "stream_malformed" => vec!["data: not-json\n\n", STREAM_DONE],
        "client_cancel" => vec![
            STREAM_HELLO,
            STREAM_WORLD,
            STREAM_HELLO,
            STREAM_WORLD,
            STREAM_FINISH,
            STREAM_DONE,
        ],
        _ => vec![STREAM_DONE],
    };
    let paced = scenario == "client_cancel";
    let completed = Arc::new(AtomicBool::new(false));
    let guard = StreamGuard {
        cancellations: state.cancellations,
        completed: completed.clone(),
    };

    let stream = unfold(
        (frames.into_iter(), guard, paced, completed),
        |(mut frames, guard, paced, completed)| async move {
            let Some(frame) = frames.next() else {
                completed.store(true, Ordering::SeqCst);
                return None;
            };
            if paced {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let bytes = Bytes::from_static(frame.as_bytes());
            Some((
                Ok::<Bytes, std::io::Error>(bytes),
                (frames, guard, paced, completed),
            ))
        },
    );

    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("text/event-stream"),
    );
    response
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
