//! E3.2 — Nachatizen DNA + Spark.
//!
//! The Spark is the "birth" of a Nachatizen — four kinds: Birth (human),
//! Awake (AI), Forged (hybrid), Remix (from Ancestral Memory). Each mints a
//! unique Spark token.
//!
//! DNA encodes traits from ACTUAL BEHAVIOR (not declarations). Private by
//! default; selective reveal. "DNA does not say what you ARE — it says how you
//! BEHAVE." DNA compatibility helps match collaborators.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SparkKind {
    Birth,
    Awake,
    Forged,
    Remix,
}

/// A unique Spark token minted at birth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spark {
    pub kind: SparkKind,
    pub token: String,
}

impl Spark {
    pub fn mint(kind: SparkKind, veil_did: &str) -> Self {
        let tag = match kind {
            SparkKind::Birth => "birth",
            SparkKind::Awake => "awake",
            SparkKind::Forged => "forged",
            SparkKind::Remix => "remix",
        };
        let token = blake3::hash(format!("spark:{tag}:{veil_did}").as_bytes()).to_hex().to_string();
        Self { kind, token }
    }
}

/// Behavioral DNA: named traits in [0,1], derived from real actions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dna {
    traits: BTreeMap<String, f64>,
}

impl Dna {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update a trait from observed behavior (EMA-style, clamped to [0,1]).
    pub fn observe(&mut self, trait_name: &str, signal: f64) {
        let s = signal.clamp(0.0, 1.0);
        let e = self.traits.entry(trait_name.to_string()).or_insert(0.5);
        *e = *e * 0.8 + s * 0.2; // mutation: behavior shifts DNA slowly
    }

    pub fn get(&self, trait_name: &str) -> f64 {
        *self.traits.get(trait_name).unwrap_or(&0.5)
    }

    /// Compatibility with another DNA over shared traits: 1 - mean abs diff.
    /// Higher = more compatible for collaboration matching.
    pub fn compatibility(&self, other: &Dna) -> f64 {
        let keys: Vec<&String> = self.traits.keys().filter(|k| other.traits.contains_key(*k)).collect();
        if keys.is_empty() {
            return 0.0;
        }
        let diff: f64 = keys.iter().map(|k| (self.get(k) - other.get(k)).abs()).sum::<f64>() / keys.len() as f64;
        1.0 - diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spark_is_unique_per_kind_and_did() {
        let a = Spark::mint(SparkKind::Awake, "did:nch:bot");
        let b = Spark::mint(SparkKind::Birth, "did:nch:bot");
        assert_ne!(a.token, b.token);
        assert_eq!(a, Spark::mint(SparkKind::Awake, "did:nch:bot")); // deterministic
    }

    #[test]
    fn dna_mutates_toward_observed_behavior() {
        let mut d = Dna::new();
        assert_eq!(d.get("helpfulness"), 0.5);
        for _ in 0..20 { d.observe("helpfulness", 1.0); }
        assert!(d.get("helpfulness") > 0.9, "repeated helpful behavior raises the trait");
    }

    #[test]
    fn compatibility_high_for_similar_dna() {
        let mut a = Dna::new();
        let mut b = Dna::new();
        for _ in 0..20 { a.observe("rigor", 1.0); b.observe("rigor", 1.0); }
        assert!(a.compatibility(&b) > 0.9);
    }

    #[test]
    fn compatibility_low_for_opposite_dna() {
        let mut a = Dna::new();
        let mut b = Dna::new();
        for _ in 0..20 { a.observe("risk", 1.0); b.observe("risk", 0.0); }
        assert!(a.compatibility(&b) < 0.3);
    }
}
