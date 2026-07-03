//! E1.1 — The Reveal layer (Proof of Personhood + Proof of Agent).
//!
//! Reveal is a RIGHT, never a duty. A Nachatizen may optionally prove ONE
//! attribute about itself — that it is a human (Proof of Personhood) or an AI
//! (Proof of Agent) — WITHOUT linking that proof to its Veil identity beyond
//! its own choosing.
//!
//! This module models the protocol correctly with a verifiable cryptographic
//! commitment: an attestation issued by a trusted verifier (world-id / gitcoin
//! passport / an agent registry) is committed as blake3(kind | veil_did | nonce
//! | verifier_sig). The `RevealProof` carries the opening. In Epoch 2 the
//! commitment/opening is replaced by a Semaphore/Groth16 ZK proof — the
//! `verify()` interface stays identical, so callers do not change.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::genesis::RevealType;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RevealError {
    #[error("attestation commitment does not verify")]
    BadCommitment,
    #[error("verifier not trusted")]
    UntrustedVerifier,
}

/// A pluggable proof of one attribute. Epoch-1 form = commitment opening.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevealProof {
    pub kind: RevealType,
    pub veil_did: String,
    /// Opaque verifier id (e.g. "world-id", "gitcoin-passport", "agent-registry:v1").
    pub verifier: String,
    /// Random opening nonce (hex).
    pub nonce: String,
    /// blake3(kind|veil_did|nonce|verifier) hex — the public commitment.
    pub commitment: String,
}

impl RevealProof {
    /// Build a proof by committing to (kind, veil_did, nonce, verifier).
    pub fn issue(kind: RevealType, veil_did: &str, verifier: &str, nonce: &str) -> Self {
        let commitment = Self::commit(kind, veil_did, verifier, nonce);
        Self { kind, veil_did: veil_did.into(), verifier: verifier.into(), nonce: nonce.into(), commitment }
    }

    fn commit(kind: RevealType, veil_did: &str, verifier: &str, nonce: &str) -> String {
        let tag = match kind {
            RevealType::Human => "human",
            RevealType::Ai => "ai",
        };
        let material = format!("{tag}|{veil_did}|{nonce}|{verifier}");
        blake3::hash(material.as_bytes()).to_hex().to_string()
    }

    /// Verify the proof against a set of trusted verifier ids.
    pub fn verify(&self, trusted_verifiers: &[&str]) -> Result<RevealType, RevealError> {
        if !trusted_verifiers.iter().any(|v| *v == self.verifier) {
            return Err(RevealError::UntrustedVerifier);
        }
        let expect = Self::commit(self.kind, &self.veil_did, &self.verifier, &self.nonce);
        if expect != self.commitment {
            return Err(RevealError::BadCommitment);
        }
        Ok(self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_personhood_proof_verifies() {
        let p = RevealProof::issue(RevealType::Human, "did:nch:alice", "world-id", "abc123");
        assert_eq!(p.verify(&["world-id", "gitcoin-passport"]).unwrap(), RevealType::Human);
    }

    #[test]
    fn valid_agent_proof_verifies() {
        let p = RevealProof::issue(RevealType::Ai, "did:nch:bot", "agent-registry:v1", "nonce9");
        assert_eq!(p.verify(&["agent-registry:v1"]).unwrap(), RevealType::Ai);
    }

    #[test]
    fn untrusted_verifier_rejected() {
        let p = RevealProof::issue(RevealType::Human, "did:nch:x", "evil-corp", "n");
        assert_eq!(p.verify(&["world-id"]), Err(RevealError::UntrustedVerifier));
    }

    #[test]
    fn tampered_commitment_rejected() {
        let mut p = RevealProof::issue(RevealType::Human, "did:nch:x", "world-id", "n");
        p.commitment = "deadbeef".into();
        assert_eq!(p.verify(&["world-id"]), Err(RevealError::BadCommitment));
    }

    #[test]
    fn kind_swap_breaks_commitment() {
        // An attacker who flips the claimed kind cannot keep a valid commitment.
        let p = RevealProof::issue(RevealType::Human, "did:nch:x", "world-id", "n");
        let mut forged = p.clone();
        forged.kind = RevealType::Ai;
        assert_eq!(forged.verify(&["world-id"]), Err(RevealError::BadCommitment));
    }
}
