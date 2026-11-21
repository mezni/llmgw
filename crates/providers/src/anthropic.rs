use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use axum::body::Bytes;
use futures_core::stream::BoxStream;
use futures_util::{Stream, StreamExt};
use gateway_core::{
    ChatChunk, ChatRequest, ChatResponse, Choice, Delta, FinishReason, FunctionCall,
    FunctionCallDelta, Message, OpenAiUsage, PromptTokensDetails, Role, ToolCall, ToolCallDelta,
    Usage,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::sse::{SseDecoder, SseEvent};
use crate::{
    classify_response_status, from_reqwest_error, parse_retry_after, Provider, ProviderError,
};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "is_false")]
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: Value,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn mock_scenario(request: &ChatRequest) -> Option<String> {
    request
        .extra
        .get("x-mock-scenario")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

#[derive(Debug, Serialize)]
struct AnthropicRequestMessage {
    role: &'static str,
    content: AnthropicMessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum AnthropicMessageContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
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
    id: Option<String>,
    name: Option<String>,
    input: Option<Value>,
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
        let mock_scenario = mock_scenario(&request);
        let wire = map_request(request, false)?;
        let mut builder = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&wire);
        if let Some(scenario) = mock_scenario {
            builder = builder.header("x-mock-scenario", scenario);
        }
        let response = builder.send().await.map_err(from_reqwest_error)?;

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
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        let mock_scenario = mock_scenario(&request);
        let wire = map_request(request, true)?;
        let mut builder = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&wire);
        if let Some(scenario) = mock_scenario {
            builder = builder.header("x-mock-scenario", scenario);
        }
        let response = builder.send().await.map_err(from_reqwest_error)?;

        if !response.status().is_success() {
            return Err(classify_error(response).await);
        }

        Ok(Box::pin(stream_chunks(response.bytes_stream())))
    }
}

struct AnthropicStreamState<S> {
    bytes: S,
    decoder: SseDecoder,
    pending: VecDeque<SseEvent>,
    input_tokens: u32,
    cached_tokens: Option<u32>,
    failed: bool,
    finished: bool,
}

