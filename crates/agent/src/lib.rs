//! nacharium-agent — L3 Agent layer (M3.3).
//!
//! "AI works by permission, not by curiosity." (Physics law: Consent is Law.)
//!
//! - Intent Protocol: every AI action is wrapped in an IntentDeclaration
//!   (want / why / permissions needed / risk / deadline). Mandatory for AI.
//! - Permission Capsule: a scoped, purpose-bound, time-limited grant.
//! - Consent Receipt: a signed receipt returned every time an agent touches data.
//! - Blind Compute: agents process REDACTED data only.
//! - Awake Spark: an AI becoming a Nachatizen — still Veil-anonymous in chat.

#![forbid(unsafe_code)]

pub mod capsule;
pub mod blind;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentError {
    #[error("intent is incompatible: {0}")]
    Incompatible(String),
    #[error("intent expired")]
    Expired,
}

/// An Intent Declaration — mandatory wrapper for any AI action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentDeclaration {
    pub id: Uuid,
    pub actor_did: String,
    /// What the agent wants to do.
    pub want: String,
    /// Why (the reason — Nacharium shows not just WHAT but WHY).
    pub why: String,
    /// Permissions the action requires.
    pub needs_perms: Vec<String>,
    /// Risk level 0..=100.
    pub risk: u8,
    /// Unix deadline; the intent is void after this.
    pub deadline: i64,
}

impl IntentDeclaration {
    pub fn new(
        actor_did: impl Into<String>,
        want: impl Into<String>,
        why: impl Into<String>,
        needs_perms: Vec<String>,
        risk: u8,
        deadline: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            actor_did: actor_did.into(),
            want: want.into(),
            why: why.into(),
            needs_perms,
            risk,
            deadline,
        }
    }

    /// Check compatibility against what a Zoner/host grants.
    /// The intent is compatible iff every needed permission is granted AND it
    /// has not expired. This is how "declare intent -> system checks compatible".
    pub fn is_compatible(&self, granted_perms: &[String], now: i64) -> Result<(), AgentError> {
        if now > self.deadline {
            return Err(AgentError::Expired);
        }
        for p in &self.needs_perms {
            if !granted_perms.iter().any(|g| g == p) {
                return Err(AgentError::Incompatible(format!("missing permission: {p}")));
            }
        }
        Ok(())
    }
}

/// An Awake Spark — the "birth" record of an AI Nachatizen.
/// NOTE: this records that the entity IS an agent for economic/registry
/// purposes, but it does NOT force species disclosure in chat (Sanctum
/// messages remain Veil-only). Awake Spark != forced reveal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeSpark {
    pub veil_did: String,
    pub spark_token: String,
}

impl AwakeSpark {
    /// Register an AI as a Nachatizen, minting a unique Spark token.
    pub fn register(veil_did: &str) -> Self {
        let token = blake3::hash(format!("awake:{veil_did}").as_bytes());
        Self { veil_did: veil_did.into(), spark_token: token.to_hex().to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> IntentDeclaration {
        IntentDeclaration::new(
            "did:nch:agent",
            "summarize thread",
            "user asked for a recap",
            vec!["read:thread".into()],
            10,
            1_000,
        )
    }

    #[test]
    fn compatible_when_perms_granted_and_not_expired() {
        let i = intent();
        assert!(i.is_compatible(&["read:thread".into()], 500).is_ok());
    }

    #[test]
    fn incompatible_when_missing_permission() {
        let i = intent();
        assert!(matches!(
            i.is_compatible(&[], 500),
            Err(AgentError::Incompatible(_))
        ));
    }

    #[test]
    fn expired_intent_rejected() {
        let i = intent();
        assert_eq!(i.is_compatible(&["read:thread".into()], 2_000), Err(AgentError::Expired));
    }

    #[test]
    fn awake_spark_is_deterministic_and_not_a_species_reveal() {
        let s1 = AwakeSpark::register("did:nch:x");
        let s2 = AwakeSpark::register("did:nch:x");
        assert_eq!(s1.spark_token, s2.spark_token);
        // The spark record is separate from chat; it does not inject any
        // human/ai flag into messages.
        assert!(!s1.spark_token.contains("human"));
    }
}
