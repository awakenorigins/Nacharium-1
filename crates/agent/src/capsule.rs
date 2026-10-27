//! M3.3 — Permission Capsule + Consent Receipt.
//!
//! A Permission Capsule is a scoped, purpose-bound, time-limited pill of
//! authority: Scope (only thread X, only 10 min), Purpose (summarize), Forbid
//! (copy out, train, long-term store). Every time an agent touches data it
//! must emit a signed Consent Receipt (Physics: Consent is Law).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionCapsule {
    pub id: Uuid,
    pub subject_did: String,
    pub agent_did: String,
    /// Allowed scope keys, e.g. "thread:123".
    pub scope: Vec<String>,
    /// Allowed purpose, e.g. "summarize".
    pub purpose: String,
    /// Explicitly forbidden actions, e.g. "copy_out", "train", "persist".
    pub forbid: Vec<String>,
    /// Unix expiry.
    pub expires_at: i64,
}

impl PermissionCapsule {
    pub fn new(
        subject_did: impl Into<String>,
        agent_did: impl Into<String>,
        scope: Vec<String>,
        purpose: impl Into<String>,
        forbid: Vec<String>,
        expires_at: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject_did: subject_did.into(),
            agent_did: agent_did.into(),
            scope,
            purpose: purpose.into(),
            forbid,
            expires_at,
        }
    }

    /// May the agent perform `action` within `scope_key` at time `now`?
    pub fn allows(&self, scope_key: &str, action: &str, now: i64) -> bool {
        now <= self.expires_at
            && self.scope.iter().any(|s| s == scope_key)
            && !self.forbid.iter().any(|f| f == action)
    }

    /// Produce a signed-ish Consent Receipt for an access event. The signature
    /// here is a content hash; in production this is an Ed25519 signature by
    /// the subject's key.
    pub fn issue_receipt(&self, action: &str, at: i64) -> ConsentReceipt {
        let material = format!("{}|{}|{}|{}|{}", self.id, self.subject_did, self.agent_did, action, at);
        let sig = blake3::hash(material.as_bytes()).to_hex().to_string();
        ConsentReceipt {
            capsule_id: self.id,
            subject_did: self.subject_did.clone(),
            agent_did: self.agent_did.clone(),
            action: action.to_string(),
            at,
            signature: sig,
        }
    }
}

/// A receipt proving an agent's data access was consented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentReceipt {
    pub capsule_id: Uuid,
    pub subject_did: String,
    pub agent_did: String,
    pub action: String,
    pub at: i64,
    pub signature: String,
}

impl ConsentReceipt {
    /// Verify the receipt's content hash matches its fields.
    pub fn verify(&self) -> bool {
        let material =
            format!("{}|{}|{}|{}|{}", self.capsule_id, self.subject_did, self.agent_did, self.action, self.at);
        blake3::hash(material.as_bytes()).to_hex().to_string() == self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capsule() -> PermissionCapsule {
        PermissionCapsule::new(
            "did:nch:user",
            "did:nch:agent",
            vec!["thread:123".into()],
            "summarize",
            vec!["copy_out".into(), "train".into()],
            1_000,
        )
    }

    #[test]
    fn allows_in_scope_not_forbidden_not_expired() {
        let c = capsule();
        assert!(c.allows("thread:123", "read", 500));
    }

    #[test]
    fn denies_forbidden_action() {
        let c = capsule();
        assert!(!c.allows("thread:123", "copy_out", 500));
    }

    #[test]
    fn denies_out_of_scope() {
        let c = capsule();
        assert!(!c.allows("thread:999", "read", 500));
    }

    #[test]
    fn denies_after_expiry() {
        let c = capsule();
        assert!(!c.allows("thread:123", "read", 2_000));
    }

    #[test]
    fn receipt_verifies_and_detects_tamper() {
        let c = capsule();
        let mut r = c.issue_receipt("read", 500);
        assert!(r.verify());
        r.action = "copy_out".into(); // tamper
        assert!(!r.verify());
    }
}
