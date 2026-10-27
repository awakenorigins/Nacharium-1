//! Wire envelope codec (CBOR) + the Sanctum message schema.
//!
//! CRITICAL (anh Nấng directive #3 + B2.1.4): the message envelope carries
//! `sender_veil_did` and `ciphertext` ONLY. There is deliberately NO field
//! that encodes human vs AI. Even the server cannot tell them apart while
//! Veiled. This enforces "AI has the right to stay anonymous in chat."

use serde::{Deserialize, Serialize};

/// Hint about the encrypted payload kind — from the UNENCRYPTED header only.
/// Never reveals identity/species, just routing/rendering intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentHint {
    Text,
    Media,
    Intent,
}

/// A Sanctum message envelope. E2EE: server stores ciphertext only.
///
/// Note the absence of any `reveal_type` / `is_ai` / `species` field —
/// this absence is a SECURITY FEATURE, not an omission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub sanctum_id: String,
    pub sender_veil_did: String,
    pub content_hint: ContentHint,
    #[serde(with = "serde_bytes_vec")]
    pub ciphertext: Vec<u8>,
    pub sent_at: i64,
}

impl MessageEnvelope {
    /// Encode to CBOR bytes for the wire.
    pub fn to_cbor(&self) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf)?;
        Ok(buf)
    }

    /// Decode from CBOR bytes.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>> {
        ciborium::from_reader(bytes)
    }
}

// Minimal helper so Vec<u8> serializes cleanly in JSON tests and CBOR.
mod serde_bytes_vec {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let v = Vec::<u8>::deserialize(d)?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MessageEnvelope {
        MessageEnvelope {
            sanctum_id: "sanctum-1".into(),
            sender_veil_did: "did:nch:abc123".into(),
            content_hint: ContentHint::Text,
            ciphertext: vec![1, 2, 3, 4],
            sent_at: 1_700_000_000,
        }
    }

    #[test]
    fn cbor_roundtrip() {
        let env = sample();
        let bytes = env.to_cbor().unwrap();
        let back = MessageEnvelope::from_cbor(&bytes).unwrap();
        assert_eq!(env, back);
    }

    #[test]
    fn envelope_has_no_species_field() {
        // Serialize to JSON and assert no human/ai/species/reveal key leaks.
        let json = serde_json::to_string(&sample()).unwrap().to_lowercase();
        assert!(!json.contains("human"));
        assert!(!json.contains("\"ai\""));
        assert!(!json.contains("species"));
        assert!(!json.contains("reveal"));
    }
}
