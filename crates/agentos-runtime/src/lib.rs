//! Durable agent runtime: propose → policy → (HITL) → apply → journal.

mod engine;

pub use engine::{ApprovalDecision, Runtime, RuntimeConfig, StepOutcome};
