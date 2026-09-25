use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Hex-encoded SHA-256 digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub String);

impl Hash {
    pub fn genesis() -> Self {
        Hash("0".repeat(64))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hash raw bytes with SHA-256.
pub fn hash_bytes(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    Hash(hex::encode(hasher.finalize()))
}

/// Canonical JSON hash (stable field order via serde_json Value).
pub fn hash_json<T: Serialize>(value: &T) -> Result<Hash, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(hash_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        let a = hash_bytes(b"oasis");
        let b = hash_bytes(b"oasis");
        assert_eq!(a, b);
        assert_eq!(a.0.len(), 64);
    }

    #[test]
    fn genesis_is_zeros() {
        assert_eq!(Hash::genesis().0.len(), 64);
        assert!(Hash::genesis().0.chars().all(|c| c == '0'));
    }
}
