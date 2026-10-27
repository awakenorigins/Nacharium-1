//! E1.2 — Civic Mask.
//!
//! A Nachatizen may wear several personas ("masks") in different Nodes, each
//! with a pairwise-unlinkable per-Node DID, yet all sharing ONE reputation
//! root. Observers cannot link two masks to the same core; only the owner can
//! prove they share a root when they choose to.
//!
//! Epoch-1 form: a mask DID is derived as blake3(root_secret | node_id). The
//! root_secret never leaves the owner. Two masks are unlinkable without the
//! secret, but the owner can produce a linking proof by revealing the secret
//! (or, in Epoch 2, a ZK proof of common root without revealing it).

use serde::{Deserialize, Serialize};

/// A per-Node persona derived from a hidden reputation root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CivicMask {
    pub node_id: String,
    /// The public mask DID shown in that Node.
    pub mask_did: String,
}

impl CivicMask {
    /// Derive a mask for `node_id` from the owner's secret root.
    pub fn derive(root_secret: &[u8], node_id: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(root_secret);
        hasher.update(b"|mask|");
        hasher.update(node_id.as_bytes());
        let did = format!("did:nch:mask:{}", hasher.finalize().to_hex());
        Self { node_id: node_id.into(), mask_did: did }
    }

    /// Prove two masks share the same reputation root (owner-initiated link).
    /// Returns true iff both masks derive from `root_secret`.
    pub fn share_root(a: &CivicMask, b: &CivicMask, root_secret: &[u8]) -> bool {
        CivicMask::derive(root_secret, &a.node_id) == *a
            && CivicMask::derive(root_secret, &b.node_id) == *b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_are_deterministic_per_node() {
        let s = b"root-secret-32-bytes-or-whatever";
        let m1 = CivicMask::derive(s, "node-A");
        let m1b = CivicMask::derive(s, "node-A");
        assert_eq!(m1, m1b);
    }

    #[test]
    fn different_nodes_yield_unlinkable_masks() {
        let s = b"root-secret";
        let a = CivicMask::derive(s, "node-A");
        let b = CivicMask::derive(s, "node-B");
        // The two mask DIDs share no structure an observer could link.
        assert_ne!(a.mask_did, b.mask_did);
    }

    #[test]
    fn owner_can_prove_common_root() {
        let s = b"root-secret";
        let a = CivicMask::derive(s, "node-A");
        let b = CivicMask::derive(s, "node-B");
        assert!(CivicMask::share_root(&a, &b, s));
    }

    #[test]
    fn wrong_secret_cannot_link() {
        let a = CivicMask::derive(b"real-root", "node-A");
        let b = CivicMask::derive(b"real-root", "node-B");
        assert!(!CivicMask::share_root(&a, &b, b"fake-root"));
    }
}
