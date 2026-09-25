//! Policy engine: evaluate proposed effects before they are applied.

use oasis_core::{EffectKind, ProposedEffect};
use serde::{Deserialize, Serialize};

/// Decision returned by the policy engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    RequireApproval { reason: String },
}

impl PolicyDecision {
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }
}

/// Configurable allow/deny/approval policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngine {
    /// Tool names that are always denied.
    pub deny_tools: Vec<String>,
    /// Tool names that always require HITL approval.
    pub approval_tools: Vec<String>,
    /// If true, unknown tools are denied.
    pub deny_unknown_tools: bool,
    /// Known tool names (when deny_unknown_tools is set).
    pub known_tools: Vec<String>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self {
            deny_tools: vec!["shell".into(), "exec".into()],
            approval_tools: vec![],
            deny_unknown_tools: false,
            known_tools: vec!["echo".into(), "add".into()],
        }
    }
}

impl PolicyEngine {
    pub fn permissive() -> Self {
        Self {
            deny_tools: vec![],
            approval_tools: vec![],
            deny_unknown_tools: false,
            known_tools: vec![],
        }
    }

    pub fn with_known_tools(tools: Vec<String>) -> Self {
        Self {
            known_tools: tools,
            ..Self::default()
        }
    }

    /// Evaluate a proposed effect. Policy runs *before* any side effect.
    pub fn evaluate(&self, effect: &ProposedEffect) -> PolicyDecision {
        match effect.kind {
            EffectKind::Observe | EffectKind::Complete => PolicyDecision::Allow,
            EffectKind::HitlRequest => PolicyDecision::RequireApproval {
                reason: "HITL request requires human response".into(),
            },
            EffectKind::ToolCall => self.evaluate_tool(effect),
        }
    }

    fn evaluate_tool(&self, effect: &ProposedEffect) -> PolicyDecision {
        if self.deny_tools.iter().any(|t| t == &effect.name) {
            return PolicyDecision::Deny {
                reason: format!("tool '{}' is denied by policy", effect.name),
            };
        }
        if self.deny_unknown_tools && !self.known_tools.iter().any(|t| t == &effect.name) {
            return PolicyDecision::Deny {
                reason: format!("unknown tool '{}' denied by policy", effect.name),
            };
        }
        if effect.requires_approval || self.approval_tools.iter().any(|t| t == &effect.name) {
            return PolicyDecision::RequireApproval {
                reason: format!("tool '{}' requires human approval", effect.name),
            };
        }
        PolicyDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn denies_shell() {
        let policy = PolicyEngine::default();
        let e = ProposedEffect::tool_call("shell", json!({"cmd": "rm -rf /"}));
        assert!(policy.evaluate(&e).is_deny());
    }

    #[test]
    fn allows_echo() {
        let policy = PolicyEngine::default();
        let e = ProposedEffect::tool_call("echo", json!({"text": "hi"}));
        assert!(policy.evaluate(&e).is_allow());
    }

    #[test]
    fn requires_approval_when_flagged() {
        let policy = PolicyEngine::default();
        let e = ProposedEffect::tool_call_with_approval("echo", json!({"text": "hi"}));
        assert!(matches!(
            policy.evaluate(&e),
            PolicyDecision::RequireApproval { .. }
        ));
    }
}
