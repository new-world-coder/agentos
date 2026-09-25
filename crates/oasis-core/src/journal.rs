use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CoreError, Result};
use crate::hash::{hash_json, Hash};

/// Payload stored inside a journal entry (canonicalized for hashing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalRecord {
    pub seq: u64,
    pub run_id: String,
    pub event_type: String,
    pub payload: Value,
    pub prev_hash: Hash,
    pub at: DateTime<Utc>,
}

/// A journal entry with its content hash forming an append-only hash chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub seq: u64,
    pub run_id: String,
    pub event_type: String,
    pub payload: Value,
    pub prev_hash: Hash,
    pub hash: Hash,
    pub at: DateTime<Utc>,
}

impl JournalEntry {
    /// Create the next entry in a hash chain.
    pub fn append(
        run_id: impl Into<String>,
        seq: u64,
        event_type: impl Into<String>,
        payload: Value,
        prev_hash: Hash,
    ) -> Result<Self> {
        let at = Utc::now();
        let run_id = run_id.into();
        let event_type = event_type.into();
        let record = JournalRecord {
            seq,
            run_id: run_id.clone(),
            event_type: event_type.clone(),
            payload: payload.clone(),
            prev_hash: prev_hash.clone(),
            at,
        };
        let hash = hash_json(&record).map_err(|e| CoreError::Other(e.to_string()))?;
        Ok(Self {
            seq,
            run_id,
            event_type,
            payload,
            prev_hash,
            hash,
            at,
        })
    }

    /// Recompute and verify this entry's hash.
    pub fn verify_hash(&self) -> Result<()> {
        let record = JournalRecord {
            seq: self.seq,
            run_id: self.run_id.clone(),
            event_type: self.event_type.clone(),
            payload: self.payload.clone(),
            prev_hash: self.prev_hash.clone(),
            at: self.at,
        };
        let expected = hash_json(&record).map_err(|e| CoreError::Other(e.to_string()))?;
        if expected != self.hash {
            return Err(CoreError::InvalidJournalChain(format!(
                "entry seq {} hash mismatch",
                self.seq
            )));
        }
        Ok(())
    }
}

/// Verify an ordered chain of journal entries.
pub fn verify_chain(entries: &[JournalEntry]) -> Result<()> {
    let mut expected_prev = Hash::genesis();
    for (i, entry) in entries.iter().enumerate() {
        if entry.seq != i as u64 {
            return Err(CoreError::InvalidJournalChain(format!(
                "expected seq {i}, got {}",
                entry.seq
            )));
        }
        if entry.prev_hash != expected_prev {
            return Err(CoreError::InvalidJournalChain(format!(
                "seq {} prev_hash mismatch",
                entry.seq
            )));
        }
        entry.verify_hash()?;
        expected_prev = entry.hash.clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chain_verifies() {
        let e0 = JournalEntry::append(
            "r1",
            0,
            "run_created",
            json!({"goal": "x"}),
            Hash::genesis(),
        )
        .unwrap();
        let e1 = JournalEntry::append(
            "r1",
            1,
            "effect_proposed",
            json!({"id": "e1"}),
            e0.hash.clone(),
        )
        .unwrap();
        verify_chain(&[e0, e1]).unwrap();
    }

    #[test]
    fn tampered_payload_fails() {
        let mut e0 = JournalEntry::append(
            "r1",
            0,
            "run_created",
            json!({"goal": "x"}),
            Hash::genesis(),
        )
        .unwrap();
        e0.payload = json!({"goal": "tampered"});
        assert!(e0.verify_hash().is_err());
    }
}
