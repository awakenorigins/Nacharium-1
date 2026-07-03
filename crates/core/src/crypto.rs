//! M0.2 — Crypto Core (B0.2.1 keypair, B0.2.2 primitives).
//!
//! "Key is Crown" — an entity IS its Ed25519 keypair. Possession of the
//! secret key == control of the account. No passwords, no server-side auth.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid public key bytes")]
    InvalidKey,
}

/// A Nacharium identity keypair. The crown.
pub struct Keypair {
    signing: SigningKey,
}

impl Keypair {
    /// Generate a fresh keypair (a new crown).
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        Self { signing }
    }

    /// The public verifying key bytes (32 bytes).
    pub fn public_bytes(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    /// Sign a message.
    pub fn sign(&self, msg: &[u8]) -> Signature {
        self.signing.sign(msg)
    }
}

/// Verify a signature against a raw 32-byte public key.
pub fn verify(pubkey: &[u8; 32], msg: &[u8], sig: &Signature) -> Result<(), CryptoError> {
    let vk = VerifyingKey::from_bytes(pubkey).map_err(|_| CryptoError::InvalidKey)?;
    vk.verify(msg, sig).map_err(|_| CryptoError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let kp = Keypair::generate();
        let msg = b"nacharium genesis";
        let sig = kp.sign(msg);
        assert!(verify(&kp.public_bytes(), msg, &sig).is_ok());
    }

    #[test]
    fn tampered_message_fails() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"original");
        assert!(verify(&kp.public_bytes(), b"tampered", &sig).is_err());
    }
}
