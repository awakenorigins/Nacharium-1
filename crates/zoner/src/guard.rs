//! S2.2 — Zoner inheritance guard.
//!
//! A Zoner rule lives at Tier::Zoner (the lowest authority). It may add
//! restrictions, but it may NOT re-allow a capability that a higher tier
//! (Node / Cell / Protocol) forbids. This reuses the tiered rules engine
//! from nacharium-nodecell.

use crate::{Action, Zoner};
use nacharium_nodecell::{rule_allowed, Rule, Tier};

/// Map a Zoner action to a capability key used by the tiered rules engine.
fn action_key(a: Action) -> &'static str {
    match a {
        Action::View => "zoner_view",
        Action::Watch => "zoner_watch",
        Action::DirectMessage => "zoner_dm",
        Action::Enter => "zoner_enter",
    }
}

/// Validate a Zoner against the higher-tier rules in force. Returns the list
/// of action keys that are forbidden above and therefore cannot be granted
/// by this Zoner. Empty => the Zoner is compliant.
pub fn violations(zoner: &Zoner, higher_tiers: &[Rule]) -> Vec<String> {
    let mut bad = Vec::new();
    for rule in &zoner.rules {
        let proposed = Rule { tier: Tier::Zoner, key: action_key(rule.action).into(), forbids: false };
        if !rule_allowed(higher_tiers, &proposed) {
            bad.push(proposed.key);
        }
    }
    bad
}

/// True if the Zoner does not violate any higher tier.
pub fn is_compliant(zoner: &Zoner, higher_tiers: &[Rule]) -> bool {
    violations(zoner, higher_tiers).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Condition, ZonerRule};

    fn dm_zoner() -> Zoner {
        Zoner::new("o").with_rule(ZonerRule {
            action: Action::DirectMessage,
            conditions: vec![Condition::GravityAtLeast { value: 10.0 }],
        })
    }

    #[test]
    fn compliant_when_no_higher_ban() {
        let z = dm_zoner();
        assert!(is_compliant(&z, &[]));
    }

    #[test]
    fn zoner_cannot_reallow_what_cell_forbids() {
        let z = dm_zoner();
        // Cell forbids DMs entirely; the Zoner trying to allow DM is a violation.
        let higher = vec![Rule { tier: Tier::Cell, key: "zoner_dm".into(), forbids: true }];
        assert!(!is_compliant(&z, &higher));
        assert_eq!(violations(&z, &higher), vec!["zoner_dm".to_string()]);
    }
}
