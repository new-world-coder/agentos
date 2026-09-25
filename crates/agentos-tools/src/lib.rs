//! Tool trait, registry, and built-in tools.

use std::collections::HashMap;
use std::sync::Arc;

use agentos_core::{CoreError, Result};
use async_trait::async_trait;
use serde_json::Value;

/// A callable tool the runtime can invoke after policy approval.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn invoke(&self, input: Value) -> Result<Value>;
}

/// Registry of named tools.
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(EchoTool));
        reg.register(Arc::new(AddTool));
        reg
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    pub async fn invoke(&self, name: &str, input: Value) -> Result<Value> {
        let tool = self
            .get(name)
            .ok_or_else(|| CoreError::Tool(format!("unknown tool: {name}")))?;
        tool.invoke(input).await
    }
}

/// Echoes input text back.
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo text back to the caller"
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(serde_json::json!({ "echo": text }))
    }
}

/// Adds two numbers.
pub struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> &str {
        "Add two numbers: {a, b}"
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let a = input
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| CoreError::Tool("missing numeric field a".into()))?;
        let b = input
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| CoreError::Tool("missing numeric field b".into()))?;
        Ok(serde_json::json!({ "sum": a + b }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_works() {
        let reg = ToolRegistry::with_builtins();
        let out = reg
            .invoke("echo", serde_json::json!({"text": "hi"}))
            .await
            .unwrap();
        assert_eq!(out["echo"], "hi");
    }
}
