use std::sync::Arc;

use providers::Provider;

#[derive(Clone)]
pub struct ProviderRegistry {
    default: Arc<dyn Provider>,
    anthropic: Option<Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new(default: Arc<dyn Provider>, anthropic: Option<Arc<dyn Provider>>) -> Self {
        Self { default, anthropic }
    }

    pub fn resolve(&self, model: &str) -> Arc<dyn Provider> {
        if model.starts_with("claude") {
            if let Some(provider) = &self.anthropic {
                return provider.clone();
            }
        }
        self.default.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use std::time::Duration;

    use providers::{AnthropicProvider, OpenAiProvider};

    use super::*;

    fn registry() -> ProviderRegistry {
        let timeout = Duration::from_millis(1);
        let openai: Arc<dyn Provider> = Arc::new(OpenAiProvider::new(
            "http://127.0.0.1:1/v1",
            "test-key",
            timeout,
        ));
        let anthropic: Arc<dyn Provider> = Arc::new(AnthropicProvider::new(
            "http://127.0.0.1:1/v1",
            "test-key",
            timeout,
        ));
        ProviderRegistry::new(openai, Some(anthropic))
    }

    #[test]
    fn resolves_claude_models_to_anthropic() {
        assert_eq!(
            registry().resolve("claude-3-5-sonnet-latest").id(),
            "anthropic"
        );
    }

    #[test]
    fn resolves_other_models_to_default() {
        let registry = registry();
        assert_eq!(registry.resolve("gpt-4o-mini").id(), "openai");
        assert_eq!(registry.resolve("fast").id(), "openai");
    }
}
