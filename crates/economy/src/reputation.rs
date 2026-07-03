//! M3.2 — Reputation staking. "Uy tín là tài sản." Stake NARI to sell;
//! fraud => the stake is slashed. Reputation staked is locked away from the
//! spendable balance.

use crate::{EconError, EntryKind, Ledger, MicroNari};

/// A reputation stake held by an account.
#[derive(Debug, Clone)]
pub struct Stake {
    pub owner: String,
    pub amount: MicroNari,
    pub slashed: bool,
}

impl Stake {
    /// Lock `amount` from the owner's balance into a stake.
    pub fn place(ledger: &mut Ledger, owner: &str, amount: MicroNari, at: i64) -> Result<Self, EconError> {
        if amount <= 0 {
            return Err(EconError::NonPositive);
        }
        let have = ledger.balance(owner);
        if have < amount {
            return Err(EconError::Insufficient { have, need: amount });
        }
        let staked = format!("stake:{owner}");
        ledger.push_public(owner, -amount, EntryKind::Stake, at);
        ledger.push_public(&staked, amount, EntryKind::Stake, at);
        Ok(Self { owner: owner.into(), amount, slashed: false })
    }

    /// Slash the stake on proven fraud — value goes to the Treasury.
    pub fn slash(&mut self, ledger: &mut Ledger, treasury: &str, at: i64) {
        if self.slashed {
            return;
        }
        let staked = format!("stake:{}", self.owner);
        ledger.push_public(&staked, -self.amount, EntryKind::Slash, at);
        ledger.push_public(treasury, self.amount, EntryKind::Slash, at);
        self.slashed = true;
    }

    /// Return the stake to the owner (honest exit).
    pub fn release(&mut self, ledger: &mut Ledger, at: i64) {
        if self.slashed {
            return;
        }
        let staked = format!("stake:{}", self.owner);
        ledger.push_public(&staked, -self.amount, EntryKind::Stake, at);
        ledger.push_public(&self.owner, self.amount, EntryKind::Stake, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ONE_NARI;

    fn funded() -> Ledger {
        let mut l = Ledger::new();
        l.genesis_grant("seller", 0).unwrap();
        l
    }

    #[test]
    fn stake_locks_balance() {
        let mut l = funded();
        Stake::place(&mut l, "seller", 3 * ONE_NARI, 1).unwrap();
        assert_eq!(l.balance("seller"), 10 * ONE_NARI);
        assert_eq!(l.balance("stake:seller"), 3 * ONE_NARI);
    }

    #[test]
    fn slash_sends_stake_to_treasury() {
        let mut l = funded();
        let mut s = Stake::place(&mut l, "seller", 3 * ONE_NARI, 1).unwrap();
        s.slash(&mut l, "treasury", 2);
        assert_eq!(l.balance("stake:seller"), 0);
        assert_eq!(l.balance("treasury"), 3 * ONE_NARI);
        assert!(s.slashed);
    }

    #[test]
    fn honest_release_returns_stake() {
        let mut l = funded();
        let mut s = Stake::place(&mut l, "seller", 3 * ONE_NARI, 1).unwrap();
        s.release(&mut l, 2);
        assert_eq!(l.balance("seller"), 13 * ONE_NARI);
    }
}
