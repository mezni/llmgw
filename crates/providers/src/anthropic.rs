use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures_core::stream::BoxStream;
use gateway_core::{
    ChatChunk, ChatRequest, ChatResponse, Choice, Message, OpenAiUsage, PromptTokensDetails, Role,
};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{from_reqwest_error, parse_retry_after, Provider, ProviderError};

pub const DEFAULT_MAX_TOKENS: u32 = 1024;
pub const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    timeout: std::time::Duration,
    id: String,
}

impl AnthropicProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout,
            id: "anthropic".to_owned(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    fn endpoint(&self) -> String {
        format!("{}/messages", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicRequestMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
    cache_read_input_tokens: Option<u32>,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let wire = map_request(request)?;
        let response = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&wire)
            .send()
            .await
            .map_err(from_reqwest_error)?;

        if !response.status().is_success() {
            return Err(classify_error(response).await);
        }

        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let message: AnthropicResponse = response.json().await.map_err(from_reqwest_error)?;
        Ok(map_response(message, wire.model, created))
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        Err(ProviderError::BadRequest {
            status: 400,
            body: json!({
                "message": "Anthropic streaming translation is not implemented yet"
            }),
        })
    }
}

fn map_request(request: ChatRequest) -> Result<AnthropicRequest, ProviderError> {
    if request.tools.is_some() {
        return Err(ProviderError::BadRequest {
            status: 400,
            body: json!({ "message": "tools are not supported for Anthropic yet" }),
        });
    }

    let mut system = Vec::new();
    let mut messages = Vec::new();
    for message in request.messages {
        let content = message.content.unwrap_or_default();
        match message.role {
            Role::System => system.push(content),
            Role::User => messages.push(AnthropicRequestMessage {
                role: "user",
                content,
            }),
            Role::Assistant => messages.push(AnthropicRequestMessage {
                role: "assistant",
                content,
            }),
            Role::Tool => {
                return Err(ProviderError::BadRequest {
                    status: 400,
                    body: json!({ "message": "tool results are not supported for Anthropic yet" }),
                })
            }
        }
    }

    Ok(AnthropicRequest {
        model: request.model,
        max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        messages,
        system: if system.is_empty() {
            None
        } else {
            Some(system.join("\n"))
        },
        temperature: request.temperature,
    })
}

fn map_response(message: AnthropicResponse, model: String, created: u64) -> ChatResponse {
    let content = message
        .content
        .into_iter()
        .filter_map(|block| {
            if block.content_type == "text" {
                block.text
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let finish_reason = match message.stop_reason.as_deref() {
        Some("max_tokens") => Some("length".to_owned()),
        Some("tool_use") => Some("tool_calls".to_owned()),
        Some("stop_sequence") => Some("stop".to_owned()),
        Some("end_turn") | None => Some("stop".to_owned()),
        Some(other) => Some(other.to_owned()),
    };
    let usage = OpenAiUsage {
        prompt_tokens: message.usage.input_tokens,
        completion_tokens: message.usage.output_tokens,
        total_tokens: message.usage.input_tokens + message.usage.output_tokens,
        prompt_tokens_details: message.usage.cache_read_input_tokens.map(|cached| {
            PromptTokensDetails {
                cached_tokens: cached,
            }
        }),
    };

    ChatResponse {
        id: "msg-mapped".to_owned(),
        object: "chat.completion".to_owned(),
        created,
        model,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content: Some(content),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason,
        }],
        usage,
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use axum::{Json, Router};
    use gateway_core::{ChatRequest, Message};
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;

    const RESPONSE_FIXTURE: &str =
        include_str!("../tests/fixtures/providers/anthropic/message-response.json");

    fn request() -> ChatRequest {
        ChatRequest {
            model: "claude-test-1".to_owned(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: Some("Be brief.".to_owned()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                Message {
                    role: Role::User,
                    content: Some("Say hello.".to_owned()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
            ],
            stream: false,
            max_tokens: None,
            temperature: Some(0.2),
            tools: None,
            response_format: None,
            extra: Default::default(),
        }
    }

    async fn spawn_anthropic(status: StatusCode, body: Value) -> AnthropicProvider {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("anthropic listener");
        let app = Router::new().route(
            "/v1/messages",
            post(move |headers: HeaderMap, Json(_request): Json<Value>| {
                let status = status;
                let body = body.clone();
                async move {
                    assert!(headers.contains_key("x-api-key"));
                    assert_eq!(
                        headers
                            .get("anthropic-version")
                            .and_then(|v| v.to_str().ok()),
                        Some(DEFAULT_ANTHROPIC_VERSION)
                    );
                    (status, Json(body))
                }
            }),
        );
        let address = listener.local_addr().expect("anthropic address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("anthropic server");
        });
        AnthropicProvider::new(
            format!("http://{address}/v1"),
            "test-anthropic-key",
            Duration::from_secs(2),
        )
    }

    #[tokio::test]
    async fn maps_non_streaming_response_and_system_prompt() {
        let fixture: Value = serde_json::from_str(RESPONSE_FIXTURE).expect("response fixture");
        let provider = spawn_anthropic(StatusCode::OK, fixture).await;
        let response = provider.chat(request()).await.expect("chat response");

        assert_eq!(response.object, "chat.completion");
        assert_eq!(response.model, "claude-test-1");
        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("Hello from Anthropic")
        );
        assert_eq!(response.choices[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(response.usage.prompt_tokens, 12);
        assert_eq!(response.usage.completion_tokens, 5);
        assert_eq!(
            response
                .usage
                .prompt_tokens_details
                .as_ref()
                .map(|details| details.cached_tokens),
            Some(3)
        );
    }

    #[tokio::test]
    async fn requires_max_tokens_and_rejects_tools() {
        let mapped = map_request(request()).expect("mapped request");
        assert_eq!(mapped.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(mapped.system.as_deref(), Some("Be brief."));
        assert_eq!(mapped.messages.len(), 1);
        assert_eq!(mapped.messages[0].role, "user");

        let mut with_tools = request();
        with_tools.tools = Some(vec![]);
        assert!(matches!(
            map_request(with_tools),
            Err(ProviderError::BadRequest { .. })
        ));
    }

    #[tokio::test]
    async fn classifies_rate_limit_without_echoing_body() {
        let provider = spawn_anthropic(
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "error": { "type": "rate_limit_error", "message": "slow down" } }),
        )
        .await;
        let error = provider.chat(request()).await.expect_err("rate limited");
        assert_eq!(error.kind(), "rate_limited");
    }

    #[tokio::test]
    async fn streaming_is_explicitly_unsupported() {
        let provider = spawn_anthropic(StatusCode::OK, json!({})).await;
        match provider.chat_stream(request()).await {
            Err(error) => assert_eq!(error.kind(), "bad_request"),
            Ok(_) => panic!("streaming should be unsupported"),
        }
    }
}
