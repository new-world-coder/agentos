use oasis_core::{Message, ProposedEffect, Result};
use async_trait::async_trait;

/// Request sent to a language-model provider.
#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub goal: String,
    pub messages: Vec<Message>,
    pub available_tools: Vec<String>,
}

/// Response from a provider: optional assistant text plus proposed effects.
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub assistant_message: Option<String>,
    pub effects: Vec<ProposedEffect>,
}

/// Abstraction over chat / tool-calling model backends.
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse>;
}
