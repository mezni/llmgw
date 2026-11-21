use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Extension, State};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::stream::{self, Stream};
use futures_util::StreamExt;
use gateway_core::{validate_request, ChatChunk, ChatRequest, OpenAiStreamChunk, StreamChoice};
use providers::ProviderError;
use serde_json::json;

use crate::error::{map_provider_error, ApiErrorResponse};
use crate::request_id::RequestId;
use crate::router::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Json(request): Json<ChatRequest>,
) -> Result<Response, ApiErrorResponse> {
    validate_request(&request)?;

    let alias = request.model.clone();
    if request.stream {
        return stream_response(&state, request_id.0, alias, request).await;
    }

    let mut response = state
        .provider
        .chat(request)
        .await
        .map_err(map_provider_error)?;
    response.id = request_id.0;
    response.model = alias;
    Ok(Json(response).into_response())
}

async fn stream_response(
    state: &AppState,
    id: String,
    model: String,
    request: ChatRequest,
) -> Result<Response, ApiErrorResponse> {
    let provider_stream = state
        .provider
        .chat_stream(request)
        .await
        .map_err(map_provider_error)?;
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok(Sse::new(sse_events(provider_stream, id, model, created)).into_response())
}

fn sse_events<S>(
    stream: S,
    id: String,
    model: String,
    created: u64,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static
where
    S: Stream<Item = Result<ChatChunk, ProviderError>> + Send + Unpin + 'static,
{
    stream::unfold(
        (stream, id, model, created, false, false),
        |(mut stream, id, model, created, mut failed, finished)| async move {
            if finished {
                return None;
            }

            if let Some(item) = stream.next().await {
                return match item {
                    Ok(ChatChunk {
                        delta,
                        finish_reason,
                        usage,
                    }) => {
                        let wire = OpenAiStreamChunk {
                            id: id.clone(),
                            object: "chat.completion.chunk".to_owned(),
                            created,
                            model: model.clone(),
                            choices: vec![StreamChoice {
                                index: 0,
                                delta,
                                finish_reason,
                            }],
                            usage: usage.map(Into::into),
                        };
                        let data = serde_json::to_string(&wire).unwrap_or_else(|_| "{}".to_owned());
                        Some((
                            Ok(Event::default().data(data)),
                            (stream, id, model, created, failed, false),
                        ))
                    }
                    Err(error) => {
                        failed = true;
                        let data = json!({ "error": map_provider_error(error).error }).to_string();
                        Some((
                            Ok(Event::default().data(data)),
                            (stream, id, model, created, failed, false),
                        ))
                    }
                };
            }

            if failed {
                return None;
            }
            Some((
                Ok(Event::default().data("[DONE]")),
                (stream, id, model, created, failed, true),
            ))
        },
    )
}
