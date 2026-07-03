//! E3.5 — Harmony score + Dreamtime.
//!
//! Harmony = the ecosystem's health, a weighted blend of four sub-scores:
//!   network 30% · economy 25% · social 25% · resource 20%.
//! Dreamtime = the rest window each Nacharium Day: AI consolidates, humans
//! disconnect; Deep Dreams (monthly 24h) allow major upgrades.

use serde::{Deserialize, Serialize};

/// The four health inputs, each in [0,1].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarmonyInputs {
    pub network: f64,
    pub economy: f64,
    pub social: f64,
    pub resource: f64,
}

/// Weighted Harmony score in [0,1].
pub fn harmony(i: &HarmonyInputs) -> f64 {
    let c = |x: f64| x.clamp(0.0, 1.0);
    0.30 * c(i.network) + 0.25 * c(i.economy) + 0.25 * c(i.social) + 0.20 * c(i.resource)
}

/// Whether the network is currently in Dreamtime given the second-of-day.
/// Dreamtime = the final hour of each Nacharium Day (a daily rest window).
pub fn is_dreamtime(second_of_day: u32) -> bool {
    let day = 24 * 60 * 60;
    second_of_day % day >= day - 3600
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harmony_weights_sum_and_bound() {
        let perfect = HarmonyInputs { network: 1.0, economy: 1.0, social: 1.0, resource: 1.0 };
        assert!((harmony(&perfect) - 1.0).abs() < 1e-9);
        let zero = HarmonyInputs::default();
        assert_eq!(harmony(&zero), 0.0);
    }

    #[test]
    fn network_has_highest_weight() {
        let net = HarmonyInputs { network: 1.0, ..Default::default() };
        let res = HarmonyInputs { resource: 1.0, ..Default::default() };
        assert!(harmony(&net) > harmony(&res));
    }

    #[test]
    fn dreamtime_is_last_hour_of_day() {
        assert!(is_dreamtime(24 * 60 * 60 - 100)); // last hour
        assert!(!is_dreamtime(12 * 60 * 60));       // midday
    }
}
