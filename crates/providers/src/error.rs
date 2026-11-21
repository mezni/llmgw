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
