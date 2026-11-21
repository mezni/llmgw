use axum::extract::{Extension, State};
use axum::Json;
use gateway_core::{validate_request, ApiError, ChatRequest, ChatResponse};

use crate::error::{map_provider_error, ApiErrorResponse};
use crate::request_id::RequestId;
use crate::router::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiErrorResponse> {
    validate_request(&request)?;

    if request.stream {
        return Err(ApiErrorResponse(ApiError::unsupported_field("stream")));
    }

    let alias = request.model.clone();
    let mut response = state
        .provider
        .chat(request)
        .await
        .map_err(map_provider_error)?;
    response.id = request_id.0;
    response.model = alias;
    Ok(Json(response))
}