fn stream_chunks<S>(
    bytes: S,
) -> impl Stream<Item = Result<ChatChunk, ProviderError>> + Send + 'static
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin + 'static,
{
    futures_util::stream::unfold(
        AnthropicStreamState {
            bytes,
            decoder: SseDecoder::new(),
            pending: VecDeque::new(),
            input_tokens: 0,
            cached_tokens: None,
            failed: false,
            finished: false,
        },
        |mut state| async move {
            loop {
                if state.failed || state.finished {
                    return None;
                }

                if let Some(event) = state.pending.pop_front() {
                    match decode_stream_event(&event.data, &mut state) {
                        Ok(Some(chunk)) => return Some((Ok(chunk), state)),
                        Ok(None) => continue,
                        Err(error) => {
                            state.failed = true;
                            return Some((Err(error), state));
                        }
                    }
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
                                    "anthropic stream ended before message_stop".to_owned(),
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

fn event_index(value: &Value) -> u32 {
    value
        .get("index")
        .and_then(Value::as_u64)
        .unwrap_or_default() as u32
}

fn tool_call_chunk(index: u32, id: &str, name: &str, arguments: &str) -> ChatChunk {
    ChatChunk {
        delta: Delta {
            tool_calls: Some(vec![ToolCallDelta {
                index,
                id: if id.is_empty() {
                    None
                } else {
                    Some(id.to_owned())
                },
                call_type: if id.is_empty() {
                    None
                } else {
                    Some("function".to_owned())
                },
                function: Some(FunctionCallDelta {
                    name: if name.is_empty() {
                        None
                    } else {
                        Some(name.to_owned())
                    },
                    arguments: Some(arguments.to_owned()),
                }),
            }]),
            ..Delta::default()
        },
        finish_reason: None,
        usage: None,
    }
}

fn decode_stream_event(
    data: &str,
    state: &mut AnthropicStreamState<impl Stream<Item = Result<Bytes, reqwest::Error>>>,
) -> Result<Option<ChatChunk>, ProviderError> {
    let value: Value = serde_json::from_str(data)
        .map_err(|error| ProviderError::Stream(format!("malformed upstream frame: {error}")))?;
    match value.get("type").and_then(Value::as_str) {
        Some("message_start") => {
            let usage = value
                .pointer("/message/usage")
                .and_then(Value::as_object)
                .ok_or_else(|| ProviderError::Stream("missing message_start usage".to_owned()))?;
            state.input_tokens = usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default() as u32;
            state.cached_tokens = usage
                .get("cache_read_input_tokens")
                .and_then(Value::as_u64)
                .map(|value| value as u32);
            Ok(Some(ChatChunk {
                delta: Delta {
                    role: Some(Role::Assistant),
                    ..Delta::default()
                },
                finish_reason: None,
                usage: None,
            }))
        }
        Some("content_block_start") => {
            let block = match value.get("content_block") {
                Some(block) => block,
                None => return Ok(None),
            };
            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                return Ok(None);
            }
            Ok(Some(tool_call_chunk(
                event_index(&value),
                block.get("id").and_then(Value::as_str).unwrap_or_default(),
                block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                "",
            )))
        }
        Some("content_block_delta") => {
            let delta = match value.get("delta") {
                Some(delta) => delta,
                None => return Ok(None),
            };
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => Ok(Some(ChatChunk {
                    delta: Delta {
                        content: Some(
                            delta
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned(),
                        ),
                        ..Delta::default()
                    },
                    finish_reason: None,
                    usage: None,
                })),
                Some("input_json_delta") => Ok(Some(tool_call_chunk(
                    event_index(&value),
                    "",
                    "",
                    delta
                        .pointer("/partial_json")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ))),
                _ => Ok(None),
            }
        }
        Some("message_delta") => {
            let output_tokens = value
                .pointer("/usage/output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default() as u32;
            let finish_reason = match value
                .pointer("/delta/stop_reason")
                .and_then(Value::as_str)
                .unwrap_or("end_turn")
            {
                "max_tokens" => FinishReason::Length,
                "tool_use" => FinishReason::ToolCalls,
                _ => FinishReason::Stop,
            };
            Ok(Some(ChatChunk {
                delta: Delta::default(),
                finish_reason: Some(finish_reason),
                usage: Some(Usage {
                    input_tokens: state.input_tokens,
                    output_tokens,
                    cached_tokens: state.cached_tokens,
                }),
            }))
        }
        Some("message_stop") => {
            state.finished = true;
            Ok(None)
        }
        Some("error") => Err(ProviderError::Stream(
            "upstream reported an anthropic stream error".to_owned(),
        )),
        Some("ping") | Some("content_block_stop") | None => Ok(None),
        Some(_) => Ok(None),
    }
}

fn map_request(request: ChatRequest, stream: bool) -> Result<AnthropicRequest, ProviderError> {
    let tools = match request.tools.as_deref() {
        Some(tools) => Some(
            tools
                .iter()
                .map(|tool| {
                    if tool.tool_type != "function" {
                        return Err(ProviderError::BadRequest {
                            status: 400,
                            body: json!({ "message": "only function tools are supported" }),
                        });
                    }
                    Ok(AnthropicTool {
                        name: tool.function.name.clone(),
                        description: tool.function.description.clone(),
                        input_schema: tool.function.parameters.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };

    let mut system = Vec::new();
    let mut messages = Vec::new();
    for message in request.messages {
        let content = message.content.unwrap_or_default();
        match message.role {
            Role::System => system.push(content),
            Role::User => messages.push(AnthropicRequestMessage {
                role: "user",
                content: AnthropicMessageContent::Text(content),
            }),
            Role::Assistant => {
                let mut blocks = Vec::new();
                if !content.is_empty() {
                    blocks.push(AnthropicContentBlock::Text { text: content });
                }
                if let Some(calls) = message.tool_calls {
                    for call in calls {
                        let input: Value = serde_json::from_str(&call.function.arguments)
                            .unwrap_or_else(|_| json!({}));
                        blocks.push(AnthropicContentBlock::ToolUse {
                            id: call.id,
                            name: call.function.name,
                            input,
                        });
                    }
                }
                messages.push(AnthropicRequestMessage {
                    role: "assistant",
                    content: if blocks.is_empty() {
                        AnthropicMessageContent::Text(String::new())
                    } else {
                        AnthropicMessageContent::Blocks(blocks)
                    },
                });
            }
            Role::Tool => {
                let tool_use_id =
                    message
                        .tool_call_id
                        .ok_or_else(|| ProviderError::BadRequest {
                            status: 400,
                            body: json!({ "message": "tool messages require tool_call_id" }),
                        })?;
                messages.push(AnthropicRequestMessage {
                    role: "user",
                    content: AnthropicMessageContent::Blocks(vec![
                        AnthropicContentBlock::ToolResult {
                            tool_use_id,
                            content,
                        },
                    ]),
                });
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
        tools,
        stream,
    })
}

fn map_response(message: AnthropicResponse, model: String, created: u64) -> ChatResponse {
    let mut text = Vec::new();
    let mut tool_calls = Vec::new();
    for block in message.content {
        match block.content_type.as_str() {
            "text" => {
                if let Some(value) = block.text {
                    text.push(value);
                }
            }
            "tool_use" => {
                let arguments = serde_json::to_string(&block.input.unwrap_or_else(|| json!({})))
                    .unwrap_or_else(|_| "{}".to_owned());
                tool_calls.push(ToolCall {
                    id: block.id.unwrap_or_default(),
                    call_type: "function".to_owned(),
                    function: FunctionCall {
                        name: block.name.unwrap_or_default(),
                        arguments,
                    },
                });
            }
            _ => {}
        }
    }
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
                content: Some(text.join("")),
                name: None,
                tool_call_id: None,
                tool_calls: Some(tool_calls),
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
    classify_response_status(status, retry_after, body)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use axum::{Json, Router};
    use gateway_core::{ChatRequest, FunctionDefinition, Message, Tool};
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;

    const RESPONSE_FIXTURE: &str =
        include_str!("../tests/fixtures/providers/anthropic/message-response.json");
    const STREAM_FIXTURE: &str =
        include_str!("../tests/fixtures/providers/anthropic/message-stream.sse");

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

    async fn spawn_anthropic_sse(body: &'static str) -> AnthropicProvider {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("anthropic listener");
        let app = Router::new().route(
            "/v1/messages",
            post(move || {
                let body = body;
                async move {
                    (
                        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                        body,
                    )
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

    fn fragmented_bytes(
        fixture: &str,
    ) -> impl Stream<Item = Result<Bytes, reqwest::Error>> + 'static {
        let frames = fixture
            .bytes()
            .map(|byte| Ok(Bytes::copy_from_slice(&[byte])))
            .collect::<Vec<_>>();
        futures_util::stream::iter(frames)
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
    async fn maps_default_request() {
        let mapped = map_request(request(), false).expect("mapped request");
        assert_eq!(mapped.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(mapped.system.as_deref(), Some("Be brief."));
        assert_eq!(mapped.messages.len(), 1);
        assert_eq!(mapped.messages[0].role, "user");
        assert!(mapped.tools.is_none());
    }

    #[tokio::test]
    async fn maps_tool_definitions_calls_and_results() {
        let mut request = request();
        request.tools = Some(vec![Tool {
            tool_type: "function".to_owned(),
            function: FunctionDefinition {
                name: "get_weather".to_owned(),
                description: Some("Get weather".to_owned()),
                parameters: json!({"type": "object"}),
            },
        }]);
        request.messages.push(Message {
            role: Role::Assistant,
            content: None,
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "toolu_weather".to_owned(),
                call_type: "function".to_owned(),
                function: FunctionCall {
                    name: "get_weather".to_owned(),
                    arguments: "{\"city\":\"Paris\"}".to_owned(),
                },
            }]),
        });
        request.messages.push(Message {
            role: Role::Tool,
            content: Some("18C".to_owned()),
            name: None,
            tool_call_id: Some("toolu_weather".to_owned()),
            tool_calls: None,
        });

        let mapped = map_request(request, false).expect("mapped tools");
        let tools = serde_json::to_value(&mapped.tools).expect("tools json");
        assert_eq!(tools[0]["name"], "get_weather");
        assert_eq!(tools[0]["input_schema"]["type"], "object");
        let messages = serde_json::to_value(&mapped.messages).expect("messages json");
        assert_eq!(messages[1]["content"][0]["type"], "tool_use");
        assert_eq!(messages[1]["content"][0]["id"], "toolu_weather");
        assert_eq!(messages[2]["content"][0]["type"], "tool_result");
        assert_eq!(messages[2]["content"][0]["tool_use_id"], "toolu_weather");
    }

    #[tokio::test]
    async fn maps_non_streaming_tool_use_to_openai_tool_calls() {
        let body = json!({
            "id": "msg_tool",
            "type": "message",
            "role": "assistant",
            "content": [
                { "type": "text", "text": "Checking the weather." },
                {
                    "type": "tool_use",
                    "id": "toolu_weather",
                    "name": "get_weather",
                    "input": { "city": "Paris" }
                }
            ],
            "stop_reason": "tool_use",
            "usage": { "input_tokens": 12, "output_tokens": 5 }
        });
        let provider = spawn_anthropic(StatusCode::OK, body).await;
        let response = provider.chat(request()).await.expect("tool response");
        let choice = &response.choices[0];
        let calls = choice.message.tool_calls.as_ref().expect("tool calls");
        assert_eq!(calls[0].id, "toolu_weather");
        assert_eq!(calls[0].function.name, "get_weather");
        assert_eq!(calls[0].function.arguments, "{\"city\":\"Paris\"}");
        assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(
            choice.message.content.as_deref(),
            Some("Checking the weather.")
        );
    }

    #[tokio::test]
    async fn assembles_streamed_anthropic_tool_fragments() {
        let fixture = r#"event: message_start
data: {"type":"message_start","message":{"usage":{"input_tokens":12,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_weather","name":"get_weather","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"city\":"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"Paris\"}"}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":5}}

event: message_stop
data: {"type":"message_stop"}

"#;
        let chunks = stream_chunks(fragmented_bytes(fixture))
            .collect::<Vec<_>>()
            .await;
        let chunks = chunks
            .into_iter()
            .map(|chunk| chunk.expect("chunk"))
            .collect::<Vec<_>>();
        let start = chunks[1].delta.tool_calls.as_ref().expect("tool start");
        assert_eq!(start[0].id.as_deref(), Some("toolu_weather"));
        assert_eq!(
            start[0].function.as_ref().and_then(|f| f.name.as_deref()),
            Some("get_weather")
        );
        let arguments: String = chunks[2..4]
            .iter()
            .filter_map(|chunk| chunk.delta.tool_calls.as_ref())
            .flatten()
            .filter_map(|call| call.function.as_ref())
            .filter_map(|function| function.arguments.as_deref())
            .collect();
        assert_eq!(arguments, "{\"city\":\"Paris\"}");
        assert_eq!(chunks[4].finish_reason, Some(FinishReason::ToolCalls));
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
    async fn translates_fragmented_anthropic_stream_events() {
        let chunks = stream_chunks(fragmented_bytes(STREAM_FIXTURE))
            .collect::<Vec<_>>()
            .await;
        let chunks = chunks
            .into_iter()
            .map(|chunk| chunk.expect("chunk"))
            .collect::<Vec<_>>();

        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].delta.role, Some(Role::Assistant));
        assert_eq!(chunks[1].delta.content.as_deref(), Some("He"));
        assert_eq!(chunks[2].delta.content.as_deref(), Some("llo"));
        assert_eq!(chunks[3].finish_reason, Some(FinishReason::Stop));
        let usage = chunks[3].usage.expect("final usage");
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.cached_tokens, Some(3));
    }

    #[tokio::test]
    async fn chat_stream_returns_translated_sse_chunks() {
        let provider = spawn_anthropic_sse(STREAM_FIXTURE).await;
        let stream = provider.chat_stream(request()).await.expect("stream");
        let chunks = stream.collect::<Vec<_>>().await;
        assert_eq!(chunks.len(), 4);
        assert!(chunks.iter().all(|chunk| chunk.is_ok()));
    }

    #[tokio::test]
    async fn cut_anthropic_stream_ends_with_error() {
        let cut = STREAM_FIXTURE
            .split("event: message_stop")
            .next()
            .expect("cut fixture");
        let chunks = stream_chunks(fragmented_bytes(cut))
            .collect::<Vec<_>>()
            .await;
        let last = chunks.last().expect("last item");
        assert!(last.is_err());
        assert_eq!(last.as_ref().expect_err("error").kind(), "stream");
    }

    #[tokio::test]
    async fn anthropic_stream_error_event_is_sanitized() {
        let body = "event: error\ndata: {\"type\":\"error\",\"error\":{\"message\":\"sk-hostile-fake\"}}\n\n";
        let chunks = stream_chunks(fragmented_bytes(body))
            .collect::<Vec<_>>()
            .await;
        let error = chunks[0].as_ref().expect_err("stream error");
        assert_eq!(error.kind(), "stream");
        assert!(!error.to_string().contains("sk-hostile-fake"));
    }
}
