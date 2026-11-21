use std::time::Duration;

use async_trait::async_trait;
use gateway_core::{ChatRequest, ChatResponse};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

use crate::{from_reqwest_error, Provider, ProviderError};

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
}

async fn classify_error(response: reqwest::Response) -> ProviderError {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    let body: Value = response.json().await.unwrap_or_else(|_| json!({}));

    match status {
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        StatusCode::BAD_REQUEST => ProviderError::BadRequest {
            status: status.as_u16(),
            body,
        },
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::Auth,
        status if status.is_server_error() => ProviderError::Upstream5xx {
            status: status.as_u16(),
        },
        status => ProviderError::BadRequest {
            status: status.as_u16(),
            body,
        },
    }
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{router, MockState};

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
}
