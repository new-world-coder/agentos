use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::hash::Hash;

/// Unique identifier for a run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(pub String);

impl RunId {
    pub fn new() -> Self {
        RunId(Uuid::new_v4().to_string())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RunId {
    fn from(s: &str) -> Self {
        RunId(s.to_string())
    }
}

/// Lifecycle status of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    AwaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn is_resumable(self) -> bool {
        matches!(
            self,
            Self::Pending | Self::Running | Self::AwaitingApproval
        )
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Conversation role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a run transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            name: Some(name.into()),
        }
    }
}

/// A discrete step within a run (provider turn or effect application).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub index: u64,
    pub kind: String,
    pub detail: Value,
    pub at: DateTime<Utc>,
}

/// Durable run record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub status: RunStatus,
    pub goal: String,
    pub messages: Vec<Message>,
    pub steps: Vec<Step>,
    pub tip_hash: Hash,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_effect_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Run {
    pub fn new(goal: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: RunId::new(),
            status: RunStatus::Pending,
            goal: goal.into(),
            messages: Vec::new(),
            steps: Vec::new(),
            tip_hash: Hash::genesis(),
            created_at: now,
            updated_at: now,
            pending_effect_id: None,
            error: None,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
