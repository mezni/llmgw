use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

pub async fn middleware(mut request: Request, next: Next) -> Response {
    let id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_valid_request_id(value))
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    request.extensions_mut().insert(RequestId(id.clone()));
    let span = tracing::info_span!("http_request", request_id = %id);
    let mut response = next.run(request).instrument(span).await;

    tracing::info!(
        request_id = %id,
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        "request completed"
    );

    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}

fn is_valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}
