use std::sync::Arc;

use agentos_core::{
    CoreError, Effect, EffectKind, EffectStatus, Hash, JournalEntry, Message, Result, Role, Run,
    RunId, RunStatus, Step,
};
use agentos_policy::{PolicyDecision, PolicyEngine};
use agentos_provider::{Provider, ProviderRequest};
use agentos_store::Store;
use agentos_tools::ToolRegistry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

/// Configuration knobs for the runtime loop.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_steps: u32,
    pub system_prompt: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_steps: 32,
            system_prompt: "You are AgentOS, a durable agent. Propose effects carefully.".into(),
        }
    }
}

/// Human approval decision for a pending effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Reject { reason: String },
}

/// Outcome of driving the runtime until pause or completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Completed,
    AwaitingApproval { effect_id: String },
    Failed { error: String },
}

/// The durable agent runtime.
pub struct Runtime {
    store: Arc<dyn Store>,
    provider: Arc<dyn Provider>,
    tools: ToolRegistry,
    policy: PolicyEngine,
    config: RuntimeConfig,
}

impl Runtime {
    pub fn new(
        store: Arc<dyn Store>,
        provider: Arc<dyn Provider>,
        tools: ToolRegistry,
        policy: PolicyEngine,
        config: RuntimeConfig,
    ) -> Self {
        Self {
            store,
            provider,
            tools,
            policy,
            config,
        }
    }

    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    /// Create a new run and journal the creation event.
    pub async fn create_run(&self, goal: impl Into<String>) -> Result<Run> {
        let goal = goal.into();
        let mut run = Run::new(goal.clone());
        run.messages.push(Message::system(&self.config.system_prompt));
        run.messages.push(Message::user(&goal));
        self.store.create_run(&run).await?;
        let entry = JournalEntry::append(
            &run.id.0,
            0,
            "run_created",
            json!({ "goal": goal, "status": run.status.as_str() }),
            Hash::genesis(),
        )?;
        self.store.append_journal(&entry).await?;
        run.tip_hash = entry.hash.clone();
        run.touch();
        self.store.update_run(&run).await?;
        info!(run_id = %run.id, "run created");
        Ok(run)
    }

    /// Drive a run until it completes, fails, or awaits approval.
    pub async fn drive(&self, run_id: &RunId) -> Result<StepOutcome> {
        let mut steps = 0u32;
        loop {
            if steps >= self.config.max_steps {
                return self
                    .fail_run(run_id, "max steps exceeded")
                    .await
                    .map(|_| StepOutcome::Failed {
                        error: "max steps exceeded".into(),
                    });
            }
            steps += 1;

            let mut run = self
                .store
                .get_run(run_id)
                .await?
                .ok_or_else(|| CoreError::RunNotFound(run_id.0.clone()))?;

            if run.status.is_terminal() {
                return Ok(match run.status {
                    RunStatus::Completed => StepOutcome::Completed,
                    RunStatus::Failed => StepOutcome::Failed {
                        error: run.error.unwrap_or_else(|| "failed".into()),
                    },
                    RunStatus::Cancelled => StepOutcome::Failed {
                        error: "cancelled".into(),
                    },
                    _ => unreachable!(),
                });
            }

            if run.status == RunStatus::AwaitingApproval {
                let effect_id = run
                    .pending_effect_id
                    .clone()
                    .ok_or_else(|| CoreError::InvalidState("awaiting approval without effect".into()))?;
                return Ok(StepOutcome::AwaitingApproval { effect_id });
            }

            // Resume mid-flight: if there is an approved pending effect, apply it first.
            if let Some(effect_id) = run.pending_effect_id.clone() {
                if let Some(effect) = self.store.get_effect(&effect_id).await? {
                    if matches!(
                        effect.status,
                        EffectStatus::Approved | EffectStatus::PolicyAllowed
                    ) {
                        self.apply_effect(&mut run, effect).await?;
                        continue;
                    }
                }
            }

            run.status = RunStatus::Running;
            run.touch();
            self.store.update_run(&run).await?;

            let response = self
                .provider
                .complete(ProviderRequest {
                    goal: run.goal.clone(),
                    messages: run.messages.clone(),
                    available_tools: self.tools.names(),
                })
                .await?;

            if let Some(text) = response.assistant_message {
                run.messages.push(Message::assistant(text));
            }

            if response.effects.is_empty() {
                return self
                    .fail_run(run_id, "provider returned no effects")
                    .await
                    .map(|_| StepOutcome::Failed {
                        error: "provider returned no effects".into(),
                    });
            }

            // Process one effect at a time for durable checkpoints.
            let proposed = response.effects.into_iter().next().unwrap();
            let outcome = self.handle_proposed(&mut run, proposed).await?;
            match outcome {
                Some(o) => return Ok(o),
                None => continue,
            }
        }
    }

