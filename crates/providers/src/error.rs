use std::time::Duration;

use reqwest::header::{HeaderMap, RETRY_AFTER};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("upstream rate limited")]
    RateLimited { retry_after: Option<Duration> },
    #[error("upstream returned status {status}")]
    Upstream5xx { status: u16 },
    #[error("upstream timeout")]
    Timeout,
    #[error("upstream network failure")]
    Network(#[source] reqwest::Error),
    #[error("upstream rejected the request with status {status}")]
    BadRequest { status: u16, body: Value },
    #[error("upstream authentication failed")]
    Auth,
    #[error("upstream stream failure: {0}")]
    Stream(String),
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::Upstream5xx { .. } | Self::Timeout | Self::Network(_)
        )
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::RateLimited { .. } => "rate_limited",
            Self::Upstream5xx { .. } => "upstream_5xx",
            Self::Timeout => "timeout",
            Self::Network(_) => "network",
            Self::BadRequest { .. } => "bad_request",
            Self::Auth => "auth",
            Self::Stream(_) => "stream",
        }
    }
}

pub fn from_reqwest_error(error: reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Network(error)
    }
}

pub fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

pub fn classify_response_status(
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    body: Value,
) -> ProviderError {
    match status {
        reqwest::StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        reqwest::StatusCode::REQUEST_TIMEOUT => ProviderError::Timeout,
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => ProviderError::Auth,
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

    use reqwest::StatusCode;
    use serde_json::json;

    use super::*;

    #[test]
    fn classifies_upstream_status_matrix() {
        let body = json!({ "error": { "message": "upstream failed with key sk-hostile-fake" } });
        assert!(matches!(
            classify_response_status(StatusCode::BAD_REQUEST, None, body.clone()),
            ProviderError::BadRequest { status: 400, .. }
        ));
        assert!(matches!(
            classify_response_status(StatusCode::PAYLOAD_TOO_LARGE, None, body.clone()),
            ProviderError::BadRequest { status: 413, .. }
        ));
        assert!(matches!(
            classify_response_status(StatusCode::REQUEST_TIMEOUT, None, body.clone()),
            ProviderError::Timeout
        ));
        assert!(matches!(
            classify_response_status(StatusCode::TOO_MANY_REQUESTS, None, body.clone()),
            ProviderError::RateLimited { .. }
        ));
        assert!(matches!(
            classify_response_status(StatusCode::UNAUTHORIZED, None, body.clone()),
            ProviderError::Auth
        ));
        assert!(matches!(
            classify_response_status(StatusCode::FORBIDDEN, None, body.clone()),
            ProviderError::Auth
        ));
        assert!(matches!(
            classify_response_status(StatusCode::INTERNAL_SERVER_ERROR, None, body.clone()),
            ProviderError::Upstream5xx { status: 500 }
        ));
    }

    #[test]
    fn retry_after_header_is_parsed_as_seconds() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, "2".parse().expect("header value"));
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(2)));
    }
}
