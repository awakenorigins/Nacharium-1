//! nacharium-nodecell — L2 Node/Cell registry + tiered rules engine.
//!
//! Node = self-governing community with its own rules. 13 Nodes = 1 Cell.
//! Rules hierarchy (a rule may never violate the tier above it):
//!   Protocol (immutable) -> Cell Rules -> Node Rules -> Zoner Rules

#![forbid(unsafe_code)]

use nacharium_transport::NodeType;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Exactly 13 Nodes form a Cell (matches the 13-token Genesis Grant).
pub const NODES_PER_CELL: usize = 13;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NodeCellError {
    #[error("cell is full: a Cell holds exactly 13 Nodes")]
    CellFull,
    #[error("node already in this cell")]
    AlreadyMember,
    #[error("rule violates a higher tier and is rejected")]
    RuleViolatesHigherTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub name: String,
    pub node_type: NodeType,
    pub cell_id: Option<Uuid>,
    /// Node-level rules (must not violate Cell/Protocol).
    pub rules: Vec<Rule>,
}

impl Node {
    pub fn new(name: impl Into<String>, node_type: NodeType) -> Self {
        Self { id: Uuid::new_v4(), name: name.into(), node_type, cell_id: None, rules: Vec::new() }
    }
}

/// Rule tiers, highest authority first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    /// Immutable — the 7 Protocols. Nothing may override.
    Protocol = 0,
    Cell = 1,
    Node = 2,
    Zoner = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub tier: Tier,
    pub key: String,
    /// A rule "forbids" some capability key; lower tiers cannot re-allow
    /// what a higher tier forbids.
    pub forbids: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub id: Uuid,
    pub name: String,
    pub node_ids: Vec<Uuid>,
    pub rules: Vec<Rule>,
}

impl Cell {
    pub fn new(name: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), name: name.into(), node_ids: Vec::new(), rules: Vec::new() }
    }

    /// Add a node to the cell. Enforces the 13-node ceiling.
    pub fn add_node(&mut self, node: &mut Node) -> Result<(), NodeCellError> {
        if self.node_ids.len() >= NODES_PER_CELL {
            return Err(NodeCellError::CellFull);
        }
        if self.node_ids.contains(&node.id) {
            return Err(NodeCellError::AlreadyMember);
        }
        self.node_ids.push(node.id);
        node.cell_id = Some(self.id);
        Ok(())
    }

    pub fn is_full(&self) -> bool {
        self.node_ids.len() >= NODES_PER_CELL
    }
}

/// Tiered rules engine: check whether a lower-tier rule is allowed given the
/// higher-tier rules already in force. A lower tier may NOT re-allow a
/// capability that a higher tier forbids.
pub fn rule_allowed(higher_tiers: &[Rule], proposed: &Rule) -> bool {
    // If any higher tier forbids this key, a lower tier trying to allow it fails.
    if !proposed.forbids {
        let forbidden_above = higher_tiers
            .iter()
            .any(|r| r.key == proposed.key && r.forbids && r.tier < proposed.tier);
        if forbidden_above {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_holds_exactly_13_nodes() {
        let mut cell = Cell::new("first-cell");
        for i in 0..NODES_PER_CELL {
            let mut n = Node::new(format!("node-{i}"), NodeType::Citizen);
            assert!(cell.add_node(&mut n).is_ok());
        }
        assert!(cell.is_full());
        let mut overflow = Node::new("node-14", NodeType::Citizen);
        assert_eq!(cell.add_node(&mut overflow), Err(NodeCellError::CellFull));
    }

    #[test]
    fn node_joins_and_records_cell_id() {
        let mut cell = Cell::new("c");
        let mut n = Node::new("n", NodeType::Sovereign);
        cell.add_node(&mut n).unwrap();
        assert_eq!(n.cell_id, Some(cell.id));
    }

    #[test]
    fn lower_tier_cannot_reallow_what_higher_forbids() {
        let higher = vec![Rule { tier: Tier::Cell, key: "sell_service".into(), forbids: true }];
        // A Node (lower tier) tries to ALLOW selling — must be rejected.
        let node_allow = Rule { tier: Tier::Node, key: "sell_service".into(), forbids: false };
        assert!(!rule_allowed(&higher, &node_allow));
        // A Node forbidding something new is fine.
        let node_forbid = Rule { tier: Tier::Node, key: "spam".into(), forbids: true };
        assert!(rule_allowed(&higher, &node_forbid));
    }

    #[test]
    fn tier_ordering_protocol_is_highest() {
        assert!(Tier::Protocol < Tier::Cell);
        assert!(Tier::Cell < Tier::Node);
        assert!(Tier::Node < Tier::Zoner);
    }
}
