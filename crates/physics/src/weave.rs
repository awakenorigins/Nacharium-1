//! E3.3 — The Weave.
//!
//! Every interaction is a thread; many threads make a living fabric. The Weave
//! is the graph: nodes = Nachatizens, edges = resonance strength. The Cell Map
//! is a real-time visualization of it. This module builds the graph and
//! computes simple structural signals (density, most-woven node).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The living fabric: an undirected weighted graph of resonance.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Weave {
    edges: BTreeMap<String, f64>, // "a|b" (sorted) -> weight
    nodes: BTreeSet<String>,
}

fn key(a: &str, b: &str) -> String {
    if a <= b { format!("{a}|{b}") } else { format!("{b}|{a}") }
}

impl Weave {
    pub fn new() -> Self {
        Self::default()
    }

    /// Weave a thread (or strengthen it) between two Nachatizens.
    pub fn weave(&mut self, a: &str, b: &str, resonance: f64) {
        if a == b || resonance <= 0.0 {
            return;
        }
        self.nodes.insert(a.to_string());
        self.nodes.insert(b.to_string());
        *self.edges.entry(key(a, b)).or_insert(0.0) += resonance;
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Graph density in [0,1]: edges / max-possible-edges.
    pub fn density(&self) -> f64 {
        let n = self.nodes.len();
        if n < 2 {
            return 0.0;
        }
        let max = (n * (n - 1)) / 2;
        self.edges.len() as f64 / max as f64
    }

    /// The most-woven node = highest total incident resonance (a Gravity Well).
    pub fn most_woven(&self) -> Option<String> {
        let mut totals: BTreeMap<&str, f64> = BTreeMap::new();
        for (k, w) in &self.edges {
            let (a, b) = k.split_once('|').unwrap();
            *totals.entry(a).or_insert(0.0) += *w;
            *totals.entry(b).or_insert(0.0) += *w;
        }
        totals.into_iter().max_by(|x, y| x.1.partial_cmp(&y.1).unwrap()).map(|(n, _)| n.to_string())
    }

    /// Export as a Cell Map (nodes + edges) for visualization clients.
    pub fn cell_map(&self) -> serde_json::Value {
        let nodes: Vec<&String> = self.nodes.iter().collect();
        let edges: Vec<serde_json::Value> = self
            .edges
            .iter()
            .map(|(k, w)| {
                let (a, b) = k.split_once('|').unwrap();
                serde_json::json!({ "source": a, "target": b, "weight": w })
            })
            .collect();
        serde_json::json!({ "nodes": nodes, "edges": edges })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weaving_builds_the_fabric() {
        let mut w = Weave::new();
        w.weave("a", "b", 1.0);
        w.weave("a", "c", 2.0);
        assert_eq!(w.node_count(), 3);
        assert_eq!(w.edge_count(), 2);
    }

    #[test]
    fn most_woven_is_the_hub() {
        let mut w = Weave::new();
        w.weave("hub", "x", 5.0);
        w.weave("hub", "y", 5.0);
        w.weave("x", "y", 1.0);
        assert_eq!(w.most_woven(), Some("hub".to_string()));
    }

    #[test]
    fn density_full_graph_is_one() {
        let mut w = Weave::new();
        w.weave("a", "b", 1.0);
        w.weave("a", "c", 1.0);
        w.weave("b", "c", 1.0);
        assert!((w.density() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cell_map_exports_nodes_and_edges() {
        let mut w = Weave::new();
        w.weave("a", "b", 1.0);
        let m = w.cell_map();
        assert_eq!(m["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(m["edges"].as_array().unwrap().len(), 1);
    }
}
