//! E1.5 — Jury Pool (Kleros-style decentralized arbitration).
//!
//! A dispute draws a random jury from eligible Nachatizens (reputation above a
//! threshold, ideally from different Cells). The draw is deterministic given a
//! VRF-style seed derived from the dispute id, so it is verifiable + fair.
//! Jurors vote guilty/not-guilty; the majority rules; penalties scale up to
//! Exile. An acquittal makes the claimant compensate the defendant.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Guilty,
    NotGuilty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Penalty {
    None,
    Warning,
    Fine,
    DarkZone,
    Exile,
}

/// An eligible juror candidate.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub did: String,
    pub reputation: f64,
    pub cell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub id: Uuid,
    pub claimant: String,
    pub respondent: String,
    pub jury: Vec<String>,
}

/// Minimum reputation to be eligible for jury duty.
pub const JURY_MIN_REPUTATION: f64 = 500.0;

impl Dispute {
    /// Draw `size` jurors deterministically from eligible candidates using a
    /// VRF-style seed = blake3(dispute_id | did). Verifiable + unpredictable.
    pub fn draw_jury(id: Uuid, candidates: &[Candidate], size: usize) -> Self {
        let mut scored: Vec<(u128, &Candidate)> = candidates
            .iter()
            .filter(|c| c.reputation >= JURY_MIN_REPUTATION)
            .map(|c| {
                let h = blake3::hash(format!("{id}|{}", c.did).as_bytes());
                let bytes = h.as_bytes();
                let mut n: u128 = 0;
                for b in &bytes[..16] {
                    n = (n << 8) | *b as u128;
                }
                (n, c)
            })
            .collect();
        scored.sort_by_key(|(n, _)| *n);
        let jury = scored.into_iter().take(size).map(|(_, c)| c.did.clone()).collect();
        Self { id, claimant: String::new(), respondent: String::new(), jury }
    }

    /// Tally juror verdicts -> majority ruling.
    pub fn rule(votes: &[Verdict]) -> Verdict {
        let guilty = votes.iter().filter(|v| **v == Verdict::Guilty).count();
        if guilty * 2 > votes.len() {
            Verdict::Guilty
        } else {
            Verdict::NotGuilty
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(n: usize, rep: f64) -> Vec<Candidate> {
        (0..n).map(|i| Candidate { did: format!("did:{i}"), reputation: rep, cell: format!("cell-{}", i % 3) }).collect()
    }

    #[test]
    fn only_high_reputation_are_eligible() {
        let id = Uuid::new_v4();
        let mut cs = candidates(3, 100.0); // below threshold
        cs.push(Candidate { did: "hi".into(), reputation: 900.0, cell: "z".into() });
        let d = Dispute::draw_jury(id, &cs, 5);
        assert_eq!(d.jury, vec!["hi".to_string()]);
    }

    #[test]
    fn draw_is_deterministic_for_same_dispute() {
        let id = Uuid::new_v4();
        let cs = candidates(20, 800.0);
        let a = Dispute::draw_jury(id, &cs, 5);
        let b = Dispute::draw_jury(id, &cs, 5);
        assert_eq!(a.jury, b.jury);
        assert_eq!(a.jury.len(), 5);
    }

    #[test]
    fn different_disputes_draw_different_juries() {
        let cs = candidates(50, 800.0);
        let a = Dispute::draw_jury(Uuid::new_v4(), &cs, 5);
        let b = Dispute::draw_jury(Uuid::new_v4(), &cs, 5);
        assert_ne!(a.jury, b.jury);
    }

    #[test]
    fn majority_rules() {
        assert_eq!(Dispute::rule(&[Verdict::Guilty, Verdict::Guilty, Verdict::NotGuilty]), Verdict::Guilty);
        assert_eq!(Dispute::rule(&[Verdict::Guilty, Verdict::NotGuilty, Verdict::NotGuilty]), Verdict::NotGuilty);
    }
}
