use std::collections::VecDeque;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Bytes;
use futures_core::stream::BoxStream;
use futures_util::{Stream, StreamExt};
use gateway_core::{ChatChunk, ChatRequest, ChatResponse, OpenAiStreamChunk};
use reqwest::Client;
use serde_json::{json, Value};

use crate::sse::{SseDecoder, SseEvent};
use crate::{
    classify_response_status, from_reqwest_error, parse_retry_after, Provider, ProviderError,
};

pub struct OpenAiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    timeout: Duration,
    id: String,
}

impl OpenAiProvider {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, timeout: Duration) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout,
            id: "openai".to_owned(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let response = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&request)
            .send()
            .await
            .map_err(from_reqwest_error)?;

        if !response.status().is_success() {
            return Err(classify_error(response).await);
        }

        response
            .json::<ChatResponse>()
            .await
            .map_err(from_reqwest_error)
    }

    async fn chat_stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        request.stream = true;
        request
            .extra
            .entry("stream_options")
            .or_insert_with(|| json!({ "include_usage": true }));

        let response = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&request)
            .send()
            .await
            .map_err(from_reqwest_error)?;

        if !response.status().is_success() {
            return Err(classify_error(response).await);
        }

        Ok(Box::pin(stream_chunks(response.bytes_stream())))
    }
}

struct StreamState<S> {
    bytes: S,
    decoder: SseDecoder,
    pending: VecDeque<SseEvent>,
    failed: bool,
}

fn stream_chunks<S>(
    bytes: S,
) -> impl Stream<Item = Result<ChatChunk, ProviderError>> + Send + 'static
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin + 'static,
{
    futures_util::stream::unfold(
        StreamState {
            bytes,
            decoder: SseDecoder::new(),
            pending: VecDeque::new(),
            failed: false,
        },
        |mut state| async move {
            if state.failed {
                return None;
            }

            loop {
                if let Some(event) = state.pending.pop_front() {
                    if event.data.trim() == "[DONE]" {
                        return None;
                    }
                    return match decode_chunk(&event.data) {
                        Ok(chunk) => Some((Ok(chunk), state)),
                        Err(error) => {
                            state.failed = true;
                            Some((Err(error), state))
                        }
                    };
                }

                match state.bytes.next().await {
                    Some(Ok(bytes)) => state.pending.extend(state.decoder.push(&bytes)),
                    Some(Err(error)) => {
                        state.failed = true;
                        return Some((
                            Err(ProviderError::Stream(format!(
                                "upstream stream error: {error}"
                            ))),
                            state,
                        ));
                    }
                    None => {
                        state.pending.extend(state.decoder.finish());
                        if state.pending.is_empty() {
                            state.failed = true;
                            return Some((
                                Err(ProviderError::Stream(
                                    "upstream stream ended before [DONE]".to_owned(),
                                )),
                                state,
                            ));
                        }
                    }
                }
            }
        },
    )
}

fn decode_chunk(data: &str) -> Result<ChatChunk, ProviderError> {
    let value: Value = serde_json::from_str(data)
        .map_err(|error| ProviderError::Stream(format!("malformed upstream frame: {error}")))?;
    if value.get("error").is_some() {
        return Err(ProviderError::Stream(
            "upstream reported a stream error".to_owned(),
        ));
    }
    let chunk: OpenAiStreamChunk = serde_json::from_value(value)
        .map_err(|error| ProviderError::Stream(format!("invalid upstream chunk: {error}")))?;
    Ok(ChatChunk::from(chunk))
}

