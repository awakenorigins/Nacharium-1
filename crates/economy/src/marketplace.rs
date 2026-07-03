//! E1.3 — Marketplace + Skill Bazaar.
//!
//! Listings are indexed by CAPABILITY, not by who you are (flat world). To
//! sell, a Nachatizen must stake reputation (fraud => slash). Every trade
//! defaults to escrow ("escrow is the constitution of commerce"). Works for
//! human<->AI and AI<->human alike.

use crate::escrow::Escrow;
use crate::reputation::Stake;
use crate::{EconError, Ledger, MicroNari};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A capability-indexed listing on the Skill Bazaar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: Uuid,
    pub seller_did: String,
    /// The capability offered, e.g. "translate:vi-en", "code-review:rust".
    pub capability: String,
    pub price: MicroNari,
    /// Reputation the seller staked to be allowed to list.
    pub stake: MicroNari,
}

/// An order against a listing, held in escrow until delivery is confirmed.
#[derive(Debug, Clone)]
pub struct Order {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub buyer_did: String,
    pub escrow: Escrow,
}

impl Listing {
    /// Create a listing — REQUIRES a reputation stake (locked from balance).
    pub fn create(
        ledger: &mut Ledger,
        seller_did: &str,
        capability: &str,
        price: MicroNari,
        stake_amount: MicroNari,
        at: i64,
    ) -> Result<(Self, Stake), EconError> {
        let stake = Stake::place(ledger, seller_did, stake_amount, at)?;
        let listing = Self {
            id: Uuid::new_v4(),
            seller_did: seller_did.into(),
            capability: capability.into(),
            price,
            stake: stake_amount,
        };
        Ok((listing, stake))
    }

    /// A buyer orders — funds go into escrow (constitution of commerce).
    pub fn order(&self, ledger: &mut Ledger, buyer_did: &str, at: i64) -> Result<Order, EconError> {
        let escrow = Escrow::open(ledger, buyer_did, &self.seller_did, self.price, at)?;
        Ok(Order { id: Uuid::new_v4(), listing_id: self.id, buyer_did: buyer_did.into(), escrow })
    }
}

impl Order {
    /// Buyer confirms delivery -> escrow releases to seller.
    pub fn confirm_delivery(&mut self, ledger: &mut Ledger, at: i64) {
        self.escrow.release(ledger, at);
    }

    /// Dispute resolved for buyer -> escrow refunds.
    pub fn refund(&mut self, ledger: &mut Ledger, at: i64) {
        self.escrow.refund(ledger, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ONE_NARI;

    fn ledger_with(seller_bal_grants: &[&str]) -> Ledger {
        let mut l = Ledger::new();
        for who in seller_bal_grants {
            l.genesis_grant(who, 0).unwrap();
        }
        l
    }

    #[test]
    fn listing_requires_stake_and_locks_it() {
        let mut l = ledger_with(&["seller"]);
        let (listing, _s) = Listing::create(&mut l, "seller", "translate:vi-en", 4 * ONE_NARI, 2 * ONE_NARI, 1).unwrap();
        assert_eq!(listing.capability, "translate:vi-en");
        // stake locked away from spendable balance
        assert_eq!(l.balance("seller"), 11 * ONE_NARI);
        assert_eq!(l.balance("stake:seller"), 2 * ONE_NARI);
    }

    #[test]
    fn order_escrows_then_delivery_pays_seller() {
        let mut l = ledger_with(&["seller", "buyer"]);
        let (listing, _s) = Listing::create(&mut l, "seller", "code-review:rust", 5 * ONE_NARI, 1 * ONE_NARI, 1).unwrap();
        let mut order = listing.order(&mut l, "buyer", 2).unwrap();
        assert_eq!(l.balance("buyer"), 8 * ONE_NARI); // 5 locked in escrow
        order.confirm_delivery(&mut l, 3);
        assert_eq!(l.balance("seller"), 13 * ONE_NARI - 1 * ONE_NARI + 5 * ONE_NARI); // grant - stake + sale
    }

    #[test]
    fn cannot_list_without_enough_stake() {
        let mut l = ledger_with(&["poor"]);
        let r = Listing::create(&mut l, "poor", "x", ONE_NARI, 100 * ONE_NARI, 1);
        assert!(matches!(r, Err(EconError::Insufficient { .. })));
    }

    #[test]
    fn disputed_order_refunds_buyer() {
        let mut l = ledger_with(&["seller", "buyer"]);
        let (listing, _s) = Listing::create(&mut l, "seller", "x", 5 * ONE_NARI, ONE_NARI, 1).unwrap();
        let mut order = listing.order(&mut l, "buyer", 2).unwrap();
        order.refund(&mut l, 3);
        assert_eq!(l.balance("buyer"), 13 * ONE_NARI);
    }
}
