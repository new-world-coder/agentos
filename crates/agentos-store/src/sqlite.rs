use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agentos_core::{
    CoreError, Effect, EffectKind, EffectStatus, Hash, JournalEntry, Result, Run, RunId, RunStatus,
};
use async_trait::async_trait;
use rusqlite::{params, Connection};

use crate::traits::Store;

/// SQLite-backed durable store (Phase 1).
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::Store(e.to_string()))?;
        }
        let conn = Connection::open(&path).map_err(|e| CoreError::Store(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| CoreError::Store(e.to_string()))?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| CoreError::Store(e.to_string()))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| CoreError::Store(e.to_string()))?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                goal TEXT NOT NULL,
                messages_json TEXT NOT NULL,
                steps_json TEXT NOT NULL,
                tip_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                pending_effect_id TEXT,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS effects (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                input_json TEXT NOT NULL,
                status TEXT NOT NULL,
                requires_approval INTEGER NOT NULL,
                result_json TEXT,
                error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(run_id) REFERENCES runs(id)
            );

            CREATE TABLE IF NOT EXISTS journal (
                run_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                hash TEXT NOT NULL,
                at TEXT NOT NULL,
                PRIMARY KEY (run_id, seq),
                FOREIGN KEY(run_id) REFERENCES runs(id)
            );

            CREATE INDEX IF NOT EXISTS idx_effects_run ON effects(run_id);
            CREATE INDEX IF NOT EXISTS idx_journal_run ON journal(run_id);
            "#,
        )
        .map_err(|e| CoreError::Store(e.to_string()))?;
        Ok(())
    }
}

fn status_from_str(s: &str) -> Result<RunStatus> {
    match s {
        "pending" => Ok(RunStatus::Pending),
        "running" => Ok(RunStatus::Running),
        "awaiting_approval" => Ok(RunStatus::AwaitingApproval),
        "completed" => Ok(RunStatus::Completed),
        "failed" => Ok(RunStatus::Failed),
        "cancelled" => Ok(RunStatus::Cancelled),
        other => Err(CoreError::Store(format!("unknown run status: {other}"))),
    }
}

fn effect_kind_from_str(s: &str) -> Result<EffectKind> {
    match s {
        "observe" => Ok(EffectKind::Observe),
        "tool_call" => Ok(EffectKind::ToolCall),
        "hitl_request" => Ok(EffectKind::HitlRequest),
        "complete" => Ok(EffectKind::Complete),
        other => Err(CoreError::Store(format!("unknown effect kind: {other}"))),
    }
}

fn effect_status_from_str(s: &str) -> Result<EffectStatus> {
    match s {
        "proposed" => Ok(EffectStatus::Proposed),
        "policy_allowed" => Ok(EffectStatus::PolicyAllowed),
        "policy_denied" => Ok(EffectStatus::PolicyDenied),
        "awaiting_approval" => Ok(EffectStatus::AwaitingApproval),
        "approved" => Ok(EffectStatus::Approved),
        "rejected" => Ok(EffectStatus::Rejected),
        "applied" => Ok(EffectStatus::Applied),
        "failed" => Ok(EffectStatus::Failed),
        other => Err(CoreError::Store(format!("unknown effect status: {other}"))),
    }
}

