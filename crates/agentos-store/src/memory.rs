use std::collections::HashMap;
use std::sync::Arc;

use agentos_core::{Effect, Hash, JournalEntry, Result, Run, RunId};
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::traits::Store;

/// In-memory store for Phase 0 / unit tests.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    runs: HashMap<String, Run>,
    effects: HashMap<String, Effect>,
    journal: HashMap<String, Vec<JournalEntry>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn create_run(&self, run: &Run) -> Result<()> {
        let mut g = self.inner.write().await;
        g.runs.insert(run.id.0.clone(), run.clone());
        g.journal.insert(run.id.0.clone(), Vec::new());
        Ok(())
    }

    async fn get_run(&self, id: &RunId) -> Result<Option<Run>> {
        let g = self.inner.read().await;
        Ok(g.runs.get(&id.0).cloned())
    }

    async fn update_run(&self, run: &Run) -> Result<()> {
        let mut g = self.inner.write().await;
        if !g.runs.contains_key(&run.id.0) {
            return Err(agentos_core::CoreError::RunNotFound(run.id.0.clone()));
        }
        g.runs.insert(run.id.0.clone(), run.clone());
        Ok(())
    }

    async fn list_runs(&self) -> Result<Vec<Run>> {
        let g = self.inner.read().await;
        let mut runs: Vec<_> = g.runs.values().cloned().collect();
        runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(runs)
    }

    async fn put_effect(&self, effect: &Effect) -> Result<()> {
        let mut g = self.inner.write().await;
        g.effects.insert(effect.id.clone(), effect.clone());
        Ok(())
    }

    async fn get_effect(&self, id: &str) -> Result<Option<Effect>> {
        let g = self.inner.read().await;
        Ok(g.effects.get(id).cloned())
    }

    async fn list_effects(&self, run_id: &RunId) -> Result<Vec<Effect>> {
        let g = self.inner.read().await;
        let mut effects: Vec<_> = g
            .effects
            .values()
            .filter(|e| e.run_id == run_id.0)
            .cloned()
            .collect();
        effects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(effects)
    }

    async fn append_journal(&self, entry: &JournalEntry) -> Result<()> {
        let mut g = self.inner.write().await;
        let chain = g
            .journal
            .entry(entry.run_id.clone())
            .or_default();
        let expected_prev = chain
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(Hash::genesis);
        if entry.prev_hash != expected_prev {
            return Err(agentos_core::CoreError::InvalidJournalChain(format!(
                "prev_hash mismatch at seq {}",
                entry.seq
            )));
        }
        if entry.seq != chain.len() as u64 {
            return Err(agentos_core::CoreError::InvalidJournalChain(format!(
                "expected seq {}, got {}",
                chain.len(),
                entry.seq
            )));
        }
        entry.verify_hash()?;
        chain.push(entry.clone());
        Ok(())
    }

    async fn list_journal(&self, run_id: &RunId) -> Result<Vec<JournalEntry>> {
        let g = self.inner.read().await;
        Ok(g.journal.get(&run_id.0).cloned().unwrap_or_default())
    }

    async fn tip_hash(&self, run_id: &RunId) -> Result<Hash> {
        let g = self.inner.read().await;
        Ok(g.journal
            .get(&run_id.0)
            .and_then(|c| c.last().map(|e| e.hash.clone()))
            .unwrap_or_else(Hash::genesis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentos_core::JournalEntry;
    use serde_json::json;

    #[tokio::test]
    async fn memory_journal_chain() {
        let store = MemoryStore::new();
        let run = Run::new("test");
        store.create_run(&run).await.unwrap();
        let e0 = JournalEntry::append(&run.id.0, 0, "created", json!({}), Hash::genesis()).unwrap();
        store.append_journal(&e0).await.unwrap();
        let tip = store.tip_hash(&run.id).await.unwrap();
        assert_eq!(tip, e0.hash);
    }
}
