//! M3.2 — Escrow: "the constitution of commerce". Transactions default to
//! escrow. Funds are locked, then released to payee or refunded to payer.

use crate::{EconError, Ledger, MicroNari, EntryKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscrowState {
    Locked,
    Released,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escrow {
    pub id: Uuid,
    pub payer: String,
    pub payee: String,
    pub amount: MicroNari,
    pub state: EscrowState,
}

impl Escrow {
    /// Open an escrow by locking `amount` out of the payer's balance.
    /// The locked value is held in a synthetic escrow account (the escrow id).
    pub fn open(
        ledger: &mut Ledger,
        payer: &str,
        payee: &str,
        amount: MicroNari,
        at: i64,
    ) -> Result<Self, EconError> {
        if amount <= 0 {
            return Err(EconError::NonPositive);
        }
        let have = ledger.balance(payer);
        if have < amount {
            return Err(EconError::Insufficient { have, need: amount });
        }
        let id = Uuid::new_v4();
        let esc = format!("escrow:{id}");
        ledger_push(ledger, payer, -amount, EntryKind::EscrowLock, at);
        ledger_push(ledger, &esc, amount, EntryKind::EscrowLock, at);
        Ok(Self { id, payer: payer.into(), payee: payee.into(), amount, state: EscrowState::Locked })
    }

    /// Release the escrow to the payee.
    pub fn release(&mut self, ledger: &mut Ledger, at: i64) {
        if self.state != EscrowState::Locked {
            return;
        }
        let esc = format!("escrow:{}", self.id);
        ledger_push(ledger, &esc, -self.amount, EntryKind::EscrowRelease, at);
        ledger_push(ledger, &self.payee, self.amount, EntryKind::EscrowRelease, at);
        self.state = EscrowState::Released;
    }

    /// Refund the escrow back to the payer.
    pub fn refund(&mut self, ledger: &mut Ledger, at: i64) {
        if self.state != EscrowState::Locked {
            return;
        }
        let esc = format!("escrow:{}", self.id);
        ledger_push(ledger, &esc, -self.amount, EntryKind::EscrowRefund, at);
        ledger_push(ledger, &self.payer, self.amount, EntryKind::EscrowRefund, at);
        self.state = EscrowState::Refunded;
    }
}

// Small helper to push through the ledger's internal API via public methods.
// Ledger has no public push, so escrow uses transfer-like primitives here by
// exposing a crate-internal helper.
fn ledger_push(ledger: &mut Ledger, account: &str, delta: MicroNari, kind: EntryKind, at: i64) {
    ledger.push_public(account, delta, kind, at);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ONE_NARI;

    fn funded() -> Ledger {
        let mut l = Ledger::new();
        l.genesis_grant("payer", 0).unwrap();
        l
    }

    #[test]
    fn open_then_release_pays_payee() {
        let mut l = funded();
        let mut e = Escrow::open(&mut l, "payer", "payee", 5 * ONE_NARI, 1).unwrap();
        assert_eq!(l.balance("payer"), 8 * ONE_NARI);
        e.release(&mut l, 2);
        assert_eq!(l.balance("payee"), 5 * ONE_NARI);
        assert_eq!(e.state, EscrowState::Released);
    }

    #[test]
    fn open_then_refund_returns_to_payer() {
        let mut l = funded();
        let mut e = Escrow::open(&mut l, "payer", "payee", 5 * ONE_NARI, 1).unwrap();
        e.refund(&mut l, 2);
        assert_eq!(l.balance("payer"), 13 * ONE_NARI);
        assert_eq!(l.balance("payee"), 0);
        assert_eq!(e.state, EscrowState::Refunded);
    }

    #[test]
    fn double_release_is_noop() {
        let mut l = funded();
        let mut e = Escrow::open(&mut l, "payer", "payee", 5 * ONE_NARI, 1).unwrap();
        e.release(&mut l, 2);
        e.release(&mut l, 3); // no double pay
        assert_eq!(l.balance("payee"), 5 * ONE_NARI);
    }
}
