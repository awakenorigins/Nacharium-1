//! E1.4 — Resonance: the quality of impact between two entities.
//!
//! Resonance CANNOT be bought, faked, or transferred. It only accrues when
//! BOTH parties signal real value from an interaction (a mutual, two-sided
//! confirmation), scaled by how substantive the interaction was.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tracks pairwise resonance. Key is an unordered pair of DIDs.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ResonanceLedger {
    // sorted "a|b" -> resonance score
    pairs: BTreeMap<String, f64>,
}

fn key(a: &str, b: &str) -> String {
    if a <= b { format!("{a}|{b}") } else { format!("{b}|{a}") }
}

impl ResonanceLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a mutually-confirmed valuable interaction. Resonance ONLY grows
    /// when BOTH sides confirm (`a_confirms && b_confirms`). One-sided praise
    /// (attention/vanity) adds nothing.
    pub fn interact(&mut self, a: &str, b: &str, weight: f64, a_confirms: bool, b_confirms: bool) {
        if a == b || weight <= 0.0 || !(a_confirms && b_confirms) {
            return;
        }
        *self.pairs.entry(key(a, b)).or_insert(0.0) += weight;
    }

    pub fn between(&self, a: &str, b: &str) -> f64 {
        *self.pairs.get(&key(a, b)).unwrap_or(&0.0)
    }

    /// Total resonance an entity has accumulated (feeds gravity).
    pub fn total_for(&self, did: &str) -> f64 {
        let prefix = format!("{did}|");
        let suffix = format!("|{did}");
        self.pairs
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix) || k.ends_with(&suffix))
            .map(|(_, v)| *v)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonance_needs_mutual_confirmation() {
        let mut r = ResonanceLedger::new();
        r.interact("a", "b", 5.0, true, false); // one-sided -> nothing
        assert_eq!(r.between("a", "b"), 0.0);
        r.interact("a", "b", 5.0, true, true); // mutual -> accrues
        assert_eq!(r.between("a", "b"), 5.0);
    }

    #[test]
    fn resonance_is_symmetric() {
        let mut r = ResonanceLedger::new();
        r.interact("a", "b", 3.0, true, true);
        assert_eq!(r.between("a", "b"), r.between("b", "a"));
    }

    #[test]
    fn self_interaction_is_ignored() {
        let mut r = ResonanceLedger::new();
        r.interact("a", "a", 9.0, true, true);
        assert_eq!(r.between("a", "a"), 0.0);
    }

    #[test]
    fn total_aggregates_all_pairs() {
        let mut r = ResonanceLedger::new();
        r.interact("a", "b", 2.0, true, true);
        r.interact("a", "c", 3.0, true, true);
        assert_eq!(r.total_for("a"), 5.0);
    }
}
