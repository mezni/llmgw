pub mod anthropic;
pub mod error;
pub mod mock;
pub mod openai;
pub mod sse;

pub use anthropic::*;
pub use error::*;
pub use mock::*;
pub use openai::*;
pub use sse::*;

use async_trait::async_trait;
use futures_core::stream::BoxStream;
use gateway_core::{ChatChunk, ChatRequest, ChatResponse};

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError>;
}
