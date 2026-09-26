use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use oasis_core::{ProposedEffect, Result};
use tokio::sync::Mutex;

use crate::traits::{Provider, ProviderRequest, ProviderResponse};

/// A scripted step for the mock provider.
#[derive(Debug, Clone)]
pub struct MockScript {
    pub assistant_message: Option<String>,
    pub effects: Vec<ProposedEffect>,
}

impl MockScript {
    pub fn complete(answer: impl Into<String>) -> Self {
        let answer = answer.into();
        Self {
            assistant_message: Some(answer.clone()),
            effects: vec![ProposedEffect::complete(answer)],
        }
    }

    pub fn tool(name: impl Into<String>, input: serde_json::Value) -> Self {
        let name = name.into();
        Self {
            assistant_message: Some(format!("calling {name}")),
            effects: vec![ProposedEffect::tool_call(name, input)],
        }
    }

    pub fn hitl(prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        Self {
            assistant_message: Some(prompt.clone()),
            effects: vec![ProposedEffect::hitl(prompt)],
        }
    }

    pub fn effects(effects: Vec<ProposedEffect>) -> Self {
        Self {
            assistant_message: None,
            effects,
        }
    }
}

/// Deterministic mock provider driven by a script queue.
#[derive(Clone)]
pub struct MockProvider {
    scripts: Arc<Mutex<Vec<MockScript>>>,
    cursor: Arc<AtomicUsize>,
    name: String,
}

impl MockProvider {
    pub fn new(scripts: Vec<MockScript>) -> Self {
        Self {
            scripts: Arc::new(Mutex::new(scripts)),
            cursor: Arc::new(AtomicUsize::new(0)),
            name: "mock".into(),
        }
    }

    pub fn echo() -> Self {
        Self::new(vec![])
    }

    pub fn steps_consumed(&self) -> usize {
        self.cursor.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse> {
        let scripts = self.scripts.lock().await;
        let idx = self.cursor.fetch_add(1, Ordering::SeqCst);
        if let Some(script) = scripts.get(idx) {
            return Ok(ProviderResponse {
                assistant_message: script.assistant_message.clone(),
                effects: script.effects.clone(),
            });
        }
        // Default: complete with an echo of the goal / last user message.
        let answer = request
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, oasis_core::Role::User))
            .map(|m| m.content.clone())
            .unwrap_or_else(|| format!("done: {}", request.goal));
        Ok(ProviderResponse {
            assistant_message: Some(answer.clone()),
            effects: vec![ProposedEffect::complete(answer)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis_core::Message;

    #[tokio::test]
    async fn mock_follows_script() {
        let provider = MockProvider::new(vec![
            MockScript::tool("echo", serde_json::json!({"text": "hi"})),
            MockScript::complete("ok"),
        ]);
        let r1 = provider
            .complete(ProviderRequest {
                goal: "g".into(),
                messages: vec![Message::user("hi")],
                available_tools: vec!["echo".into()],
            })
            .await
            .unwrap();
        assert_eq!(r1.effects[0].name, "echo");
        let r2 = provider
            .complete(ProviderRequest {
                goal: "g".into(),
                messages: vec![Message::user("hi")],
                available_tools: vec![],
            })
            .await
            .unwrap();
        assert_eq!(r2.effects[0].kind, oasis_core::EffectKind::Complete);
    }
}
