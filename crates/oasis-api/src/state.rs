use std::path::PathBuf;
use std::sync::Arc;

use oasis_policy::PolicyEngine;
use oasis_provider::MockProvider;
use oasis_runtime::{Runtime, RuntimeConfig};
use oasis_store::{MemoryStore, SqliteStore, Store};
use oasis_tools::ToolRegistry;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<Runtime>,
}

/// Build a runtime with SQLite (if path given) or in-memory store, plus mock provider.
pub fn build_runtime(db_path: Option<PathBuf>) -> anyhow::Result<Runtime> {
    let store: Arc<dyn Store> = match db_path {
        Some(path) => Arc::new(SqliteStore::open(path)?),
        None => Arc::new(MemoryStore::new()),
    };
    let tools = ToolRegistry::with_builtins();
    let policy = PolicyEngine::with_known_tools(tools.names());
    let provider = Arc::new(MockProvider::echo());
    Ok(Runtime::new(
        store,
        provider,
        tools,
        policy,
        RuntimeConfig::default(),
    ))
}
