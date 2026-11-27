//! Deterministic model for UI and agent-loop tests. No network, no flakiness.

use crate::model::{ChatRequest, Delta, ModelError, ModelProvider, ProviderStream};
use futures_util::{StreamExt, stream};
use std::future::Future;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

pub struct MockProvider {
    script: Vec<Delta>,
}

impl MockProvider {
    pub fn new(script: Vec<Delta>) -> Self {
        Self { script }
    }

    /// A tiny provider that echoes the last user message. Useful for smoke
    /// tests and for `provider.kind = "mock"` in configuration.
    pub fn echo() -> Self {
        Self::new(vec![])
    }
}

impl ModelProvider for MockProvider {
    fn stream_chat(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderStream, ModelError>> + Send + '_>> {
        let script: Vec<Delta> = if self.script.is_empty() {
            request
                .messages
                .last()
                .map(|m| vec![Delta::Text(format!("echo: {}", m.content))])
                .unwrap_or_default()
        } else {
            self.script.clone()
        };

        Box::pin(async move {
            let stream = stream::iter(script.into_iter().map(Ok)).take_while(move |_| {
                let token = cancel.clone();
                async move { !token.is_cancelled() }
            });
            Ok(Box::pin(stream) as ProviderStream)
        })
    }
}