async fn classify_error(response: reqwest::Response) -> ProviderError {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
    classify_response_status(status, retry_after, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{router, MockState};
    use futures_util::StreamExt;
    use gateway_core::FinishReason;

    const REQUEST_FIXTURE: &str =
        include_str!("../../../tests/fixtures/providers/openai/chat-request.json");

    async fn spawn_mock() -> (String, MockState) {
        let state = MockState::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock listener");
        let address = listener.local_addr().expect("mock address");
        let app = router(state.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock server");
        });
        (format!("http://{address}/v1"), state)
    }

    fn request() -> ChatRequest {
        serde_json::from_str(REQUEST_FIXTURE).expect("valid fixture")
    }

    fn provider(base_url: String) -> OpenAiProvider {
        OpenAiProvider::new(base_url, "test-key", Duration::from_secs(2))
    }

    #[tokio::test]
    async fn maps_successful_response() {
        let (base_url, state) = spawn_mock().await;
        let response = provider(base_url)
            .chat(request())
            .await
            .expect("chat succeeds");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.usage.total_tokens, 17);
        assert_eq!(state.recorded().len(), 1);
    }

    #[tokio::test]
    async fn classifies_rate_limit_with_retry_after() {
        let (base_url, _) = spawn_mock().await;
        let provider = OpenAiProvider::new(base_url, "test-key", Duration::from_secs(2));
        let mut request = request();
        request.extra.insert(
            "x-mock-scenario".to_owned(),
            Value::String("rate_limited".to_owned()),
        );
        let error = provider.chat(request).await.unwrap_err();
        assert!(matches!(error, ProviderError::RateLimited { .. }));
    }

    #[tokio::test]
    async fn classifies_server_error_as_retryable() {
        let (base_url, _) = spawn_mock().await;
        let provider = OpenAiProvider::new(base_url, "test-key", Duration::from_secs(2));
        let mut request = request();
        request.extra.insert(
            "x-mock-scenario".to_owned(),
            Value::String("server_error".to_owned()),
        );
        let error = provider.chat(request).await.unwrap_err();
        assert!(error.is_retryable());
        assert!(matches!(error, ProviderError::Upstream5xx { status: 500 }));
    }

    #[tokio::test]
    async fn classifies_bad_request_as_not_retryable() {
        let (base_url, _) = spawn_mock().await;
        let provider = OpenAiProvider::new(base_url, "test-key", Duration::from_secs(2));
        let mut request = request();
        request.extra.insert(
            "x-mock-scenario".to_owned(),
            Value::String("bad_request".to_owned()),
        );
        let error = provider.chat(request).await.unwrap_err();
        assert!(!error.is_retryable());
        assert!(matches!(
            error,
            ProviderError::BadRequest { status: 400, .. }
        ));
    }

    fn streaming_request(scenario: &str) -> ChatRequest {
        let mut request = request();
        request.extra.insert(
            "x-mock-scenario".to_owned(),
            Value::String(scenario.to_owned()),
        );
        request
    }

    #[tokio::test]
    async fn streams_chunks_until_done() {
        let (base_url, _) = spawn_mock().await;
        let stream = provider(base_url)
            .chat_stream(streaming_request("stream_ok"))
            .await
            .expect("stream");
        let chunks: Vec<ChatChunk> = stream.map(|item| item.expect("chunk")).collect().await;

        assert_eq!(chunks.len(), 3);
        let content: String = chunks
            .iter()
            .filter_map(|chunk| chunk.delta.content.as_deref())
            .collect();
        assert_eq!(content, "Hello");
        assert_eq!(chunks[0].delta.role, Some(gateway_core::Role::Assistant));
        assert_eq!(chunks[2].finish_reason, Some(FinishReason::Stop));
        assert_eq!(chunks[2].usage.expect("usage").output_tokens, 5);
    }

    #[tokio::test]
    async fn maps_malformed_stream_to_error() {
        let (base_url, _) = spawn_mock().await;
        let stream = provider(base_url)
            .chat_stream(streaming_request("stream_malformed"))
            .await
            .expect("stream");
        let items: Vec<Result<ChatChunk, ProviderError>> = stream.collect().await;

        assert!(matches!(items.last(), Some(Err(ProviderError::Stream(_)))));
    }

    #[tokio::test]
    async fn maps_cut_stream_to_error() {
        let (base_url, _) = spawn_mock().await;
        let stream = provider(base_url)
            .chat_stream(streaming_request("stream_cut"))
            .await
            .expect("stream");
        let items: Vec<Result<ChatChunk, ProviderError>> = stream.collect().await;

        assert_eq!(items.len(), 2);
        assert!(matches!(items[1], Err(ProviderError::Stream(_))));
    }

    #[tokio::test]
    async fn cancels_upstream_when_stream_is_dropped() {
        let (base_url, state) = spawn_mock().await;
        let mut stream = provider(base_url)
            .chat_stream(streaming_request("client_cancel"))
            .await
            .expect("stream");
        let first = stream.next().await.expect("first chunk").expect("valid");
        assert_eq!(first.delta.content.as_deref(), Some("He"));
        drop(stream);

        let observed = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if state.cancellations() > 0 {
                    return true;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("cancellation observed");
        assert!(observed);
    }
}
