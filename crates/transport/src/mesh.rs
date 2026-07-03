//! E3.4 — Mesh scale + survival routing.
//!
//! When the main network path is gone, messages find their own way (No King
//! can Kill). This models a Reticulum/LoRa-style store-and-forward mesh: a
//! message carries a hop budget; each Edge node forwards it to reachable peers
//! until it arrives or the budget is exhausted. Offline messages queue until a
//! peer reappears.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// A survival-routed message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPacket {
    pub id: String,
    pub dest: String,
    pub hops_left: u8,
}

/// A tiny mesh: adjacency + per-node offline queues.
#[derive(Debug, Default)]
pub struct Mesh {
    adj: BTreeMap<String, Vec<String>>,
    queues: BTreeMap<String, VecDeque<MeshPacket>>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a bidirectional mesh link.
    pub fn link(&mut self, a: &str, b: &str) {
        self.adj.entry(a.to_string()).or_default().push(b.to_string());
        self.adj.entry(b.to_string()).or_default().push(a.to_string());
    }

    /// Try to deliver from `start` to `packet.dest` via BFS within the hop
    /// budget. Returns true if a path within budget exists (survival routing).
    pub fn deliver(&self, start: &str, packet: &MeshPacket) -> bool {
        if start == packet.dest {
            return true;
        }
        let mut seen = vec![start.to_string()];
        let mut frontier: VecDeque<(String, u8)> = VecDeque::new();
        frontier.push_back((start.to_string(), packet.hops_left));
        while let Some((node, budget)) = frontier.pop_front() {
            if budget == 0 {
                continue;
            }
            if let Some(peers) = self.adj.get(&node) {
                for p in peers {
                    if p == &packet.dest {
                        return true;
                    }
                    if !seen.contains(p) {
                        seen.push(p.clone());
                        frontier.push_back((p.clone(), budget - 1));
                    }
                }
            }
        }
        false
    }

    /// Queue a packet for an offline peer; drained when it reappears.
    pub fn queue_offline(&mut self, peer: &str, packet: MeshPacket) {
        self.queues.entry(peer.to_string()).or_default().push_back(packet);
    }

    pub fn drain(&mut self, peer: &str) -> Vec<MeshPacket> {
        self.queues.remove(peer).map(|q| q.into_iter().collect()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(dest: &str, hops: u8) -> MeshPacket {
        MeshPacket { id: "p1".into(), dest: dest.into(), hops_left: hops }
    }

    #[test]
    fn message_finds_multi_hop_path() {
        let mut m = Mesh::new();
        m.link("a", "b");
        m.link("b", "c");
        m.link("c", "d");
        assert!(m.deliver("a", &pkt("d", 3))); // a->b->c->d within budget
    }

    #[test]
    fn fails_when_hop_budget_too_small() {
        let mut m = Mesh::new();
        m.link("a", "b");
        m.link("b", "c");
        m.link("c", "d");
        assert!(!m.deliver("a", &pkt("d", 1))); // not enough hops
    }

    #[test]
    fn offline_queue_holds_then_drains() {
        let mut m = Mesh::new();
        m.queue_offline("edge1", pkt("x", 5));
        m.queue_offline("edge1", pkt("y", 5));
        let drained = m.drain("edge1");
        assert_eq!(drained.len(), 2);
        assert!(m.drain("edge1").is_empty()); // emptied
    }
}
