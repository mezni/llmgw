use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use futures_util::StreamExt;
use gateway::{router, UsageLog};
use gateway_core::{UsageEvent, UsageStatus};
use providers::mock::{router as mock_router, MockState};
use providers::OpenAiProvider;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tower::ServiceExt;

async fn spawn_gateway() -> axum::Router {
    let (app, _, _) = spawn_gateway_with_usage().await;
    app
}

async fn spawn_gateway_with_usage() -> (axum::Router, MockState, mpsc::Receiver<UsageEvent>) {
    let mock_state = MockState::default();
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener");
    let mock_address = mock_listener.local_addr().expect("mock address");
    let mock_app = mock_router(mock_state.clone());
    tokio::spawn(async move {
        axum::serve(mock_listener, mock_app)
            .await
            .expect("mock server");
    });

    let base_url = format!("http://{mock_address}/v1");
    let provider = OpenAiProvider::new(base_url, "test-key", Duration::from_secs(2));
    let (usage, receiver) = UsageLog::channel(16);
    (
        router(Arc::new(provider), usage, 1024 * 1024),
        mock_state,
        receiver,
    )
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

fn stream_body(scenario: &str) -> Value {
    json!({
        "model": "smart",
        "messages": [{ "role": "user", "content": "Say hello." }],
        "stream": true,
        "x-mock-scenario": scenario
    })
}

fn sse_events(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(str::to_owned)
        .collect()
}

async fn read_sse(response: Response) -> Vec<String> {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body read");
    sse_events(&String::from_utf8_lossy(&bytes))
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

#[tokio::test]
async fn streams_completion_as_openai_sse() {
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("x-request-id", "req-stream-1")
            .body(Body::from(stream_body("stream_ok").to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        response.headers().get("x-request-id").unwrap(),
        "req-stream-1"
    );

    let events = read_sse(response).await;
    assert_eq!(events.len(), 4);
    assert_eq!(events[3], "[DONE]");

    let first: Value = serde_json::from_str(&events[0]).expect("first chunk");
    assert_eq!(first["id"], "req-stream-1");
    assert_eq!(first["model"], "smart");
    assert_eq!(first["object"], "chat.completion.chunk");
    assert_eq!(first["choices"][0]["delta"]["content"], "He");

    let final_chunk: Value = serde_json::from_str(&events[2]).expect("final chunk");
    assert_eq!(final_chunk["choices"][0]["finish_reason"], "stop");
    assert_eq!(final_chunk["usage"]["total_tokens"], 17);
}

#[tokio::test]
async fn emits_error_event_without_done_on_cut_stream() {
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(stream_body("stream_cut").to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let events = read_sse(response).await;
    assert_eq!(events.len(), 2);
    let error: Value = serde_json::from_str(&events[1]).expect("error event");
    assert_eq!(error["error"]["type"], "upstream_error");
    assert!(!events.iter().any(|event| event == "[DONE]"));
}

#[tokio::test]
async fn emits_error_event_on_malformed_stream() {
    let response = call(
        spawn_gateway().await,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(stream_body("stream_malformed").to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let events = read_sse(response).await;
    assert_eq!(events.len(), 1);
    let error: Value = serde_json::from_str(&events[0]).expect("error event");
    assert_eq!(error["error"]["code"], "upstream_failed");
    assert!(!events.iter().any(|event| event == "[DONE]"));
}

#[tokio::test]
async fn client_disconnect_cancels_upstream_stream() {
    let (app, mock_state, mut usage_events) = spawn_gateway_with_usage().await;
    let response = call(
        app,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(stream_body("client_cancel").to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let mut frames = response.into_body().into_data_stream();
    let first = frames
        .next()
        .await
        .expect("first frame")
        .expect("valid frame");
    assert!(String::from_utf8_lossy(&first).contains("\"content\":\"He\""));
    drop(frames);

    let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if mock_state.cancellations() > 0 {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("cancellation observed");
    assert!(cancelled);

    let event = usage_events.try_recv().expect("cancellation usage event");
    assert_eq!(event.status, UsageStatus::ClientCancelled);
    assert_eq!(event.alias, "smart");
    assert!(event.ttft_ms.is_some());
}

#[tokio::test]
async fn logs_non_streaming_usage() {
    let (app, _, mut usage_events) = spawn_gateway_with_usage().await;
    let response = call(
        app,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("x-gateway-tag", "smoke")
            .header("x-request-id", "req-usage-1")
            .body(Body::from(chat_body().to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let event = usage_events.try_recv().expect("usage event");
    assert_eq!(event.id, "req-usage-1");
    assert_eq!(event.alias, "fast");
    assert_eq!(event.tag.as_deref(), Some("smoke"));
    assert_eq!(event.provider_id.as_deref(), Some("openai"));
    assert_eq!(event.input_tokens, Some(12));
    assert_eq!(event.output_tokens, Some(5));
    assert_eq!(event.status, UsageStatus::Ok);
    assert!(event.latency_ms.is_some());
}

#[tokio::test]
async fn logs_stream_usage_and_ttft() {
    let (app, _, mut usage_events) = spawn_gateway_with_usage().await;
    let response = call(
        app,
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("x-request-id", "req-stream-usage")
            .body(Body::from(stream_body("stream_ok").to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_sse(response).await.len(), 4);

    let event = usage_events.try_recv().expect("stream usage event");
    assert_eq!(event.id, "req-stream-usage");
    assert!(event.stream);
    assert_eq!(event.input_tokens, Some(12));
    assert_eq!(event.output_tokens, Some(5));
    assert!(event.ttft_ms.is_some());
    assert_eq!(event.status, UsageStatus::Ok);
}
