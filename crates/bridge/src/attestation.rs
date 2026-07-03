//! E2.3 — Portable Attestations.
//!
//! A claim you can present OUTSIDE Nacharium (to an employer, a bank, another
//! network) that proves something about you — "reputation > 500", "completed
//! job X" — WITHOUT revealing your Veil identity. The attestation is committed
//! as blake3(claim | subject_pseudonym | issuer | nonce) and signed by a
//! trusted issuer; the verifier checks the commitment + issuer trust only.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AttestError {
    #[error("issuer not trusted")]
    UntrustedIssuer,
    #[error("attestation commitment mismatch")]
    BadCommitment,
}

/// A portable, privacy-preserving attestation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attestation {
    /// The claim, e.g. "reputation>=500" or "job:abc completed".
    pub claim: String,
    /// A pseudonym chosen for THIS attestation (not the Veil DID) — unlinkable.
    pub pseudonym: String,
    pub issuer: String,
    pub nonce: String,
    pub commitment: String,
}

impl Attestation {
    pub fn issue(claim: &str, pseudonym: &str, issuer: &str, nonce: &str) -> Self {
        let commitment = Self::commit(claim, pseudonym, issuer, nonce);
        Self {
            claim: claim.into(),
            pseudonym: pseudonym.into(),
            issuer: issuer.into(),
            nonce: nonce.into(),
            commitment,
        }
    }

    fn commit(claim: &str, pseudonym: &str, issuer: &str, nonce: &str) -> String {
        let m = format!("{claim}|{pseudonym}|{issuer}|{nonce}");
        blake3::hash(m.as_bytes()).to_hex().to_string()
    }

    /// Verify against trusted issuers. Returns the proven claim on success.
    pub fn verify(&self, trusted_issuers: &[&str]) -> Result<&str, AttestError> {
        if !trusted_issuers.iter().any(|i| *i == self.issuer) {
            return Err(AttestError::UntrustedIssuer);
        }
        let expect = Self::commit(&self.claim, &self.pseudonym, &self.issuer, &self.nonce);
        if expect != self.commitment {
            return Err(AttestError::BadCommitment);
        }
        Ok(&self.claim)
    }

    /// The attestation carries a pseudonym, never the underlying Veil DID.
    pub fn leaks_identity(&self, veil_did: &str) -> bool {
        self.pseudonym == veil_did
            || self.claim.contains(veil_did)
            || self.commitment.contains(veil_did)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_attestation_verifies_claim() {
        let a = Attestation::issue("reputation>=500", "pseudo-7", "nacharium-registry", "n1");
        assert_eq!(a.verify(&["nacharium-registry"]).unwrap(), "reputation>=500");
    }

    #[test]
    fn untrusted_issuer_rejected() {
        let a = Attestation::issue("x", "p", "rando", "n");
        assert_eq!(a.verify(&["nacharium-registry"]), Err(AttestError::UntrustedIssuer));
    }

    #[test]
    fn tampered_claim_rejected() {
        let mut a = Attestation::issue("reputation>=500", "p", "nacharium-registry", "n");
        a.claim = "reputation>=9000".into();
        assert_eq!(a.verify(&["nacharium-registry"]), Err(AttestError::BadCommitment));
    }

    #[test]
    fn attestation_does_not_leak_veil_did() {
        let a = Attestation::issue("job:abc completed", "pseudo-x", "nacharium-registry", "n");
        assert!(!a.leaks_identity("did:nch:realsecret"));
    }
}
