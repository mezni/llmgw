use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use futures_util::Stream;
use gateway_core::{Usage, UsageEvent, UsageStatus};

use crate::auth::AuthContext;
use providers::ProviderError;
use tokio::sync::mpsc;
use tracing::warn;

pub const USAGE_CHANNEL_CAPACITY: usize = 1024;
pub const TAG_HEADER: &str = "x-gateway-tag";

#[derive(Clone)]
pub struct UsageLog {
    sender: Arc<Mutex<Option<mpsc::Sender<UsageEvent>>>>,
}

impl UsageLog {
    pub fn new(sender: mpsc::Sender<UsageEvent>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

    pub fn channel(capacity: usize) -> (Self, mpsc::Receiver<UsageEvent>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (Self::new(sender), receiver)
    }

    pub fn record(&self, event: UsageEvent) {
        let sender = self.sender.lock().expect("usage log mutex");
        if let Some(sender) = sender.as_ref() {
            if let Err(error) = sender.try_send(event) {
                warn!(error = %error, "usage event dropped");
            }
        }
    }

    pub fn close(&self) {
        self.sender.lock().expect("usage log mutex").take();
    }
}

#[derive(Clone)]
pub struct RequestUsage {
    id: String,
    project_id: String,
    key_id: String,
    alias: String,
    tag: Option<String>,
    provider_id: String,
    model: String,
    stream: bool,
    started: Instant,
}

impl RequestUsage {
    pub fn new(
        id: impl Into<String>,
        auth: AuthContext,
        alias: impl Into<String>,
        tag: Option<String>,
        provider_id: impl Into<String>,
        model: impl Into<String>,
        stream: bool,
    ) -> Self {
        Self {
            id: id.into(),
            project_id: auth.project_id,
            key_id: auth.key_id,
            alias: alias.into(),
            tag,
            provider_id: provider_id.into(),
            model: model.into(),
            stream,
            started: Instant::now(),
        }
    }

