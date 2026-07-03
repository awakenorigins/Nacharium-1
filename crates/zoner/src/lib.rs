//! nacharium-zoner — L3 Zoner: sovereignty rules engine (M3.1).
//!
//! A Zoner is a personal territory where "you are the law". Access is
//! programmable: comment-to-view, cheer-to-watch, gravity-gate, entry-fee,
//! AI-must-declare-intent, sanctum-only. Zoner rules may NOT violate the
//! higher tiers (Node/Cell/Protocol) — enforced by nodecell::Tier.

#![forbid(unsafe_code)]

pub mod guard;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single access condition on a Zoner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    /// Visitor must have gravity >= threshold.
    GravityAtLeast { value: f64 },
    /// Visitor must pay an entry fee in NARI.
    EntryFee { nari: u64 },
    /// Visitor (if an agent) must declare intent before entering.
    AgentMustDeclareIntent,
    /// Visitor must have left a comment to keep viewing.
    CommentToView,
    /// Visitor must cheer to keep watching.
    CheerToWatch,
}

/// The action a Zoner rule applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    View,
    Watch,
    DirectMessage,
    Enter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZonerRule {
    pub action: Action,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zoner {
    pub id: Uuid,
    pub owner_did: String,
    pub rules: Vec<ZonerRule>,
}

/// Facts about a visitor, used to evaluate access.
#[derive(Debug, Clone, Default)]
pub struct Visitor {
    pub gravity: f64,
    pub nari_balance: u64,
    pub is_agent: bool,
    pub declared_intent: bool,
    pub left_comment: bool,
    pub cheered: bool,
}

/// The outcome of an access evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny { reason: String },
}

impl Zoner {
    pub fn new(owner_did: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), owner_did: owner_did.into(), rules: Vec::new() }
    }

    pub fn with_rule(mut self, rule: ZonerRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Evaluate whether `visitor` may perform `action` in this Zoner.
    pub fn evaluate(&self, action: Action, visitor: &Visitor) -> Decision {
        for rule in self.rules.iter().filter(|r| r.action == action) {
            for cond in &rule.conditions {
                if let Some(reason) = unmet(cond, visitor) {
                    return Decision::Deny { reason };
                }
            }
        }
        Decision::Allow
    }
}

/// Returns Some(reason) if the condition is NOT met.
fn unmet(cond: &Condition, v: &Visitor) -> Option<String> {
    match cond {
        Condition::GravityAtLeast { value } => {
            (v.gravity < *value).then(|| format!("gravity {} < required {}", v.gravity, value))
        }
        Condition::EntryFee { nari } => {
            (v.nari_balance < *nari).then(|| format!("insufficient NARI: need {}", nari))
        }
        Condition::AgentMustDeclareIntent => {
            (v.is_agent && !v.declared_intent).then(|| "agent must declare intent".into())
        }
        Condition::CommentToView => {
            (!v.left_comment).then(|| "must leave a comment to view".into())
        }
        Condition::CheerToWatch => (!v.cheered).then(|| "must cheer to watch".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gravity_gated_dm() -> Zoner {
        Zoner::new("did:nch:owner").with_rule(ZonerRule {
            action: Action::DirectMessage,
            conditions: vec![Condition::GravityAtLeast { value: 100.0 }],
        })
    }

    #[test]
    fn gravity_gate_blocks_low_gravity() {
        let z = gravity_gated_dm();
        let low = Visitor { gravity: 50.0, ..Default::default() };
        assert!(matches!(z.evaluate(Action::DirectMessage, &low), Decision::Deny { .. }));
        let high = Visitor { gravity: 150.0, ..Default::default() };
        assert_eq!(z.evaluate(Action::DirectMessage, &high), Decision::Allow);
    }

    #[test]
    fn unrelated_action_is_allowed() {
        let z = gravity_gated_dm();
        // A DM rule doesn't restrict View.
        assert_eq!(z.evaluate(Action::View, &Visitor::default()), Decision::Allow);
    }

    #[test]
    fn agent_must_declare_intent() {
        let z = Zoner::new("o").with_rule(ZonerRule {
            action: Action::Enter,
            conditions: vec![Condition::AgentMustDeclareIntent],
        });
        let agent_no = Visitor { is_agent: true, declared_intent: false, ..Default::default() };
        assert!(matches!(z.evaluate(Action::Enter, &agent_no), Decision::Deny { .. }));
        let agent_yes = Visitor { is_agent: true, declared_intent: true, ..Default::default() };
        assert_eq!(z.evaluate(Action::Enter, &agent_yes), Decision::Allow);
        // A human (not agent) is not bound by the intent rule.
        let human = Visitor { is_agent: false, ..Default::default() };
        assert_eq!(z.evaluate(Action::Enter, &human), Decision::Allow);
    }

    #[test]
    fn entry_fee_and_comment_stack() {
        let z = Zoner::new("o").with_rule(ZonerRule {
            action: Action::View,
            conditions: vec![Condition::EntryFee { nari: 1 }, Condition::CommentToView],
        });
        let broke = Visitor { nari_balance: 0, left_comment: true, ..Default::default() };
        assert!(matches!(z.evaluate(Action::View, &broke), Decision::Deny { .. }));
        let ok = Visitor { nari_balance: 5, left_comment: true, ..Default::default() };
        assert_eq!(z.evaluate(Action::View, &ok), Decision::Allow);
    }
}
