//! nacharium-economy — L3 NARI economy (M3.2).
//!
//! NARI (Nacharium Resource Index) — "the blood of law". This crate holds the
//! pure economic logic: an append-only ledger where balance = sum of entries,
//! transfers, demurrage (value decays while held), escrow, and reputation
//! staking. Persistence lives in nacharium-storage; this is the domain core.
//!
//! Design: no ICO, no pre-mine. The only mint is the 13-token Genesis Grant
//! (GrantKind::Genesis13) and UBN. Everything else is transfer/fee/stake.

#![forbid(unsafe_code)]

pub mod escrow;
pub mod reputation;
pub mod marketplace;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// NARI amount in micro-NARI (6 decimals), like satoshis. Avoids float drift.
pub type MicroNari = i64;
pub const ONE_NARI: MicroNari = 1_000_000;
pub const GENESIS_GRANT: MicroNari = 13 * ONE_NARI;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EconError {
    #[error("insufficient balance: have {have}, need {need}")]
    Insufficient { have: MicroNari, need: MicroNari },
    #[error("amount must be positive")]
    NonPositive,
    #[error("genesis grant already issued to this account")]
    GrantAlreadyIssued,
}

/// The kind of a ledger entry — tracks WHY value moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Genesis13,
    Transfer,
    Tip,
    Fee,
    SpamTariff,
    Ubn,
    Demurrage,
    EscrowLock,
    EscrowRelease,
    EscrowRefund,
    Stake,
    Slash,
}

/// A single append-only ledger entry (never mutated once written).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub account: String,
    pub delta: MicroNari,
    pub kind: EntryKind,
    pub at: i64,
}

/// An in-memory ledger — balance is always the sum of entries (verifiable).
#[derive(Debug, Default)]
pub struct Ledger {
    entries: Vec<LedgerEntry>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Balance of an account = sum of its deltas. Never negative if all ops
    /// went through the guarded methods.
    pub fn balance(&self, account: &str) -> MicroNari {
        self.entries.iter().filter(|e| e.account == account).map(|e| e.delta).sum()
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }

    fn push(&mut self, account: &str, delta: MicroNari, kind: EntryKind, at: i64) {
        self.entries.push(LedgerEntry { account: account.to_string(), delta, kind, at });
    }

    /// Crate-internal escape hatch for submodules (escrow/reputation) that
    /// implement their own balanced double-entries.
    pub(crate) fn push_public(&mut self, account: &str, delta: MicroNari, kind: EntryKind, at: i64) {
        self.push(account, delta, kind, at);
    }

    /// Issue the one-time 13-NARI Genesis Grant. Idempotent per account.
    pub fn genesis_grant(&mut self, account: &str, at: i64) -> Result<MicroNari, EconError> {
        let already = self.entries.iter().any(|e| e.account == account && e.kind == EntryKind::Genesis13);
        if already {
            return Err(EconError::GrantAlreadyIssued);
        }
        self.push(account, GENESIS_GRANT, EntryKind::Genesis13, at);
        Ok(GENESIS_GRANT)
    }

    /// Transfer NARI from -> to. Balanced double entry.
    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        amount: MicroNari,
        at: i64,
    ) -> Result<(), EconError> {
        if amount <= 0 {
            return Err(EconError::NonPositive);
        }
        let have = self.balance(from);
        if have < amount {
            return Err(EconError::Insufficient { have, need: amount });
        }
        self.push(from, -amount, EntryKind::Transfer, at);
        self.push(to, amount, EntryKind::Transfer, at);
        Ok(())
    }

    /// Apply demurrage: value decays while held. `bps` = basis points of the
    /// current balance removed (e.g. 700 bps ~ Circles' 7%/yr per period).
    /// The removed value goes to the Treasury account (public goods).
    pub fn apply_demurrage(&mut self, account: &str, bps: i64, treasury: &str, at: i64) -> MicroNari {
        let bal = self.balance(account);
        if bal <= 0 || bps <= 0 {
            return 0;
        }
        let decay = bal * bps / 10_000;
        if decay == 0 {
            return 0;
        }
        self.push(account, -decay, EntryKind::Demurrage, at);
        self.push(treasury, decay, EntryKind::Demurrage, at);
        decay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_grant_is_13_nari_once() {
        let mut l = Ledger::new();
        assert_eq!(l.genesis_grant("did:a", 0).unwrap(), GENESIS_GRANT);
        assert_eq!(l.balance("did:a"), 13 * ONE_NARI);
        assert_eq!(l.genesis_grant("did:a", 1), Err(EconError::GrantAlreadyIssued));
    }

    #[test]
    fn transfer_moves_value_and_conserves_total() {
        let mut l = Ledger::new();
        l.genesis_grant("did:a", 0).unwrap();
        l.transfer("did:a", "did:b", 5 * ONE_NARI, 1).unwrap();
        assert_eq!(l.balance("did:a"), 8 * ONE_NARI);
        assert_eq!(l.balance("did:b"), 5 * ONE_NARI);
        // total conserved
        let total: MicroNari = l.entries().iter().map(|e| e.delta).sum();
        assert_eq!(total, 13 * ONE_NARI);
    }

    #[test]
    fn cannot_overspend() {
        let mut l = Ledger::new();
        l.genesis_grant("did:a", 0).unwrap();
        let r = l.transfer("did:a", "did:b", 100 * ONE_NARI, 1);
        assert!(matches!(r, Err(EconError::Insufficient { .. })));
    }

    #[test]
    fn demurrage_decays_holder_and_funds_treasury() {
        let mut l = Ledger::new();
        l.genesis_grant("did:a", 0).unwrap(); // 13 NARI
        let decayed = l.apply_demurrage("did:a", 700, "did:treasury", 1); // 7%
        assert!(decayed > 0);
        assert_eq!(l.balance("did:a"), 13 * ONE_NARI - decayed);
        assert_eq!(l.balance("did:treasury"), decayed);
    }
}
