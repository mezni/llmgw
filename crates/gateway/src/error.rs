use axum::http::{header::RETRY_AFTER, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use gateway_core::ApiError;
use providers::ProviderError;
use serde_json::json;

pub struct ApiErrorResponse(pub ApiError);

impl From<ApiError> for ApiErrorResponse {
    fn from(error: ApiError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        let error = self.0;
        let mut headers = HeaderMap::new();
        if let Some(seconds) = error.retry_after {
            if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) {
                headers.insert(RETRY_AFTER, value);
            }
        }

        let status =
            StatusCode::from_u16(error.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, headers, Json(json!({ "error": error.error }))).into_response()
    }
}

pub fn map_provider_error(error: ProviderError) -> ApiError {
    match error {
        ProviderError::BadRequest { status, .. } => match status {
            400 | 422 => ApiError::new(
                400,
                "invalid_request_error",
                "invalid_request",
                "upstream rejected the request",
            ),
            408 => ApiError::new(
                504,
                "timeout_error",
                "upstream_timeout",
                "upstream request timed out",
            ),
            413 => ApiError::new(
                413,
                "invalid_request_error",
                "request_too_large",
                "upstream rejected the request body",
            ),
            _ => ApiError::new(
                status,
                "invalid_request_error",
                "invalid_request",
                "upstream rejected the request",
            ),
        },
        ProviderError::RateLimited { retry_after } => {
            let mut api_error = ApiError::new(
                429,
                "rate_limit_error",
                "rate_limit_exceeded",
                "upstream rate limit exceeded",
            );
            api_error.retry_after = retry_after.map(|duration| duration.as_secs().max(1));
            api_error
        }
        ProviderError::Upstream5xx { .. } => ApiError::new(
            502,
            "upstream_error",
            "upstream_failed",
            "upstream request failed",
        ),
        ProviderError::Timeout => ApiError::new(
            504,
            "timeout_error",
            "upstream_timeout",
            "upstream request timed out",
        ),
        ProviderError::Network(_) => ApiError::new(
            502,
            "upstream_error",
            "upstream_failed",
            "upstream unavailable",
        ),
        ProviderError::Auth => ApiError::new(
            500,
            "internal_error",
            "internal_error",
            "upstream authentication failed",
        ),
        ProviderError::Stream(_) => ApiError::new(
            502,
            "upstream_error",
            "upstream_failed",
            "upstream stream failed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::*;

    #[test]
    fn maps_upstream_error_matrix_without_echoing_body() {
        let hostile = json!({ "error": { "message": "upstream failed with key sk-hostile-fake" } });
        let cases = [
            (
                ProviderError::BadRequest {
                    status: 400,
                    body: hostile.clone(),
                },
                400,
                "invalid_request_error",
                "invalid_request",
            ),
            (
                ProviderError::BadRequest {
                    status: 413,
                    body: hostile.clone(),
                },
                413,
                "invalid_request_error",
                "request_too_large",
            ),
            (
                ProviderError::BadRequest {
                    status: 408,
                    body: hostile.clone(),
                },
                504,
                "timeout_error",
                "upstream_timeout",
            ),
            (
                ProviderError::Upstream5xx { status: 500 },
                502,
                "upstream_error",
                "upstream_failed",
            ),
            (
                ProviderError::Timeout,
                504,
                "timeout_error",
                "upstream_timeout",
            ),
            (
                ProviderError::Auth,
                500,
                "internal_error",
                "internal_error",
            ),
            (
                ProviderError::Stream("broken".to_owned()),
                502,
                "upstream_error",
                "upstream_failed",
            ),
        ];

        for (provider_error, status, error_type, code) in cases {
            let mapped = map_provider_error(provider_error);
            assert_eq!(mapped.status, status);
            assert_eq!(mapped.error.error_type, error_type);
            assert_eq!(mapped.error.code, code);
            assert!(!mapped.error.message.contains("sk-hostile-fake"));
        }
    }

    #[test]
    fn rate_limit_carries_retry_after_header() {
        let mapped = map_provider_error(ProviderError::RateLimited {
            retry_after: Some(Duration::from_secs(2)),
        });
        assert_eq!(mapped.status, 429);
        assert_eq!(mapped.retry_after, Some(2));
    }

    #[tokio::test]
    async fn maps_network_failures_to_upstream_error() {
        let error = reqwest::Client::new()
            .get("http://127.0.0.1:1/v1/chat/completions")
            .send()
            .await
            .expect_err("closed port");
        let mapped = map_provider_error(ProviderError::Network(error));
        assert_eq!(mapped.status, 502);
        assert_eq!(mapped.error.code, "upstream_failed");
    }
}