    /// Resume a run after a crash or process restart.
    pub async fn resume(&self, run_id: &RunId) -> Result<StepOutcome> {
        let run = self
            .store
            .get_run(run_id)
            .await?
            .ok_or_else(|| CoreError::RunNotFound(run_id.0.clone()))?;
        if !run.status.is_resumable() {
            return Err(CoreError::NotResumable(run.status.to_string()));
        }
        // Verify journal integrity before continuing.
        let journal = self.store.list_journal(run_id).await?;
        agentos_core::verify_chain(&journal)?;
        info!(run_id = %run_id, status = %run.status, "resuming run");
        self.journal(
            &run,
            "run_resumed",
            json!({ "status": run.status.as_str(), "tip": run.tip_hash.as_str() }),
        )
        .await?;
        self.drive(run_id).await
    }

    /// Resolve a pending HITL / approval gate.
    pub async fn resolve_approval(
        &self,
        run_id: &RunId,
        effect_id: &str,
        decision: ApprovalDecision,
    ) -> Result<StepOutcome> {
        let mut run = self
            .store
            .get_run(run_id)
            .await?
            .ok_or_else(|| CoreError::RunNotFound(run_id.0.clone()))?;
        if run.status != RunStatus::AwaitingApproval {
            return Err(CoreError::InvalidState(format!(
                "run is {}, not awaiting_approval",
                run.status
            )));
        }
        if run.pending_effect_id.as_deref() != Some(effect_id) {
            return Err(CoreError::InvalidState(
                "effect_id does not match pending effect".into(),
            ));
        }
        let mut effect = self
            .store
            .get_effect(effect_id)
            .await?
            .ok_or_else(|| CoreError::EffectNotFound(effect_id.into()))?;

        match decision {
            ApprovalDecision::Approve => {
                effect.status = EffectStatus::Approved;
                effect.touch();
                self.store.put_effect(&effect).await?;
                self.journal(
                    &run,
                    "effect_approved",
                    json!({ "effect_id": effect_id }),
                )
                .await?;
                run.status = RunStatus::Running;
                run.touch();
                self.store.update_run(&run).await?;
                self.apply_effect(&mut run, effect).await?;
                self.drive(run_id).await
            }
            ApprovalDecision::Reject { reason } => {
                effect.status = EffectStatus::Rejected;
                effect.error = Some(reason.clone());
                effect.touch();
                self.store.put_effect(&effect).await?;
                self.journal(
                    &run,
                    "effect_rejected",
                    json!({ "effect_id": effect_id, "reason": reason }),
                )
                .await?;
                run.pending_effect_id = None;
                self.store.update_run(&run).await?;
                self.fail_run(run_id, format!("effect rejected: {reason}"))
                    .await?;
                Ok(StepOutcome::Failed {
                    error: format!("effect rejected: {reason}"),
                })
            }
        }
    }

