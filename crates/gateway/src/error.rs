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
        ProviderError::BadRequest { status, .. } => ApiError::new(
            status,
            "invalid_request_error",
            "invalid_request",
            "upstream rejected the request",
        ),
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
