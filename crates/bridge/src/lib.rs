//! nacharium-bridge — L5 Reality Bridge + Membrane + Portable Attestations.
//!
//! E2.2 The Membrane: a two-way interface between the real and the digital,
//! partitioned into zones with different rules:
//!   Physical (goods, strict) · Digital (data, fast) ·
//!   Financial (money, compliance) · Knowledge (ideas, freest).
//!
//! E2.3 Portable Attestations: a proof you can carry OUT to the real world
//! (reputation, job completion) WITHOUT revealing your Veil identity — only
//! the claim + a verifier signature, committed like the Reveal proof.

#![forbid(unsafe_code)]

pub mod attestation;

use serde::{Deserialize, Serialize};

/// The four Membrane zones, ordered from strictest to freest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembraneZone {
    Physical,
    Financial,
    Digital,
    Knowledge,
}

impl MembraneZone {
    /// Friction level 0..=100 (how much verification/compliance a crossing needs).
    pub fn friction(self) -> u8 {
        match self {
            MembraneZone::Physical => 80,   // goods: strict
            MembraneZone::Financial => 90,  // money: compliance-heavy
            MembraneZone::Digital => 30,    // data: fast
            MembraneZone::Knowledge => 5,   // ideas: freest
        }
    }

    /// Whether a crossing in this zone requires a compliance check.
    pub fn requires_compliance(self) -> bool {
        matches!(self, MembraneZone::Financial | MembraneZone::Physical)
    }
}

/// A crossing request through the Membrane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crossing {
    pub zone: MembraneZone,
    pub compliance_ok: bool,
}

impl Crossing {
    /// May this crossing proceed?
    pub fn allowed(&self) -> bool {
        !self.zone.requires_compliance() || self.compliance_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_is_freest_financial_strictest() {
        assert!(MembraneZone::Knowledge.friction() < MembraneZone::Digital.friction());
        assert!(MembraneZone::Digital.friction() < MembraneZone::Physical.friction());
        assert!(MembraneZone::Physical.friction() < MembraneZone::Financial.friction());
    }

    #[test]
    fn financial_crossing_needs_compliance() {
        let blocked = Crossing { zone: MembraneZone::Financial, compliance_ok: false };
        assert!(!blocked.allowed());
        let ok = Crossing { zone: MembraneZone::Financial, compliance_ok: true };
        assert!(ok.allowed());
    }

    #[test]
    fn knowledge_crossing_is_free() {
        let c = Crossing { zone: MembraneZone::Knowledge, compliance_ok: false };
        assert!(c.allowed());
    }
}