    async fn handle_proposed(
        &self,
        run: &mut Run,
        proposed: agentos_core::ProposedEffect,
    ) -> Result<Option<StepOutcome>> {
        let mut effect = Effect::from_proposed(&run.id.0, proposed.clone());
        self.store.put_effect(&effect).await?;
        self.journal(
            run,
            "effect_proposed",
            json!({
                "effect_id": effect.id,
                "kind": effect.kind.as_str(),
                "name": effect.name,
            }),
        )
        .await?;

        // Policy-before-effect: never apply until policy allows.
        let decision = self.policy.evaluate(&proposed);
        match decision {
            PolicyDecision::Deny { reason } => {
                effect.status = EffectStatus::PolicyDenied;
                effect.error = Some(reason.clone());
                effect.touch();
                self.store.put_effect(&effect).await?;
                self.journal(
                    run,
                    "effect_policy_denied",
                    json!({ "effect_id": effect.id, "reason": reason }),
                )
                .await?;
                self.fail_run(&run.id, format!("policy denied: {reason}"))
                    .await?;
                return Ok(Some(StepOutcome::Failed {
                    error: format!("policy denied: {reason}"),
                }));
            }
            PolicyDecision::RequireApproval { reason } => {
                effect.status = EffectStatus::AwaitingApproval;
                effect.touch();
                self.store.put_effect(&effect).await?;
                run.status = RunStatus::AwaitingApproval;
                run.pending_effect_id = Some(effect.id.clone());
                run.touch();
                self.store.update_run(run).await?;
                self.journal(
                    run,
                    "effect_awaiting_approval",
                    json!({ "effect_id": effect.id, "reason": reason }),
                )
                .await?;
                return Ok(Some(StepOutcome::AwaitingApproval {
                    effect_id: effect.id,
                }));
            }
            PolicyDecision::Allow => {
                effect.status = EffectStatus::PolicyAllowed;
                effect.touch();
                self.store.put_effect(&effect).await?;
                self.journal(
                    run,
                    "effect_policy_allowed",
                    json!({ "effect_id": effect.id }),
                )
                .await?;
            }
        }

        self.apply_effect(run, effect).await?;
        Ok(None)
    }

    async fn apply_effect(&self, run: &mut Run, mut effect: Effect) -> Result<()> {
        info!(run_id = %run.id, effect_id = %effect.id, kind = %effect.kind.as_str(), "applying effect");
        let result = match effect.kind {
            EffectKind::Observe => {
                let content = effect
                    .input
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                run.messages.push(Message::assistant(content));
                Ok(json!({ "observed": true }))
            }
            EffectKind::ToolCall => {
                match self.tools.invoke(&effect.name, effect.input.clone()).await {
                    Ok(value) => {
                        run.messages.push(Message::tool(
                            &effect.id,
                            &effect.name,
                            value.to_string(),
                        ));
                        Ok(value)
                    }
                    Err(e) => Err(e),
                }
            }
            EffectKind::HitlRequest => {
                // HITL is resolved via resolve_approval; applying means human approved continuation.
                let note = effect
                    .input
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("approved");
                run.messages.push(Message {
                    role: Role::User,
                    content: format!("[HITL approved] {note}"),
                    tool_call_id: None,
                    name: Some("human".into()),
                });
                Ok(json!({ "hitl": "approved" }))
            }
            EffectKind::Complete => {
                let answer = effect
                    .input
                    .get("answer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                run.messages.push(Message::assistant(&answer));
                run.status = RunStatus::Completed;
                run.pending_effect_id = None;
                run.error = None;
                effect.status = EffectStatus::Applied;
                effect.result = Some(json!({ "answer": answer }));
                effect.touch();
                self.store.put_effect(&effect).await?;
                self.journal(
                    run,
                    "effect_applied",
                    json!({ "effect_id": effect.id, "kind": "complete" }),
                )
                .await?;
                run.steps.push(Step {
                    index: run.steps.len() as u64,
                    kind: "complete".into(),
                    detail: json!({ "effect_id": effect.id }),
                    at: Utc::now(),
                });
                run.touch();
                self.store.update_run(run).await?;
                self.journal(run, "run_completed", json!({ "answer": answer }))
                    .await?;
                return Ok(());
            }
        };

        match result {
            Ok(value) => {
                effect.status = EffectStatus::Applied;
                effect.result = Some(value);
                effect.touch();
                self.store.put_effect(&effect).await?;
                run.pending_effect_id = None;
                run.steps.push(Step {
                    index: run.steps.len() as u64,
                    kind: effect.kind.as_str().into(),
                    detail: json!({ "effect_id": effect.id, "name": effect.name }),
                    at: Utc::now(),
                });
                run.touch();
                self.store.update_run(run).await?;
                self.journal(
                    run,
                    "effect_applied",
                    json!({ "effect_id": effect.id, "name": effect.name }),
                )
                .await?;
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "effect application failed");
                effect.status = EffectStatus::Failed;
                effect.error = Some(e.to_string());
                effect.touch();
                self.store.put_effect(&effect).await?;
                self.journal(
                    run,
                    "effect_failed",
                    json!({ "effect_id": effect.id, "error": e.to_string() }),
                )
                .await?;
                self.fail_run(&run.id, e.to_string()).await?;
                Err(e)
            }
        }
    }

