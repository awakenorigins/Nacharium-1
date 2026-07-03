//! E3.1 — DAO Super Chorus.
//!
//! Node-level decisions use a simple Chorus. Cell-level uses the weighted
//! Chorus. But changes touching the PROTOCOL-adjacent config (the closest a
//! governed system gets to the constitution) require a SUPER CHORUS: a higher
//! harmonic threshold AND a supermajority of participating stake-weight AND a
//! larger quorum. This is how the DAO governs itself without letting a simple
//! majority reach toward the immutable 7 Protocols.

use crate::Voice;

/// Governance tier for a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovTier {
    Node,
    Cell,
    /// Protocol-adjacent config — requires Super Chorus.
    Protocol,
}

impl GovTier {
    /// Minimum harmonic point (0..=100) to pass at this tier.
    pub fn harmonic_threshold(self) -> u8 {
        match self {
            GovTier::Node => 50,
            GovTier::Cell => 60,
            GovTier::Protocol => 75,
        }
    }

    /// Minimum number of voices (quorum) at this tier.
    pub fn quorum(self) -> usize {
        match self {
            GovTier::Node => 3,
            GovTier::Cell => 7,
            GovTier::Protocol => 13, // one full Cell
        }
    }

    /// Required supermajority fraction of agreeing stake-weight (Protocol only).
    pub fn supermajority(self) -> f64 {
        match self {
            GovTier::Node => 0.5,
            GovTier::Cell => 0.6,
            GovTier::Protocol => 2.0 / 3.0,
        }
    }
}

/// Fraction of total stake-weight whose position is at/above the threshold.
fn agreeing_fraction(voices: &[Voice], threshold: u8) -> f64 {
    let total: f64 = voices.iter().map(|v| v.stake as f64 * v.confidence.clamp(0.0, 1.0)).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let agree: f64 = voices
        .iter()
        .filter(|v| v.position >= threshold)
        .map(|v| v.stake as f64 * v.confidence.clamp(0.0, 1.0))
        .sum();
    agree / total
}

/// Does a proposal pass at the given governance tier?
/// Requires: harmonic_point >= tier threshold, quorum met, AND (for the
/// supermajority) enough agreeing stake-weight.
pub fn super_chorus_passes(
    voices: &[Voice],
    harmonic_point: Option<u8>,
    tier: GovTier,
) -> bool {
    let thr = tier.harmonic_threshold();
    let hp_ok = matches!(harmonic_point, Some(hp) if hp >= thr);
    let quorum_ok = voices.len() >= tier.quorum();
    let super_ok = agreeing_fraction(voices, thr) >= tier.supermajority();
    hp_ok && quorum_ok && super_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(pos: u8, stake: u64) -> Voice {
        Voice { voter_did: "x".into(), position: pos, confidence: 1.0, stake, reasoning: String::new() }
    }

    #[test]
    fn protocol_tier_is_strictest() {
        assert!(GovTier::Protocol.harmonic_threshold() > GovTier::Cell.harmonic_threshold());
        assert!(GovTier::Protocol.quorum() > GovTier::Cell.quorum());
        assert!(GovTier::Protocol.supermajority() > GovTier::Cell.supermajority());
    }

    #[test]
    fn protocol_change_needs_supermajority_and_quorum() {
        // 13 strong voices at 90 -> passes protocol tier.
        let voices: Vec<Voice> = (0..13).map(|_| v(90, 10)).collect();
        assert!(super_chorus_passes(&voices, Some(90), GovTier::Protocol));
    }

    #[test]
    fn protocol_change_fails_below_quorum() {
        let voices: Vec<Voice> = (0..5).map(|_| v(90, 10)).collect();
        assert!(!super_chorus_passes(&voices, Some(90), GovTier::Protocol));
    }

    #[test]
    fn simple_majority_cannot_pass_protocol_tier() {
        // 13 voices but only ~55% stake agrees -> below 2/3 supermajority.
        let mut voices: Vec<Voice> = (0..7).map(|_| v(90, 10)).collect();
        voices.extend((0..6).map(|_| v(10, 10)));
        // harmonic point may be high, but supermajority fraction ~0.54 < 0.667
        assert!(!super_chorus_passes(&voices, Some(90), GovTier::Protocol));
    }
}
