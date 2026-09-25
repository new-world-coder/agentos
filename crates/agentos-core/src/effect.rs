use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Kind of side-effect an agent may propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    /// Emit a message / observation (no external side effect).
    Observe,
    /// Invoke a registered tool.
    ToolCall,
    /// Request human-in-the-loop approval before continuing.
    HitlRequest,
    /// Mark the run complete with a final answer.
    Complete,
}

impl EffectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::ToolCall => "tool_call",
            Self::HitlRequest => "hitl_request",
            Self::Complete => "complete",
        }
    }

    /// Effects that mutate the outside world and require policy checks.
    pub fn requires_policy(self) -> bool {
        matches!(self, Self::ToolCall)
    }

    /// Effects that pause the run for human input.
    pub fn requires_hitl(self) -> bool {
        matches!(self, Self::HitlRequest)
    }
}

/// Lifecycle of a proposed/applied effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStatus {
    Proposed,
    PolicyAllowed,
    PolicyDenied,
    AwaitingApproval,
    Approved,
    Rejected,
    Applied,
    Failed,
}

impl EffectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::PolicyAllowed => "policy_allowed",
            Self::PolicyDenied => "policy_denied",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Applied => "applied",
            Self::Failed => "failed",
        }
    }
}

/// An effect proposed by the provider before policy / HITL / application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposedEffect {
    pub id: String,
    pub kind: EffectKind,
    pub name: String,
    pub input: Value,
    #[serde(default)]
    pub requires_approval: bool,
}

impl ProposedEffect {
    pub fn observe(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: EffectKind::Observe,
            name: "observe".into(),
            input: serde_json::json!({ "content": content.into() }),
            requires_approval: false,
        }
    }

    pub fn tool_call(name: impl Into<String>, input: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: EffectKind::ToolCall,
            name: name.into(),
            input,
            requires_approval: false,
        }
    }

    pub fn tool_call_with_approval(name: impl Into<String>, input: Value) -> Self {
        let mut e = Self::tool_call(name, input);
        e.requires_approval = true;
        e
    }

    pub fn hitl(prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: EffectKind::HitlRequest,
            name: "hitl".into(),
            input: serde_json::json!({ "prompt": prompt.into() }),
            requires_approval: true,
        }
    }

    pub fn complete(answer: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: EffectKind::Complete,
            name: "complete".into(),
            input: serde_json::json!({ "answer": answer.into() }),
            requires_approval: false,
        }
    }
}

/// A recorded effect with status and optional result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    pub run_id: String,
    pub kind: EffectKind,
    pub name: String,
    pub input: Value,
    pub status: EffectStatus,
    pub requires_approval: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Effect {
    pub fn from_proposed(run_id: impl Into<String>, proposed: ProposedEffect) -> Self {
        let now = Utc::now();
        Self {
            id: proposed.id,
            run_id: run_id.into(),
            kind: proposed.kind,
            name: proposed.name,
            input: proposed.input,
            status: EffectStatus::Proposed,
            requires_approval: proposed.requires_approval,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