    async fn fail_run(&self, run_id: &RunId, error: impl Into<String>) -> Result<()> {
        let error = error.into();
        let mut run = self
            .store
            .get_run(run_id)
            .await?
            .ok_or_else(|| CoreError::RunNotFound(run_id.0.clone()))?;
        run.status = RunStatus::Failed;
        run.error = Some(error.clone());
        run.pending_effect_id = None;
        run.touch();
        self.store.update_run(&run).await?;
        let _ = self
            .journal(&run, "run_failed", json!({ "error": error }))
            .await;
        Ok(())
    }

    async fn journal(&self, run: &Run, event_type: &str, payload: serde_json::Value) -> Result<JournalEntry> {
        // Re-read tip to stay consistent under concurrent writers (single-writer for now).
        let tip = self.store.tip_hash(&run.id).await?;
        let journal = self.store.list_journal(&run.id).await?;
        let seq = journal.len() as u64;
        let entry = JournalEntry::append(&run.id.0, seq, event_type, payload, tip)?;
        self.store.append_journal(&entry).await?;
        // Keep tip_hash on run in sync.
        if let Some(mut fresh) = self.store.get_run(&run.id).await? {
            fresh.tip_hash = entry.hash.clone();
            fresh.touch();
            self.store.update_run(&fresh).await?;
        }
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentos_provider::{MockProvider, MockScript};
    use agentos_store::{MemoryStore, SqliteStore};

    fn test_runtime(provider: MockProvider, store: Arc<dyn Store>) -> Runtime {
        let tools = ToolRegistry::with_builtins();
        let policy = PolicyEngine::with_known_tools(tools.names());
        Runtime::new(
            store,
            Arc::new(provider),
            tools,
            policy,
            RuntimeConfig::default(),
        )
    }

    #[tokio::test]
    async fn completes_with_mock_provider() {
        let store = Arc::new(MemoryStore::new());
        let rt = test_runtime(MockProvider::echo(), store);
        let run = rt.create_run("say hello").await.unwrap();
        let outcome = rt.drive(&run.id).await.unwrap();
        assert_eq!(outcome, StepOutcome::Completed);
        let run = rt.store().get_run(&run.id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        let journal = rt.store().list_journal(&run.id).await.unwrap();
        agentos_core::verify_chain(&journal).unwrap();
        assert!(journal.iter().any(|e| e.event_type == "run_completed"));
    }

    #[tokio::test]
    async fn tool_call_then_complete() {
        let store = Arc::new(MemoryStore::new());
        let provider = MockProvider::new(vec![
            MockScript::tool("add", json!({"a": 2, "b": 3})),
            MockScript::complete("5"),
        ]);
        let rt = test_runtime(provider, store);
        let run = rt.create_run("add 2+3").await.unwrap();
        let outcome = rt.drive(&run.id).await.unwrap();
        assert_eq!(outcome, StepOutcome::Completed);
    }

    #[tokio::test]
    async fn policy_denies_shell() {
        let store = Arc::new(MemoryStore::new());
        let provider = MockProvider::new(vec![MockScript::tool(
            "shell",
            json!({"cmd": "echo pwned"}),
        )]);
        let rt = test_runtime(provider, store);
        let run = rt.create_run("hack").await.unwrap();
        let outcome = rt.drive(&run.id).await.unwrap();
        assert!(matches!(outcome, StepOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn hitl_pauses_then_resumes() {
        let store = Arc::new(MemoryStore::new());
        let provider = MockProvider::new(vec![
            MockScript::hitl("approve send?"),
            MockScript::complete("sent"),
        ]);
        let rt = test_runtime(provider, store);
        let run = rt.create_run("send email").await.unwrap();
        let outcome = rt.drive(&run.id).await.unwrap();
        let effect_id = match outcome {
            StepOutcome::AwaitingApproval { effect_id } => effect_id,
            other => panic!("expected awaiting approval, got {other:?}"),
        };
        let outcome = rt
            .resolve_approval(&run.id, &effect_id, ApprovalDecision::Approve)
            .await
            .unwrap();
        assert_eq!(outcome, StepOutcome::Completed);
    }

    #[tokio::test]
    async fn hitl_survives_sqlite_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hitl.db");

        let (run_id, effect_id) = {
            let store = Arc::new(SqliteStore::open(&path).unwrap());
            let provider = MockProvider::new(vec![MockScript::hitl("continue?")]);
            let rt = test_runtime(provider, store);
            let run = rt.create_run("survive").await.unwrap();
            let outcome = rt.drive(&run.id).await.unwrap();
            let effect_id = match outcome {
                StepOutcome::AwaitingApproval { effect_id } => effect_id,
                other => panic!("expected awaiting approval, got {other:?}"),
            };
            (run.id, effect_id)
        };

        let store = Arc::new(SqliteStore::open(&path).unwrap());
        let run = store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::AwaitingApproval);

        let provider = MockProvider::new(vec![MockScript::complete("resumed-ok")]);
        let rt = test_runtime(provider, store);
        let outcome = rt
            .resolve_approval(&run_id, &effect_id, ApprovalDecision::Approve)
            .await
            .unwrap();
        assert_eq!(outcome, StepOutcome::Completed);
        let journal = rt.store().list_journal(&run_id).await.unwrap();
        agentos_core::verify_chain(&journal).unwrap();
        assert!(journal.iter().any(|e| e.event_type == "run_completed"));
    }

    #[tokio::test]
    async fn sqlite_crash_resume_mid_running() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mid.db");

        let run_id = {
            let store = Arc::new(SqliteStore::open(&path).unwrap());
            let provider = MockProvider::new(vec![MockScript::tool(
                "echo",
                json!({"text": "step1"}),
            )]);
            // After tool, mock defaults to complete — so we need to stop before second turn.
            // Approach: manually craft a Running run after first effect via drive with
            // a custom provider that only returns the tool once, then we kill before drive continues
            // by using max_steps... Actually drive loops until complete. So:
            // Create run, append journal, set status=Running with messages as if mid-flight,
            // then resume with completing provider.
            let rt = test_runtime(provider, store.clone());
            let mut run = rt.create_run("mid-flight").await.unwrap();
            // Simulate that a tool was applied and we crashed before the next provider call.
            run.status = RunStatus::Running;
            run.messages.push(Message::assistant("calling echo"));
            run.messages
                .push(Message::tool("e1", "echo", r#"{"echo":"step1"}"#));
            run.touch();
            store.update_run(&run).await.unwrap();
            let entry = {
                let tip = store.tip_hash(&run.id).await.unwrap();
                let seq = store.list_journal(&run.id).await.unwrap().len() as u64;
                JournalEntry::append(
                    &run.id.0,
                    seq,
                    "effect_applied",
                    json!({"effect_id": "e1", "name": "echo", "simulated_crash": true}),
                    tip,
                )
                .unwrap()
            };
            store.append_journal(&entry).await.unwrap();
            run.tip_hash = entry.hash;
            store.update_run(&run).await.unwrap();
            run.id
        };

        // "Crash": drop everything. Reopen and resume with completing provider.
        let store = Arc::new(SqliteStore::open(&path).unwrap());
        let provider = MockProvider::new(vec![MockScript::complete("all good after crash")]);
        let rt = test_runtime(provider, store);
        let outcome = rt.resume(&run_id).await.unwrap();
        assert_eq!(outcome, StepOutcome::Completed);
        let run = rt.store().get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        let journal = rt.store().list_journal(&run_id).await.unwrap();
        agentos_core::verify_chain(&journal).unwrap();
        assert!(journal.iter().any(|e| e.event_type == "run_resumed"));
        assert!(journal.iter().any(|e| e.event_type == "run_completed"));
    }
}
