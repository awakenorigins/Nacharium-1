//! nacharium-chorus — L4 governance + justice.
//!
//! The Chorus (E1.6): instead of yes/no voting, Nachatizens "sing" a position
//! on a 0..=100 spectrum with confidence + stake + reasoning. The system finds
//! the HARMONIC POINT — a stake-and-confidence-weighted median. Nacharium does
//! not need a majority; it needs harmony.
//!
//! Jury Pool (E1.5): disputes are resolved by a randomly-drawn jury (VRF-seeded
//! from the dispute id) of eligible Nachatizens (reputation > threshold). The
//! verdict is the majority of jurors; penalties range up to Exile.

#![forbid(unsafe_code)]

pub mod jury;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One entity's "sung" position in a Chorus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub voter_did: String,
    /// Position on the 0..=100 spectrum.
    pub position: u8,
    /// Confidence in [0,1].
    pub confidence: f64,
    /// Stake (NARI micro) backing this voice.
    pub stake: u64,
    pub reasoning: String,
}

/// A governance proposal being sung on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub config_path: String,
    pub voices: Vec<Voice>,
}

impl Proposal {
    pub fn new(config_path: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), config_path: config_path.into(), voices: Vec::new() }
    }

    pub fn sing(&mut self, v: Voice) {
        self.voices.push(v);
    }

    /// The Harmonic Point: the position at which cumulative (stake * confidence)
    /// weight crosses half of the total weight — a weighted median. Robust to a
    /// loud low-stake minority; rewards conviction backed by stake.
    pub fn harmonic_point(&self) -> Option<u8> {
        if self.voices.is_empty() {
            return None;
        }
        let mut weighted: Vec<(u8, f64)> = self
            .voices
            .iter()
            .map(|v| (v.position, (v.stake as f64) * v.confidence.clamp(0.0, 1.0)))
            .filter(|(_, w)| *w > 0.0)
            .collect();
        if weighted.is_empty() {
            // fall back to unweighted median of positions
            let mut ps: Vec<u8> = self.voices.iter().map(|v| v.position).collect();
            ps.sort_unstable();
            return Some(ps[ps.len() / 2]);
        }
        weighted.sort_by(|a, b| a.0.cmp(&b.0));
        let total: f64 = weighted.iter().map(|(_, w)| *w).sum();
        let mut cum = 0.0;
        for (pos, w) in &weighted {
            cum += *w;
            if cum * 2.0 >= total {
                return Some(*pos);
            }
        }
        weighted.last().map(|(p, _)| *p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(did: &str, pos: u8, conf: f64, stake: u64) -> Voice {
        Voice { voter_did: did.into(), position: pos, confidence: conf, stake, reasoning: String::new() }
    }

    #[test]
    fn empty_has_no_harmonic_point() {
        assert_eq!(Proposal::new("x").harmonic_point(), None);
    }

    #[test]
    fn harmonic_point_is_stake_weighted() {
        let mut p = Proposal::new("economy.fee");
        // A loud low-stake crowd at 10, one high-stake conviction at 80.
        p.sing(voice("a", 10, 1.0, 1));
        p.sing(voice("b", 10, 1.0, 1));
        p.sing(voice("c", 80, 1.0, 100));
        // Weighted median should be pulled to the high-stake voice.
        assert_eq!(p.harmonic_point(), Some(80));
    }

    #[test]
    fn equal_stakes_give_median() {
        let mut p = Proposal::new("x");
        p.sing(voice("a", 20, 1.0, 10));
        p.sing(voice("b", 50, 1.0, 10));
        p.sing(voice("c", 80, 1.0, 10));
        assert_eq!(p.harmonic_point(), Some(50));
    }

    #[test]
    fn zero_weight_falls_back_to_median() {
        let mut p = Proposal::new("x");
        p.sing(voice("a", 30, 0.0, 0));
        p.sing(voice("b", 70, 0.0, 0));
        p.sing(voice("c", 40, 0.0, 0));
        assert!(p.harmonic_point().is_some());
    }
}
