pub mod error;
pub mod mock;
pub mod openai;

pub use error::*;
pub use mock::*;
pub use openai::*;

use async_trait::async_trait;
use gateway_core::{ChatRequest, ChatResponse};

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
}
