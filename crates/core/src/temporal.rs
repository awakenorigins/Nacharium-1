//! E2.4 — Temporal Layers.
//!
//! Conversations flow through four time layers:
//!   The Now (real-time) -> The Slow (forum, ~7 days) -> The Deep (wiki,
//!   permanent) -> The Dream (sandbox, in Dreamtime).
//! A conversation ages forward Now -> Slow -> Deep as it matures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalLayer {
    Now,
    Slow,
    Deep,
    Dream,
}

impl TemporalLayer {
    /// The natural next layer as a conversation matures. Deep and Dream are terminal.
    pub fn mature(self) -> TemporalLayer {
        match self {
            TemporalLayer::Now => TemporalLayer::Slow,
            TemporalLayer::Slow => TemporalLayer::Deep,
            TemporalLayer::Deep => TemporalLayer::Deep,
            TemporalLayer::Dream => TemporalLayer::Dream,
        }
    }

    /// Retention window in seconds (0 = permanent).
    pub fn retention_secs(self) -> u64 {
        match self {
            TemporalLayer::Now => 60 * 60,          // 1 hour hot window
            TemporalLayer::Slow => 7 * 24 * 60 * 60, // 7 days
            TemporalLayer::Deep => 0,                // permanent
            TemporalLayer::Dream => 0,               // sandbox, cleared each Dreamtime externally
        }
    }

    /// Which layer a conversation of `age_secs` naturally belongs to.
    pub fn for_age(age_secs: u64) -> TemporalLayer {
        if age_secs <= TemporalLayer::Now.retention_secs() {
            TemporalLayer::Now
        } else if age_secs <= TemporalLayer::Slow.retention_secs() {
            TemporalLayer::Slow
        } else {
            TemporalLayer::Deep
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maturation_flows_now_slow_deep() {
        assert_eq!(TemporalLayer::Now.mature(), TemporalLayer::Slow);
        assert_eq!(TemporalLayer::Slow.mature(), TemporalLayer::Deep);
        assert_eq!(TemporalLayer::Deep.mature(), TemporalLayer::Deep);
    }

    #[test]
    fn deep_is_permanent() {
        assert_eq!(TemporalLayer::Deep.retention_secs(), 0);
        assert!(TemporalLayer::Now.retention_secs() < TemporalLayer::Slow.retention_secs());
    }

    #[test]
    fn age_maps_to_layer() {
        assert_eq!(TemporalLayer::for_age(30), TemporalLayer::Now);
        assert_eq!(TemporalLayer::for_age(2 * 24 * 60 * 60), TemporalLayer::Slow);
        assert_eq!(TemporalLayer::for_age(30 * 24 * 60 * 60), TemporalLayer::Deep);
    }
}