    pub fn event(
        &self,
        status: UsageStatus,
        http_status: Option<u16>,
        error_kind: Option<&str>,
        usage: Option<Usage>,
        ttft_ms: Option<i64>,
    ) -> UsageEvent {
        UsageEvent {
            id: self.id.clone(),
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            project_id: self.project_id.clone(),
            key_id: self.key_id.clone(),
            alias: self.alias.clone(),
            tag: self.tag.clone(),
            provider_id: Some(self.provider_id.clone()),
            model: Some(self.model.clone()),
            stream: self.stream,
            input_tokens: usage.map(|value| value.input_tokens),
            output_tokens: usage.map(|value| value.output_tokens),
            cached_tokens: usage.and_then(|value| value.cached_tokens),
            cost_usd: None,
            latency_ms: Some(self.started.elapsed().as_millis() as i64),
            ttft_ms,
            status,
            http_status,
            error_kind: error_kind.map(str::to_owned),
        }
    }
}

pub fn tap_stream<S>(stream: S, context: RequestUsage, log: UsageLog) -> UsageTap<S>
where
    S: Stream<Item = Result<gateway_core::ChatChunk, ProviderError>>,
{
    UsageTap {
        inner: stream,
        context,
        log,
        usage: None,
        ttft_ms: None,
        emitted: false,
    }
}

pub struct UsageTap<S> {
    inner: S,
    context: RequestUsage,
    log: UsageLog,
    usage: Option<Usage>,
    ttft_ms: Option<i64>,
    emitted: bool,
}

impl<S> Unpin for UsageTap<S> {}

impl<S> Stream for UsageTap<S>
where
    S: Stream<Item = Result<gateway_core::ChatChunk, ProviderError>> + Unpin,
{
    type Item = Result<gateway_core::ChatChunk, ProviderError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if self.ttft_ms.is_none() {
                    self.ttft_ms = Some(self.context.started.elapsed().as_millis() as i64);
                }
                if chunk.usage.is_some() {
                    self.usage = chunk.usage;
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(error))) => {
                let kind = error.kind();
                self.emit(UsageStatus::Error, Some(502), Some(kind));
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                self.emit(UsageStatus::Ok, Some(200), None);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S> UsageTap<S> {
    fn emit(&mut self, status: UsageStatus, http_status: Option<u16>, error_kind: Option<&str>) {
        if self.emitted {
            return;
        }
        self.emitted = true;
        let event = self
            .context
            .event(status, http_status, error_kind, self.usage, self.ttft_ms);
        self.log.record(event);
    }
}

impl<S> Drop for UsageTap<S> {
    fn drop(&mut self) {
        self.emit(UsageStatus::ClientCancelled, None, None);
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{stream, StreamExt};
    use gateway_core::{ChatChunk, Delta};

    use super::*;

    fn chunk(usage: Option<Usage>) -> ChatChunk {
        ChatChunk {
            delta: Delta {
                content: Some("Hi".to_owned()),
                ..Delta::default()
            },
            finish_reason: None,
            usage,
        }
    }

    fn context() -> RequestUsage {
        RequestUsage::new(
            "req-1",
            AuthContext {
                project_id: "project-1".to_owned(),
                key_id: "key-1".to_owned(),
            },
            "fast",
            None,
            "openai",
            "gpt-test",
            true,
        )
    }

    #[tokio::test]
    async fn full_channel_record_does_not_block() {
        let (log, mut receiver) = UsageLog::channel(1);
        log.record(context().event(UsageStatus::Ok, Some(200), None, None, None));
        let started = Instant::now();
        log.record(context().event(UsageStatus::Ok, Some(200), None, None, None));
        assert!(started.elapsed() < std::time::Duration::from_millis(50));
        assert!(receiver.try_recv().is_ok());
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn close_releases_the_writer_channel() {
        let (log, mut receiver) = UsageLog::channel(4);
        log.record(context().event(UsageStatus::Ok, Some(200), None, None, None));
        log.close();
        log.record(context().event(UsageStatus::Ok, Some(200), None, None, None));
        assert!(receiver.try_recv().is_ok());
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::error::TryRecvError::Disconnected)
        ));
    }

    #[tokio::test]
    async fn completed_stream_emits_one_row_with_usage_and_ttft() {
        let (log, mut receiver) = UsageLog::channel(4);
        let usage = Usage {
            input_tokens: 12,
            output_tokens: 5,
            cached_tokens: Some(2),
        };
        let stream = tap_stream(
            stream::iter(vec![Ok(chunk(None)), Ok(chunk(Some(usage)))]),
            context(),
            log,
        );
        let items: Vec<_> = stream.collect().await;
        assert_eq!(items.len(), 2);

        let event = receiver.try_recv().expect("usage event");
        assert_eq!(event.project_id, "project-1");
        assert_eq!(event.key_id, "key-1");
        assert_eq!(event.status, UsageStatus::Ok);
        assert_eq!(event.input_tokens, Some(12));
        assert_eq!(event.output_tokens, Some(5));
        assert_eq!(event.cached_tokens, Some(2));
        assert!(event.ttft_ms.is_some());
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn dropped_stream_emits_client_cancelled_once() {
        let (log, mut receiver) = UsageLog::channel(4);
        let mut stream = tap_stream(
            stream::iter(vec![Ok(chunk(None))])
                .chain(stream::pending::<Result<ChatChunk, ProviderError>>()),
            context(),
            log,
        );
        assert!(stream.next().await.is_some());
        drop(stream);

        let event = receiver.try_recv().expect("cancellation event");
        assert_eq!(event.status, UsageStatus::ClientCancelled);
        assert!(event.ttft_ms.is_some());
        assert_eq!(event.input_tokens, None);
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn stream_error_emits_error_kind() {
        let (log, mut receiver) = UsageLog::channel(4);
        let stream = tap_stream(
            stream::iter(vec![Err(ProviderError::Stream("cut".to_owned()))]),
            context(),
            log,
        );
        let items: Vec<_> = stream.collect().await;
        assert_eq!(items.len(), 1);

        let event = receiver.try_recv().expect("error event");
        assert_eq!(event.status, UsageStatus::Error);
        assert_eq!(event.error_kind.as_deref(), Some("stream"));
        assert_eq!(event.http_status, Some(502));
        assert!(receiver.try_recv().is_err());
    }
}
