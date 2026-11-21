use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageStatus {
    Ok,
    Error,
    ClientCancelled,
}

impl UsageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::ClientCancelled => "client_cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: String,
    pub ts: String,
    pub project_id: String,
    pub key_id: String,
    pub alias: String,
    pub tag: Option<String>,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub stream: bool,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cached_tokens: Option<u32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<i64>,
    pub ttft_ms: Option<i64>,
    pub status: UsageStatus,
    pub http_status: Option<u16>,
    pub error_kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(status: UsageStatus) -> UsageEvent {
        UsageEvent {
            id: "req-1".to_owned(),
            ts: "2026-11-20T12:00:00Z".to_owned(),
            project_id: "local".to_owned(),
            key_id: "local-key".to_owned(),
            alias: "fast".to_owned(),
            tag: None,
            provider_id: Some("openai".to_owned()),
            model: Some("gpt-test".to_owned()),
            stream: false,
            input_tokens: Some(12),
            output_tokens: Some(5),
            cached_tokens: None,
            cost_usd: None,
            latency_ms: Some(42),
            ttft_ms: None,
            status,
            http_status: Some(200),
            error_kind: None,
        }
    }

    #[test]
    fn status_uses_stable_log_values() {
        assert_eq!(UsageStatus::Ok.as_str(), "ok");
        assert_eq!(UsageStatus::Error.as_str(), "error");
        assert_eq!(UsageStatus::ClientCancelled.as_str(), "client_cancelled");
    }

    #[test]
    fn event_round_trips_without_prompt_or_response_bodies() {
        let serialized = serde_json::to_string(&event(UsageStatus::Ok)).expect("serialize event");
        assert!(!serialized.contains("request_body"));
        assert!(!serialized.contains("response_body"));
        let decoded: UsageEvent = serde_json::from_str(&serialized).expect("deserialize event");
        assert_eq!(decoded, event(UsageStatus::Ok));
        assert_eq!(decoded.status.as_str(), "ok");
    }
}
