use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: OpenAiUsage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    pub cached_tokens: u32,
}

impl From<Usage> for OpenAiUsage {
    fn from(value: Usage) -> Self {
        Self {
            prompt_tokens: value.input_tokens,
            completion_tokens: value.output_tokens,
            total_tokens: value.input_tokens + value.output_tokens,
            prompt_tokens_details: value.cached_tokens.map(|cached| PromptTokensDetails {
                cached_tokens: cached,
            }),
        }
    }
}

impl From<&OpenAiUsage> for Usage {
    fn from(value: &OpenAiUsage) -> Self {
        Self {
            input_tokens: value.prompt_tokens,
            output_tokens: value.completion_tokens,
            cached_tokens: value
                .prompt_tokens_details
                .as_ref()
                .map(|details| details.cached_tokens),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallDelta {
    pub index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChunk {
    pub delta: Delta,
    pub finish_reason: Option<FinishReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAiStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAiUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<FinishReason>,
}

impl From<OpenAiStreamChunk> for ChatChunk {
    fn from(value: OpenAiStreamChunk) -> Self {
        let usage = value.usage.as_ref().map(Usage::from);
        let choice = value.choices.into_iter().next();
        Self {
            delta: choice
                .as_ref()
                .map(|choice| choice.delta.clone())
                .unwrap_or_default(),
            finish_reason: choice.and_then(|choice| choice.finish_reason),
            usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST_FIXTURE: &str =
        include_str!("../../../tests/fixtures/providers/openai/chat-request.json");
    const RESPONSE_FIXTURE: &str =
        include_str!("../../../tests/fixtures/providers/openai/chat-response.json");
    const CHUNKS_FIXTURE: &str =
        include_str!("../../../tests/fixtures/providers/openai/chat-chunks.json");

    #[test]
    fn round_trips_openai_request_fixture() {
        let request: ChatRequest = serde_json::from_str(REQUEST_FIXTURE).expect("valid fixture");
        assert_eq!(request.model, "fast");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.extra.get("top_p"), Some(&Value::from(0.9)));

        let encoded = serde_json::to_string(&request).expect("serializable");
        let decoded: ChatRequest = serde_json::from_str(&encoded).expect("round trip");
        assert_eq!(request, decoded);
    }

    #[test]
    fn round_trips_openai_response_fixture() {
        let response: ChatResponse = serde_json::from_str(RESPONSE_FIXTURE).expect("valid fixture");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.role, Role::Assistant);
        let usage = Usage::from(&response.usage);
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.cached_tokens, Some(2));
    }

    #[test]
    fn rejects_multimodal_content() {
        let json = r#"{"model":"fast","messages":[{"role":"user","content":[{"type":"text","text":"hi"}]}]}"#;
        assert!(serde_json::from_str::<ChatRequest>(json).is_err());
    }

    #[test]
    fn converts_openai_stream_chunks_to_normalized_chunks() {
        let chunks: Vec<OpenAiStreamChunk> =
            serde_json::from_str(CHUNKS_FIXTURE).expect("valid chunk fixture");
        let normalized: Vec<ChatChunk> = chunks.into_iter().map(ChatChunk::from).collect();

        assert_eq!(
            normalized[0].delta,
            Delta {
                role: Some(Role::Assistant),
                content: Some("He".to_owned()),
                tool_calls: None,
            }
        );
        assert_eq!(normalized[0].finish_reason, None);
        assert_eq!(normalized[2].finish_reason, Some(FinishReason::Stop));
        assert_eq!(normalized[2].usage.expect("usage").input_tokens, 12);
    }

    #[test]
    fn round_trips_openai_stream_chunk() {
        let chunks: Vec<OpenAiStreamChunk> =
            serde_json::from_str(CHUNKS_FIXTURE).expect("valid chunk fixture");
        let chunk = chunks.into_iter().next().expect("first chunk");
        let encoded = serde_json::to_string(&chunk).expect("serializable");
        let decoded: OpenAiStreamChunk = serde_json::from_str(&encoded).expect("round trip");
        assert_eq!(chunk, decoded);
    }
}
