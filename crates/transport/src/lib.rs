//! nacharium-transport — L1 P2P transport (B1.2.1).
//!
//! A libp2p host: TCP + Noise + Yamux, with gossipsub (pub/sub for Node/Cell
//! topics), identify, and ping. This is the "No King can Kill" fabric —
//! nodes talk peer-to-peer, no central server.

#![forbid(unsafe_code)]

pub mod node_types;
pub mod mesh;

use libp2p::{
    gossipsub, identify, ping,
    swarm::NetworkBehaviour,
};

/// The combined network behaviour for a Nacharium node.
#[derive(NetworkBehaviour)]
pub struct NacharuBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
}

/// Standard gossipsub topic for a Node's message stream.
pub fn node_topic(node_id: &str) -> gossipsub::IdentTopic {
    gossipsub::IdentTopic::new(format!("nch/node/{node_id}/v1"))
}

/// Standard gossipsub topic for a Cell.
pub fn cell_topic(cell_id: &str) -> gossipsub::IdentTopic {
    gossipsub::IdentTopic::new(format!("nch/cell/{cell_id}/v1"))
}

/// Standard gossipsub topic for a Sanctum's ciphertext stream.
pub fn sanctum_topic(sanctum_id: &str) -> gossipsub::IdentTopic {
    gossipsub::IdentTopic::new(format!("nch/sanctum/{sanctum_id}/v1"))
}

/// System-config replication topic (governance projection).
pub const SYSTEM_CONFIG_TOPIC: &str = "nch/system-config/v1";

pub use node_types::NodeType;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_naming_is_namespaced() {
        assert_eq!(node_topic("abc").to_string(), "nch/node/abc/v1");
        assert_eq!(cell_topic("c1").to_string(), "nch/cell/c1/v1");
        assert_eq!(sanctum_topic("s1").to_string(), "nch/sanctum/s1/v1");
    }
}