#[async_trait]
impl Store for SqliteStore {
    async fn create_run(&self, run: &Run) -> Result<()> {
        let conn = self.conn.clone();
        let run = run.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            conn.execute(
                r#"INSERT INTO runs
                (id, status, goal, messages_json, steps_json, tip_hash, created_at, updated_at, pending_effect_id, error)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
                params![
                    run.id.0,
                    run.status.as_str(),
                    run.goal,
                    serde_json::to_string(&run.messages).map_err(|e| CoreError::Store(e.to_string()))?,
                    serde_json::to_string(&run.steps).map_err(|e| CoreError::Store(e.to_string()))?,
                    run.tip_hash.as_str(),
                    run.created_at.to_rfc3339(),
                    run.updated_at.to_rfc3339(),
                    run.pending_effect_id,
                    run.error,
                ],
            )
            .map_err(|e| CoreError::Store(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn get_run(&self, id: &RunId) -> Result<Option<Run>> {
        let conn = self.conn.clone();
        let id = id.0.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let mut stmt = conn
                .prepare(
                    r#"SELECT id, status, goal, messages_json, steps_json, tip_hash,
                       created_at, updated_at, pending_effect_id, error
                       FROM runs WHERE id = ?1"#,
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let mut rows = stmt
                .query(params![id])
                .map_err(|e| CoreError::Store(e.to_string()))?;
            if let Some(row) = rows.next().map_err(|e| CoreError::Store(e.to_string()))? {
                Ok(Some(row_to_run(row)?))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn update_run(&self, run: &Run) -> Result<()> {
        let conn = self.conn.clone();
        let run = run.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let n = conn
                .execute(
                    r#"UPDATE runs SET
                    status = ?2, goal = ?3, messages_json = ?4, steps_json = ?5,
                    tip_hash = ?6, updated_at = ?7, pending_effect_id = ?8, error = ?9
                    WHERE id = ?1"#,
                    params![
                        run.id.0,
                        run.status.as_str(),
                        run.goal,
                        serde_json::to_string(&run.messages).map_err(|e| CoreError::Store(e.to_string()))?,
                        serde_json::to_string(&run.steps).map_err(|e| CoreError::Store(e.to_string()))?,
                        run.tip_hash.as_str(),
                        run.updated_at.to_rfc3339(),
                        run.pending_effect_id,
                        run.error,
                    ],
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            if n == 0 {
                return Err(CoreError::RunNotFound(run.id.0));
            }
            Ok(())
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn list_runs(&self) -> Result<Vec<Run>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let mut stmt = conn
                .prepare(
                    r#"SELECT id, status, goal, messages_json, steps_json, tip_hash,
                       created_at, updated_at, pending_effect_id, error
                       FROM runs ORDER BY created_at DESC"#,
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| Ok(row_to_run(row)))
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| CoreError::Store(e.to_string()))??);
            }
            Ok(out)
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn put_effect(&self, effect: &Effect) -> Result<()> {
        let conn = self.conn.clone();
        let effect = effect.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            conn.execute(
                r#"INSERT INTO effects
                (id, run_id, kind, name, input_json, status, requires_approval, result_json, error, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                  status = excluded.status,
                  result_json = excluded.result_json,
                  error = excluded.error,
                  updated_at = excluded.updated_at"#,
                params![
                    effect.id,
                    effect.run_id,
                    effect.kind.as_str(),
                    effect.name,
                    serde_json::to_string(&effect.input).map_err(|e| CoreError::Store(e.to_string()))?,
                    effect.status.as_str(),
                    effect.requires_approval as i64,
                    effect
                        .result
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|e| CoreError::Store(e.to_string()))?,
                    effect.error,
                    effect.created_at.to_rfc3339(),
                    effect.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| CoreError::Store(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn get_effect(&self, id: &str) -> Result<Option<Effect>> {
        let conn = self.conn.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let mut stmt = conn
                .prepare(
                    r#"SELECT id, run_id, kind, name, input_json, status, requires_approval,
                       result_json, error, created_at, updated_at
                       FROM effects WHERE id = ?1"#,
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let mut rows = stmt
                .query(params![id])
                .map_err(|e| CoreError::Store(e.to_string()))?;
            if let Some(row) = rows.next().map_err(|e| CoreError::Store(e.to_string()))? {
                Ok(Some(row_to_effect(row)?))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn list_effects(&self, run_id: &RunId) -> Result<Vec<Effect>> {
        let conn = self.conn.clone();
        let run_id = run_id.0.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let mut stmt = conn
                .prepare(
                    r#"SELECT id, run_id, kind, name, input_json, status, requires_approval,
                       result_json, error, created_at, updated_at
                       FROM effects WHERE run_id = ?1 ORDER BY created_at ASC"#,
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let rows = stmt
                .query_map(params![run_id], |row| Ok(row_to_effect(row)))
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| CoreError::Store(e.to_string()))??);
            }
            Ok(out)
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn append_journal(&self, entry: &JournalEntry) -> Result<()> {
        let conn = self.conn.clone();
        let entry = entry.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let tip: Option<String> = conn
                .query_row(
                    "SELECT hash FROM journal WHERE run_id = ?1 ORDER BY seq DESC LIMIT 1",
                    params![entry.run_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let expected_prev = tip.unwrap_or_else(|| Hash::genesis().0);
            if entry.prev_hash.as_str() != expected_prev {
                return Err(CoreError::InvalidJournalChain(format!(
                    "prev_hash mismatch at seq {}",
                    entry.seq
                )));
            }
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM journal WHERE run_id = ?1",
                    params![entry.run_id],
                    |row| row.get(0),
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            if entry.seq != count as u64 {
                return Err(CoreError::InvalidJournalChain(format!(
                    "expected seq {count}, got {}",
                    entry.seq
                )));
            }
            entry.verify_hash()?;
            conn.execute(
                r#"INSERT INTO journal (run_id, seq, event_type, payload_json, prev_hash, hash, at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                params![
                    entry.run_id,
                    entry.seq as i64,
                    entry.event_type,
                    serde_json::to_string(&entry.payload).map_err(|e| CoreError::Store(e.to_string()))?,
                    entry.prev_hash.as_str(),
                    entry.hash.as_str(),
                    entry.at.to_rfc3339(),
                ],
            )
            .map_err(|e| CoreError::Store(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn list_journal(&self, run_id: &RunId) -> Result<Vec<JournalEntry>> {
        let conn = self.conn.clone();
        let run_id = run_id.0.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let mut stmt = conn
                .prepare(
                    r#"SELECT run_id, seq, event_type, payload_json, prev_hash, hash, at
                       FROM journal WHERE run_id = ?1 ORDER BY seq ASC"#,
                )
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let rows = stmt
                .query_map(params![run_id], |row| Ok(row_to_journal(row)))
                .map_err(|e| CoreError::Store(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| CoreError::Store(e.to_string()))??);
            }
            Ok(out)
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    async fn tip_hash(&self, run_id: &RunId) -> Result<Hash> {
        let conn = self.conn.clone();
        let run_id = run_id.0.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| CoreError::Store(e.to_string()))?;
            let tip: Option<String> = conn
                .query_row(
                    "SELECT hash FROM journal WHERE run_id = ?1 ORDER BY seq DESC LIMIT 1",
                    params![run_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| CoreError::Store(e.to_string()))?;
            Ok(tip.map(Hash).unwrap_or_else(Hash::genesis))
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }
}

trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

fn row_to_run(row: &rusqlite::Row<'_>) -> Result<Run> {
    let status: String = row.get(1).map_err(|e| CoreError::Store(e.to_string()))?;
    let messages_json: String = row.get(3).map_err(|e| CoreError::Store(e.to_string()))?;
    let steps_json: String = row.get(4).map_err(|e| CoreError::Store(e.to_string()))?;
    let tip: String = row.get(5).map_err(|e| CoreError::Store(e.to_string()))?;
    let created: String = row.get(6).map_err(|e| CoreError::Store(e.to_string()))?;
    let updated: String = row.get(7).map_err(|e| CoreError::Store(e.to_string()))?;
    Ok(Run {
        id: RunId(row.get(0).map_err(|e| CoreError::Store(e.to_string()))?),
        status: status_from_str(&status)?,
        goal: row.get(2).map_err(|e| CoreError::Store(e.to_string()))?,
        messages: serde_json::from_str(&messages_json).map_err(|e| CoreError::Store(e.to_string()))?,
        steps: serde_json::from_str(&steps_json).map_err(|e| CoreError::Store(e.to_string()))?,
        tip_hash: Hash(tip),
        created_at: chrono::DateTime::parse_from_rfc3339(&created)
            .map_err(|e| CoreError::Store(e.to_string()))?
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated)
            .map_err(|e| CoreError::Store(e.to_string()))?
            .with_timezone(&chrono::Utc),
        pending_effect_id: row.get(8).map_err(|e| CoreError::Store(e.to_string()))?,
        error: row.get(9).map_err(|e| CoreError::Store(e.to_string()))?,
    })
}

fn row_to_effect(row: &rusqlite::Row<'_>) -> Result<Effect> {
    let kind: String = row.get(2).map_err(|e| CoreError::Store(e.to_string()))?;
    let status: String = row.get(5).map_err(|e| CoreError::Store(e.to_string()))?;
    let input_json: String = row.get(4).map_err(|e| CoreError::Store(e.to_string()))?;
    let result_json: Option<String> = row.get(7).map_err(|e| CoreError::Store(e.to_string()))?;
    let created: String = row.get(9).map_err(|e| CoreError::Store(e.to_string()))?;
    let updated: String = row.get(10).map_err(|e| CoreError::Store(e.to_string()))?;
    let requires: i64 = row.get(6).map_err(|e| CoreError::Store(e.to_string()))?;
    Ok(Effect {
        id: row.get(0).map_err(|e| CoreError::Store(e.to_string()))?,
        run_id: row.get(1).map_err(|e| CoreError::Store(e.to_string()))?,
        kind: effect_kind_from_str(&kind)?,
        name: row.get(3).map_err(|e| CoreError::Store(e.to_string()))?,
        input: serde_json::from_str(&input_json).map_err(|e| CoreError::Store(e.to_string()))?,
        status: effect_status_from_str(&status)?,
        requires_approval: requires != 0,
        result: result_json
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| CoreError::Store(e.to_string()))?,
        error: row.get(8).map_err(|e| CoreError::Store(e.to_string()))?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created)
            .map_err(|e| CoreError::Store(e.to_string()))?
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated)
            .map_err(|e| CoreError::Store(e.to_string()))?
            .with_timezone(&chrono::Utc),
    })
}

fn row_to_journal(row: &rusqlite::Row<'_>) -> Result<JournalEntry> {
    let payload_json: String = row.get(3).map_err(|e| CoreError::Store(e.to_string()))?;
    let at: String = row.get(6).map_err(|e| CoreError::Store(e.to_string()))?;
    let seq: i64 = row.get(1).map_err(|e| CoreError::Store(e.to_string()))?;
    Ok(JournalEntry {
        run_id: row.get(0).map_err(|e| CoreError::Store(e.to_string()))?,
        seq: seq as u64,
        event_type: row.get(2).map_err(|e| CoreError::Store(e.to_string()))?,
        payload: serde_json::from_str(&payload_json).map_err(|e| CoreError::Store(e.to_string()))?,
        prev_hash: Hash(row.get(4).map_err(|e| CoreError::Store(e.to_string()))?),
        hash: Hash(row.get(5).map_err(|e| CoreError::Store(e.to_string()))?),
        at: chrono::DateTime::parse_from_rfc3339(&at)
            .map_err(|e| CoreError::Store(e.to_string()))?
            .with_timezone(&chrono::Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentos_core::verify_chain;
    use serde_json::json;

    #[tokio::test]
    async fn sqlite_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agentos.db");
        let run_id;
        {
            let store = SqliteStore::open(&path).unwrap();
            let run = Run::new("persist me");
            run_id = run.id.clone();
            store.create_run(&run).await.unwrap();
            let e0 =
                JournalEntry::append(&run.id.0, 0, "run_created", json!({"goal": run.goal}), Hash::genesis())
                    .unwrap();
            store.append_journal(&e0).await.unwrap();
            let mut run = store.get_run(&run_id).await.unwrap().unwrap();
            run.status = RunStatus::Running;
            run.tip_hash = e0.hash.clone();
            store.update_run(&run).await.unwrap();
        }
        let store = SqliteStore::open(&path).unwrap();
        let run = store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(run.goal, "persist me");
        let journal = store.list_journal(&run_id).await.unwrap();
        assert_eq!(journal.len(), 1);
        verify_chain(&journal).unwrap();
    }
}
