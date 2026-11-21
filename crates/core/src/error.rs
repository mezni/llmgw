use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: u16,
    pub error: ErrorBody,
    pub retry_after: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorBody {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: &'static str,
    pub code: &'static str,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error.message)
    }
}

impl ApiError {
    pub fn new(
        status: u16,
        error_type: &'static str,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            error: ErrorBody {
                message: message.into(),
                error_type,
                code,
            },
            retry_after: None,
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(400, "invalid_request_error", "invalid_request", message)
    }

    pub fn unsupported_field(field: &str) -> Self {
        Self::new(
            400,
            "invalid_request_error",
            "unsupported_field",
            format!("field '{field}' is not supported"),
        )
    }
}

const REJECTED_FIELDS: [&str; 9] = [
    "logprobs",
    "top_logprobs",
    "functions",
    "audio",
    "modalities",
    "prediction",
    "service_tier",
    "web_search_options",
    "function_call",
];

pub fn validate_request(request: &crate::ChatRequest) -> Result<(), ApiError> {
    if request.messages.is_empty() {
        return Err(ApiError::invalid_request(
            "messages must contain at least one message",
        ));
    }

    for field in REJECTED_FIELDS {
        if request.extra.contains_key(field) {
            return Err(ApiError::unsupported_field(field));
        }
    }

    if let Some(value) = request.extra.get("n") {
        let accepted = value.as_u64() == Some(1);
        if !accepted {
            return Err(ApiError::unsupported_field("n"));
        }
    }

    if request.max_tokens == Some(0) {
        return Err(ApiError::invalid_request("max_tokens must be at least 1"));
    }

    if let Some(temperature) = request.temperature {
        if !(0.0..=2.0).contains(&temperature) {
            return Err(ApiError::invalid_request(
                "temperature must be between 0 and 2",
            ));
        }
    }

    for message in &request.messages {
        match message.role {
            crate::Role::System | crate::Role::User => {
                if message.content.is_none() {
                    return Err(ApiError::invalid_request(
                        "system and user messages require content",
                    ));
                }
            }
            crate::Role::Tool => {
                if message.tool_call_id.is_none() {
                    return Err(ApiError::invalid_request(
                        "tool messages require tool_call_id",
                    ));
                }
            }
            crate::Role::Assistant => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatRequest, Message, Role};

    fn request() -> ChatRequest {
        ChatRequest {
            model: "fast".to_owned(),
            messages: vec![Message {
                role: Role::User,
                content: Some("hi".to_owned()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            }],
            stream: false,
            max_tokens: None,
            temperature: None,
            tools: None,
            response_format: None,
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn rejects_empty_messages() {
        let mut request = request();
        request.messages.clear();
        assert_eq!(
            validate_request(&request).unwrap_err().error.code,
            "invalid_request"
        );
    }

    #[test]
    fn rejects_unsupported_fields() {
        let mut request = request();
        request
            .extra
            .insert("logprobs".to_owned(), serde_json::json!(true));
        let error = validate_request(&request).unwrap_err();
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "unsupported_field");
    }

    #[test]
    fn accepts_normal_request() {
        assert!(validate_request(&request()).is_ok());
    }
}
