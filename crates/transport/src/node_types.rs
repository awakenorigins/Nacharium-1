//! B1.2.4 — Node type handlers.
//!
//! 5 node types from the blueprint, each with different resource profile
//! and capabilities. Edge = PicoClaw ($10, phone/BLE/LoRa). Mula = governance
//! (measures Harmony, no veto power — Protocol 3: No God).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Full node: server, relay, cache, stores blobs.
    Sovereign,
    /// Light node: phone/laptop, partial state.
    Citizen,
    /// PicoClaw edge: minimal, mesh-capable, offline-first.
    Edge,
    /// Provides real-world data (Reality Bridge).
    Oracle,
    /// Governance node: measures Harmony. No veto power.
    Mula,
}

impl NodeType {
    /// Whether this node type can act as a relay for others.
    pub fn is_relay(self) -> bool {
        matches!(self, NodeType::Sovereign)
    }

    /// Whether this node type supports offline mesh (Reticulum/LoRa/BLE).
    pub fn supports_mesh(self) -> bool {
        matches!(self, NodeType::Edge)
    }

    /// Whether this node participates in governance measurement.
    /// Note: measuring Harmony is NOT the same as veto power.
    pub fn measures_harmony(self) -> bool {
        matches!(self, NodeType::Mula)
    }

    /// No node type — not even Mula — holds absolute power (Protocol 3).
    pub fn has_veto_power(self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sovereign_relays_edge_meshes() {
        assert!(NodeType::Sovereign.is_relay());
        assert!(!NodeType::Edge.is_relay());
        assert!(NodeType::Edge.supports_mesh());
    }

    #[test]
    fn mula_measures_but_cannot_veto() {
        assert!(NodeType::Mula.measures_harmony());
        // Protocol 3: No God — no node has absolute power.
        assert!(!NodeType::Mula.has_veto_power());
        for t in [NodeType::Sovereign, NodeType::Citizen, NodeType::Edge, NodeType::Oracle, NodeType::Mula] {
            assert!(!t.has_veto_power(), "{t:?} must not have veto power");
        }
    }
}
