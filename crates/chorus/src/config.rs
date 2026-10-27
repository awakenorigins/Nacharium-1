//! E2.5 — System config governance apply.
//!
//! The System = immutable protocol constants + GOVERNED config. This module
//! is the ONLY writer of governed config: a Chorus proposal whose harmonic
//! point passes quorum+threshold produces a new config version. Versions are
//! HASH-CHAINED (each carries the previous version's hash) so history is
//! tamper-evident and every node can verify the same lineage.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single governed config version. Hash-chained to its predecessor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigVersion {
    pub version: u64,
    pub values: BTreeMap<String, i64>,
    pub prev_hash: String,
    pub hash: String,
}

impl ConfigVersion {
    /// The genesis config version (no predecessor).
    pub fn genesis(values: BTreeMap<String, i64>) -> Self {
        let mut v = ConfigVersion { version: 0, values, prev_hash: "0".into(), hash: String::new() };
        v.hash = v.compute_hash();
        v
    }

    fn compute_hash(&self) -> String {
        let mut h = blake3::Hasher::new();
        h.update(&self.version.to_le_bytes());
        for (k, val) in &self.values {
            h.update(k.as_bytes());
            h.update(&val.to_le_bytes());
        }
        h.update(self.prev_hash.as_bytes());
        h.finalize().to_hex().to_string()
    }

    /// Apply a governed change to one key, producing the next chained version.
    /// This is the only path that mutates config — callers gate it behind a
    /// passed Chorus proposal (quorum + threshold), never an admin write.
    pub fn apply_change(&self, key: &str, value: i64) -> ConfigVersion {
        let mut values = self.values.clone();
        values.insert(key.to_string(), value);
        let mut next = ConfigVersion {
            version: self.version + 1,
            values,
            prev_hash: self.hash.clone(),
            hash: String::new(),
        };
        next.hash = next.compute_hash();
        next
    }

    /// Verify the chain link to a claimed predecessor.
    pub fn verifies_after(&self, prev: &ConfigVersion) -> bool {
        self.prev_hash == prev.hash
            && self.version == prev.version + 1
            && self.hash == self.compute_hash()
    }
}

/// Guard: a proposal may apply only if its harmonic point meets threshold AND
/// enough voices (quorum) participated. Mirrors the "only the tally writes
/// config" rule.
pub fn proposal_passes(harmonic_point: Option<u8>, threshold: u8, voices: usize, quorum: usize) -> bool {
    matches!(harmonic_point, Some(hp) if hp >= threshold) && voices >= quorum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn genesis() -> ConfigVersion {
        let mut m = BTreeMap::new();
        m.insert("economy.transfer_fee_bps".to_string(), 10);
        ConfigVersion::genesis(m)
    }

    #[test]
    fn genesis_hashes_and_chains() {
        let g = genesis();
        assert_eq!(g.version, 0);
        assert!(!g.hash.is_empty());
    }

    #[test]
    fn apply_change_chains_and_verifies() {
        let g = genesis();
        let v1 = g.apply_change("economy.transfer_fee_bps", 25);
        assert_eq!(v1.version, 1);
        assert_eq!(*v1.values.get("economy.transfer_fee_bps").unwrap(), 25);
        assert!(v1.verifies_after(&g));
    }

    #[test]
    fn broken_chain_does_not_verify() {
        let g = genesis();
        let v1 = g.apply_change("k", 5);
        let mut forged = v1.clone();
        forged.prev_hash = "tampered".into();
        assert!(!forged.verifies_after(&g));
    }

    #[test]
    fn proposal_needs_threshold_and_quorum() {
        assert!(proposal_passes(Some(70), 60, 10, 5));      // passes
        assert!(!proposal_passes(Some(50), 60, 10, 5));     // below threshold
        assert!(!proposal_passes(Some(70), 60, 3, 5));      // below quorum
        assert!(!proposal_passes(None, 60, 10, 5));         // no harmonic point
    }
}
