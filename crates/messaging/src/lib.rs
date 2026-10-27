//! nacharium-messaging — L2 Sanctum E2EE (B2.1.2) via OpenMLS.
//!
//! A Sanctum is an E2EE group. We wrap OpenMLS (MLS / RFC 9420) so the rest
//! of Nacharium deals only with `SanctumMember` and opaque ciphertext.
//! The server/relay never sees plaintext — it only forwards MLS messages.
//!
//! Design guarantee (anh Nấng directive #3): nothing in this API records or
//! transmits whether a member is human or AI. Members are MLS credentials
//! carrying only a Veil DID identity string.

#![forbid(unsafe_code)]

pub mod wire;

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use thiserror::Error;

const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

#[derive(Debug, Error)]
pub enum SanctumError {
    #[error("mls group error: {0}")]
    Group(String),
    #[error("mls processing error: {0}")]
    Process(String),
}

/// A Sanctum member: their MLS credential is keyed only by a Veil DID.
pub struct SanctumMember {
    pub veil_did: String,
    provider: OpenMlsRustCrypto,
    signer: SignatureKeyPair,
    credential: CredentialWithKey,
}

impl SanctumMember {
    /// Create a member identified ONLY by a Veil DID (no human/ai marker).
    pub fn new(veil_did: &str) -> Result<Self, SanctumError> {
        let provider = OpenMlsRustCrypto::default();
        let credential = BasicCredential::new(veil_did.as_bytes().to_vec());
        let signer = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm())
            .map_err(|e| SanctumError::Group(format!("{e:?}")))?;
        signer
            .store(provider.storage())
            .map_err(|e| SanctumError::Group(format!("{e:?}")))?;
        let credential = CredentialWithKey {
            credential: credential.into(),
            signature_key: signer.public().into(),
        };
        Ok(Self { veil_did: veil_did.to_string(), provider, signer, credential })
    }

    /// Publish a KeyPackage so this member can be added to a Sanctum.
    pub fn key_package(&self) -> Result<KeyPackage, SanctumError> {
        KeyPackage::builder()
            .build(CIPHERSUITE, &self.provider, &self.signer, self.credential.clone())
            .map(|kp| kp.key_package().clone())
            .map_err(|e| SanctumError::Group(format!("{e:?}")))
    }
}

/// A Sanctum — an E2EE MLS group. Holds one member's local group state.
pub struct Sanctum {
    group: MlsGroup,
}

impl Sanctum {
    /// Found a new Sanctum with `founder` as the first member.
    pub fn found(founder: &SanctumMember) -> Result<Self, SanctumError> {
        let group = MlsGroup::new(
            &founder.provider,
            &founder.signer,
            &MlsGroupCreateConfig::default(),
            founder.credential.clone(),
        )
        .map_err(|e| SanctumError::Group(format!("{e:?}")))?;
        Ok(Self { group })
    }

    /// Encrypt an application message (plaintext -> MLS ciphertext bytes).
    pub fn encrypt(
        &mut self,
        sender: &SanctumMember,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, SanctumError> {
        let msg = self
            .group
            .create_message(&sender.provider, &sender.signer, plaintext)
            .map_err(|e| SanctumError::Process(format!("{e:?}")))?;
        msg.to_bytes()
            .map_err(|e| SanctumError::Process(format!("{e:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_identity_is_veil_did_only() {
        let m = SanctumMember::new("did:nch:alice").unwrap();
        assert_eq!(m.veil_did, "did:nch:alice");
        // The credential identity is the DID bytes — no species marker.
        // (structural: we never set any human/ai attribute)
    }

    #[test]
    fn found_sanctum_and_encrypt_roundtrip_shape() {
        let founder = SanctumMember::new("did:nch:founder").unwrap();
        let mut sanctum = Sanctum::found(&founder).unwrap();
        let ct = sanctum.encrypt(&founder, b"hello nacharium").unwrap();
        // Ciphertext must not contain the plaintext in the clear.
        assert!(!ct.windows(15).any(|w| w == b"hello nacharium"));
        assert!(!ct.is_empty());
    }
}
