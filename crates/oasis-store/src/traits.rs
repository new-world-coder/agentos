use oasis_core::{Effect, JournalEntry, Result, Run, RunId};
use async_trait::async_trait;

/// Durable store for runs, effects, and the effect journal.
#[async_trait]
pub trait Store: Send + Sync {
    async fn create_run(&self, run: &Run) -> Result<()>;
    async fn get_run(&self, id: &RunId) -> Result<Option<Run>>;
    async fn update_run(&self, run: &Run) -> Result<()>;
    async fn list_runs(&self) -> Result<Vec<Run>>;

    async fn put_effect(&self, effect: &Effect) -> Result<()>;
    async fn get_effect(&self, id: &str) -> Result<Option<Effect>>;
    async fn list_effects(&self, run_id: &RunId) -> Result<Vec<Effect>>;

    async fn append_journal(&self, entry: &JournalEntry) -> Result<()>;
    async fn list_journal(&self, run_id: &RunId) -> Result<Vec<JournalEntry>>;
    async fn tip_hash(&self, run_id: &RunId) -> Result<oasis_core::Hash>;
}
