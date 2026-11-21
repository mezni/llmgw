use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::stream::{self, Stream};
use futures_util::StreamExt;
use gateway_core::{
    validate_request, ChatChunk, ChatRequest, OpenAiStreamChunk, StreamChoice, Usage, UsageStatus,
};
use providers::{Provider, ProviderError};
use serde_json::json;

use crate::error::{map_provider_error, ApiErrorResponse};
use crate::request_id::RequestId;
use crate::router::AppState;
use crate::usage::{tap_stream, RequestUsage, TAG_HEADER};

pub async fn handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<Response, ApiErrorResponse> {
    let alias = request.model.clone();
    let provider = state.providers.resolve(&alias);
    let tag = headers
        .get(TAG_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let context = RequestUsage::new(
        request_id.0.clone(),
        alias.clone(),
        tag,
        provider.id().to_owned(),
        alias.clone(),
        request.stream,
    );

    if let Err(error) = validate_request(&request) {
        state.usage.record(context.event(
            UsageStatus::Error,
            Some(error.status),
            Some(error.error.code),
            None,
            None,
        ));
        return Err(error.into());
    }

    if request.stream {
        return stream_response(&state, provider, request_id.0, alias, context, request).await;
    }

    let mut response = match provider.chat(request).await {
        Ok(response) => response,
        Err(error) => {
            let api_error = map_provider_error(error);
            state.usage.record(context.event(
                UsageStatus::Error,
                Some(api_error.status),
                Some(api_error.error.code),
                None,
                None,
            ));
            return Err(api_error.into());
        }
    };
    response.id = request_id.0;
    response.model = alias;
    let usage = Some(Usage::from(&response.usage));
    state
        .usage
        .record(context.event(UsageStatus::Ok, Some(200), None, usage, None));
    Ok(Json(response).into_response())
}

async fn stream_response(
    state: &AppState,
    provider: Arc<dyn Provider>,
    id: String,
    model: String,
    context: RequestUsage,
    request: ChatRequest,
) -> Result<Response, ApiErrorResponse> {
    let provider_stream = match provider.chat_stream(request).await {
        Ok(stream) => stream,
        Err(error) => {
            let api_error = map_provider_error(error);
            state.usage.record(context.event(
                UsageStatus::Error,
                Some(api_error.status),
                Some(api_error.error.code),
                None,
                None,
            ));
            return Err(api_error.into());
        }
    };
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let tapped = tap_stream(provider_stream, context, state.usage.clone());
    Ok(Sse::new(sse_events(tapped, id, model, created)).into_response())
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
