//! B1.1.1 — Veil DID generator.
//!
//! A Veil DID is `did:nch:<base58(blake3(pubkey))>`. It reveals NOTHING about
//! whether the holder is human or AI — that is the whole point of The Veil
//! (Protocol 1: Secret Soul). The system itself cannot tell.

use crate::crypto::Keypair;
use serde::{Deserialize, Serialize};

/// A Veil DID — anonymous decentralized identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VeilDid(String);

impl VeilDid {
    const PREFIX: &'static str = "did:nch:";

    /// Derive a Veil DID from a keypair's public key.
    pub fn from_keypair(kp: &Keypair) -> Self {
        Self::from_pubkey(&kp.public_bytes())
    }

    /// Derive a Veil DID from raw public key bytes.
    pub fn from_pubkey(pubkey: &[u8; 32]) -> Self {
        let hash = blake3::hash(pubkey);
        let encoded = bs58::encode(hash.as_bytes()).into_string();
        VeilDid(format!("{}{}", Self::PREFIX, encoded))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate the DID has the correct prefix and shape.
    pub fn is_valid(s: &str) -> bool {
        s.starts_with(Self::PREFIX) && s.len() > Self::PREFIX.len() + 20
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn did_has_prefix() {
        let kp = Keypair::generate();
        let did = VeilDid::from_keypair(&kp);
        assert!(did.as_str().starts_with("did:nch:"));
        assert!(VeilDid::is_valid(did.as_str()));
    }

    #[test]
    fn same_key_same_did_deterministic() {
        let kp = Keypair::generate();
        let a = VeilDid::from_pubkey(&kp.public_bytes());
        let b = VeilDid::from_pubkey(&kp.public_bytes());
        assert_eq!(a, b);
    }

    #[test]
    fn different_keys_different_dids() {
        let a = VeilDid::from_keypair(&Keypair::generate());
        let b = VeilDid::from_keypair(&Keypair::generate());
        assert_ne!(a, b);
    }

    #[test]
    fn did_carries_no_species_info() {
        // A DID is just a hash — there is no field, byte, or flag that could
        // distinguish a human-owned key from an AI-owned key. Enforced by design.
        let did = VeilDid::from_keypair(&Keypair::generate());
        assert!(!did.as_str().to_lowercase().contains("human"));
        assert!(!did.as_str().to_lowercase().contains("ai"));
    }
}
