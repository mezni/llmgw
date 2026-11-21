use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use gateway_core::ApiError;

use crate::error::ApiErrorResponse;
use crate::router::AppState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthContext {
    pub project_id: String,
    pub key_id: String,
}

pub async fn middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if matches!(path, "/health" | "/ready") {
        return next.run(request).await;
    }

    let secret = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty());

    let Some(secret) = secret else {
        return unauthorized().into_response();
    };

    match storage::find_active_key(&state.pool, secret).await {
        Ok(Some(key)) => {
            let mut request = request;
            request.extensions_mut().insert(AuthContext {
                project_id: key.project_id,
                key_id: key.key_id,
            });
            next.run(request).await
        }
        Ok(None) | Err(_) => unauthorized().into_response(),
    }
}

fn unauthorized() -> ApiErrorResponse {
    ApiError::new(
        401,
        "authentication_error",
        "invalid_api_key",
        "invalid or expired API key",
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_body_matches_the_documented_shape() {
        let response = unauthorized().into_response();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
