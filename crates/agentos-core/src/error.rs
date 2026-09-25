use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core error type shared across AgentOS crates.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("run not found: {0}")]
    RunNotFound(String),

    #[error("effect not found: {0}")]
    EffectNotFound(String),

    #[error("invalid journal chain: {0}")]
    InvalidJournalChain(String),

    #[error("policy denied: {0}")]
    PolicyDenied(String),

    #[error("awaiting human approval for effect {0}")]
    AwaitingApproval(String),

    #[error("run is not resumable from status {0}")]
    NotResumable(String),

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

impl Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CoreError {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(CoreError::Other(s))
    }
}
