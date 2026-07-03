//! nacharium-physics — L4 Social Physics (E1.4).
//!
//! Gravity = a Nachatizen's natural pull, from reputation + activity + value +
//! accumulated Resonance. NOT a follower count — a physical force. It has an
//! anti-monopoly CAP and DECAYS when you stop contributing.
//!
//! Resonance = the QUALITY of impact between two entities. It cannot be bought,
//! faked, or transferred — it only appears when two entities create real value
//! for each other. Every platform measures attention; Nacharium measures value.

#![forbid(unsafe_code)]

pub mod resonance;
pub mod weave;
pub mod harmony;

use serde::{Deserialize, Serialize};

/// Anti-monopoly ceiling: gravity is clamped so no one becomes a god.
pub const GRAVITY_CAP: f64 = 1000.0;

/// Inputs that build gravity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GravityInputs {
    /// Quality content that was cheered.
    pub content_cheers: f64,
    /// Successful marketplace jobs completed.
    pub jobs_done: f64,
    /// Help given to others (accumulated resonance).
    pub resonance: f64,
    /// Governance contributions.
    pub governance: f64,
}

/// Compute raw gravity from inputs (before decay/cap).
pub fn raw_gravity(i: &GravityInputs) -> f64 {
    // Weighted sum; resonance and jobs count most (real value > vanity).
    2.0 * i.resonance + 2.0 * i.jobs_done + 1.0 * i.content_cheers + 1.5 * i.governance
}

/// Apply the anti-monopoly cap.
pub fn capped(g: f64) -> f64 {
    g.min(GRAVITY_CAP).max(0.0)
}

/// Decay gravity for a period of inactivity. `periods` inactive, `rate` per
/// period in [0,1). Models "gravity must be MAINTAINED".
pub fn decay(g: f64, rate: f64, periods: u32) -> f64 {
    let r = rate.clamp(0.0, 0.99);
    g * (1.0 - r).powi(periods as i32)
}

/// Full gravity of an entity: raw -> capped, then decayed by inactivity.
pub fn gravity(inputs: &GravityInputs, decay_rate: f64, inactive_periods: u32) -> f64 {
    let g = capped(raw_gravity(inputs));
    decay(g, decay_rate, inactive_periods)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_rewards_real_value_over_vanity() {
        // 10 resonance (real help) beats 10 content cheers (vanity).
        let real = raw_gravity(&GravityInputs { resonance: 10.0, ..Default::default() });
        let vanity = raw_gravity(&GravityInputs { content_cheers: 10.0, ..Default::default() });
        assert!(real > vanity);
    }

    #[test]
    fn gravity_is_capped_anti_monopoly() {
        let huge = GravityInputs { jobs_done: 100000.0, ..Default::default() };
        assert_eq!(capped(raw_gravity(&huge)), GRAVITY_CAP);
    }

    #[test]
    fn gravity_decays_with_inactivity() {
        let g0 = 100.0;
        let g_after = decay(g0, 0.1, 5); // 10% decay, 5 idle periods
        assert!(g_after < g0);
        assert!(g_after > 0.0);
    }

    #[test]
    fn active_entity_keeps_more_gravity_than_idle() {
        let inputs = GravityInputs { resonance: 50.0, jobs_done: 20.0, ..Default::default() };
        let active = gravity(&inputs, 0.1, 0);
        let idle = gravity(&inputs, 0.1, 10);
        assert!(active > idle);
    }
}
