//! E2.1 — DarkZone + immune system.
//!
//! A Node that loses control (too many violations) falls into the DarkZone:
//! reduced connectivity/discovery, Dark NARI only. A Cell with too many
//! DarkZone Nodes becomes a DarkZone Cell — the community polices itself
//! (immune response). Emergence: the path back to mainnet runs through the
//! Jury Pool. DarkZone is not hell — it is wilderness: high risk, high freedom.

use serde::{Deserialize, Serialize};

/// Health status of a Node or Cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    /// Normal mainnet.
    #[default]
    Main,
    /// Quarantined — reduced reach, Dark NARI, on the way out or back.
    Dark,
}

/// Tracks violations that push a node toward the DarkZone.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImmuneState {
    pub violations: u32,
    pub zone: Zone,
}

/// Number of violations that trip a Node into the DarkZone.
pub const DARKZONE_THRESHOLD: u32 = 3;
/// Fraction of a Cell's nodes that must be Dark for the Cell to fall.
pub const CELL_DARK_FRACTION: f64 = 0.5;

impl ImmuneState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a violation; auto-quarantine into DarkZone past the threshold.
    pub fn record_violation(&mut self) {
        self.violations += 1;
        if self.violations >= DARKZONE_THRESHOLD {
            self.zone = Zone::Dark;
        }
    }

    pub fn is_dark(&self) -> bool {
        self.zone == Zone::Dark
    }

    /// Reach multiplier: DarkZone nodes get their discovery/connectivity cut.
    pub fn reach_multiplier(&self) -> f64 {
        if self.is_dark() { 0.1 } else { 1.0 }
    }

    /// Emergence: a Jury acquittal restores the node to mainnet and resets it.
    pub fn emerge_via_jury(&mut self, acquitted: bool) {
        if acquitted {
            self.violations = 0;
            self.zone = Zone::Main;
        }
    }
}

/// A Cell becomes Dark when at least CELL_DARK_FRACTION of its nodes are Dark.
pub fn cell_is_dark(dark_nodes: usize, total_nodes: usize) -> bool {
    if total_nodes == 0 {
        return false;
    }
    (dark_nodes as f64) / (total_nodes as f64) >= CELL_DARK_FRACTION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_falls_to_darkzone_past_threshold() {
        let mut s = ImmuneState::new();
        s.record_violation();
        s.record_violation();
        assert!(!s.is_dark());
        s.record_violation(); // 3rd -> dark
        assert!(s.is_dark());
        assert!(s.reach_multiplier() < 1.0);
    }

    #[test]
    fn jury_acquittal_restores_to_mainnet() {
        let mut s = ImmuneState::new();
        for _ in 0..5 { s.record_violation(); }
        assert!(s.is_dark());
        s.emerge_via_jury(true);
        assert!(!s.is_dark());
        assert_eq!(s.violations, 0);
    }

    #[test]
    fn guilty_verdict_keeps_darkzone() {
        let mut s = ImmuneState::new();
        for _ in 0..5 { s.record_violation(); }
        s.emerge_via_jury(false);
        assert!(s.is_dark());
    }

    #[test]
    fn cell_falls_when_half_nodes_dark() {
        assert!(!cell_is_dark(6, 13));   // < 50%
        assert!(cell_is_dark(7, 13));    // >= 50%
        assert!(!cell_is_dark(0, 0));    // empty cell not dark
    }
}
