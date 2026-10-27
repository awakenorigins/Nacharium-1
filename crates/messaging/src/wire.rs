//! S1.5 — Wire a Sanctum ciphertext into a transport envelope.
//!
//! Bridges messaging (OpenMLS ciphertext) -> core::envelope (the wire format
//! that goes over gossipsub). The relay only ever handles this envelope and
//! never the plaintext. The envelope carries NO human/ai field by design.

use crate::{Sanctum, SanctumError, SanctumMember};
use nacharium_core::envelope::{ContentHint, MessageEnvelope};

/// Encrypt a plaintext and wrap it in a transport envelope ready for the
/// `nch/sanctum/{id}/v1` gossipsub topic.
pub fn sealed_envelope(
    sanctum: &mut Sanctum,
    sender: &SanctumMember,
    sanctum_id: &str,
    plaintext: &[u8],
    sent_at: i64,
) -> Result<MessageEnvelope, SanctumError> {
    let ciphertext = sanctum.encrypt(sender, plaintext)?;
    Ok(MessageEnvelope {
        sanctum_id: sanctum_id.to_string(),
        sender_veil_did: sender.veil_did.clone(),
        content_hint: ContentHint::Text,
        ciphertext,
        sent_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_envelope_is_transport_ready_and_opaque() {
        let founder = SanctumMember::new("did:nch:founder").unwrap();
        let mut sanctum = Sanctum::found(&founder).unwrap();
        let env =
            sealed_envelope(&mut sanctum, &founder, "sanctum-1", b"secret words", 1_700_000_000)
                .unwrap();

        // Round-trips through the wire codec.
        let bytes = env.to_cbor().unwrap();
        let back = MessageEnvelope::from_cbor(&bytes).unwrap();
        assert_eq!(back.sanctum_id, "sanctum-1");
        assert_eq!(back.sender_veil_did, "did:nch:founder");

        // Relay-visible bytes must not contain the plaintext.
        assert!(!back.ciphertext.windows(12).any(|w| w == b"secret words"));

        // Serialized envelope carries no species marker.
        let json = serde_json::to_string(&back).unwrap().to_lowercase();
        assert!(!json.contains("human"));
        assert!(!json.contains("species"));
    }
}
