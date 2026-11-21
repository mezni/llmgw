use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use gateway::router;
use providers::mock::{router as mock_router, MockState};
use providers::OpenAiProvider;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn spawn_gateway() -> axum::Router {
    let mock_state = MockState::default();
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener");
    let mock_address = mock_listener.local_addr().expect("mock address");
    let mock_app = mock_router(mock_state);
    tokio::spawn(async move {
        axum::serve(mock_listener, mock_app)
            .await
            .expect("mock server");
    });

    let base_url = format!("http://{mock_address}/v1");
    let provider = OpenAiProvider::new(base_url, "test-key", Duration::from_secs(2));
    router(Arc::new(provider), 1024 * 1024)
}

async fn call(app: axum::Router, request: Request<Body>) -> Response {
    app.oneshot(request).await.expect("router responds")
}

fn chat_body() -> Value {
    json!({
        "model": "fast",
        "messages": [{ "role": "user", "content": "Say hello." }]
    })
}

#[tokio::test]
async fn health_returns_ok() {
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn chat_completion_works_end_to_end() {
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("x-request-id", "req-integration-1")
            .body(Body::from(chat_body().to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-request-id").unwrap(),
        "req-integration-1"
    );
    let body: Value = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|bytes| serde_json::from_slice(&bytes).expect("json body"))
        .expect("body read");
    assert_eq!(body["id"], "req-integration-1");
    assert_eq!(body["model"], "fast");
    assert_eq!(body["choices"][0]["message"]["content"], "echo: Say hello.");
    assert_eq!(body["object"], "chat.completion");
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(chat_body().to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let id = response
        .headers()
        .get("x-request-id")
        .expect("request id header")
        .to_str()
        .expect("valid header")
        .to_owned();
    assert!(!id.is_empty());
}

#[tokio::test]
async fn rejects_unsupported_fields() {
    let body = json!({
        "model": "fast",
        "messages": [{ "role": "user", "content": "hi" }],
        "logprobs": true
    });
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|bytes| serde_json::from_slice(&bytes).expect("json body"))
        .expect("body read");
    assert_eq!(body["error"]["code"], "unsupported_field");
}

#[tokio::test]
async fn maps_upstream_rate_limit() {
    let body = json!({
        "model": "fast",
        "messages": [{ "role": "user", "content": "hi" }],
        "x-mock-scenario": "rate_limited"
    });
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: Value = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|bytes| serde_json::from_slice(&bytes).expect("json body"))
        .expect("body read");
    assert_eq!(body["error"]["type"], "rate_limit_error");
}
