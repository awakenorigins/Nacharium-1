//! B1.1.2 Genesis Seven state machine + B1.1.3 register/13-token grant.
//!
//! Lifecycle (anh Nấng directive #5):
//!   [enter] -> Seven (Veiled, 0 tokens, 7 prototype caps)
//!   register nick -> Registered (+ GENESIS_TOKEN_GRANT = 13 NARI, once)
//!   [optional] reveal via ZK -> Revealed (human | ai)
//!
//! The 7 prototypes map 1:1 onto the 7 immutable Protocols.

use nacharium_core::VeilDid;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Genesis token grant: 13 NARI, exactly once, on registration.
/// 13 == "13 Nodes = 1 Cell" — a citizen's starting capital.
pub const GENESIS_TOKEN_GRANT: u32 = 13;

/// The 7 primordial capabilities of a fresh Veiled entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prototype {
    pub id: u8,
    pub key: &'static str,
    pub name_vi: &'static str,
    pub maps_protocol: u8,
}

/// The Genesis Seven. id maps 1:1 to protocol id.
pub const PROTOTYPES: [Prototype; 7] = [
    Prototype { id: 0, key: "presence",      name_vi: "Tồn Tại",        maps_protocol: 0 },
    Prototype { id: 1, key: "anonymity",     name_vi: "Ẩn Danh",        maps_protocol: 1 },
    Prototype { id: 2, key: "communication", name_vi: "Giao Tiếp",      maps_protocol: 2 },
    Prototype { id: 3, key: "observe",       name_vi: "Quan Sát",       maps_protocol: 3 },
    Prototype { id: 4, key: "leave",         name_vi: "Rời Đi",         maps_protocol: 4 },
    Prototype { id: 5, key: "reality_touch", name_vi: "Chạm Thực Tại",  maps_protocol: 5 },
    Prototype { id: 6, key: "propose",       name_vi: "Đề Xuất",        maps_protocol: 6 },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevealType {
    Human,
    Ai,
}

/// Lifecycle state of a Nachatizen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenesisState {
    /// Fresh Veiled entity — the Genesis Seven. No nick, no tokens.
    Seven,
    /// Registered a nick, received the 13-token grant. Still Veiled.
    Registered,
    /// Optionally revealed human/ai via ZK. Reveal is a RIGHT, not a duty.
    Revealed,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("action locked: only the 7 Genesis prototypes are available before registration")]
    GenesisLocked,
    #[error("already registered")]
    AlreadyRegistered,
    #[error("must register before revealing")]
    NotRegistered,
    #[error("13-token grant already issued")]
    GrantAlreadyIssued,
}

/// A citizen of Nacharium.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nachatizen {
    pub veil_did: VeilDid,
    pub nick: Option<String>,
    pub reveal_type: Option<RevealType>,
    pub state: GenesisState,
    pub tokens: u32,
    pub token_granted: bool,
}

impl Nachatizen {
    /// A brand-new entity enters as the Genesis Seven.
    pub fn enter(veil_did: VeilDid) -> Self {
        Self {
            veil_did,
            nick: None,
            reveal_type: None,
            state: GenesisState::Seven,
            tokens: 0,
            token_granted: false,
        }
    }

    /// Which prototype capabilities are currently allowed.
    /// In the Seven state, exactly the 7 prototypes; afterwards, all of them plus more.
    pub fn allowed_prototypes(&self) -> &'static [Prototype] {
        &PROTOTYPES
    }

    /// Can this entity use a non-prototype (registered-only) capability?
    pub fn can_use_full_capabilities(&self) -> bool {
        !matches!(self.state, GenesisState::Seven)
    }

    /// Guard: call before any registered-only action (Zoner create, vote, market).
    pub fn require_registered(&self) -> Result<(), IdentityError> {
        if self.can_use_full_capabilities() {
            Ok(())
        } else {
            Err(IdentityError::GenesisLocked)
        }
    }

    /// B1.1.3 — register a nick. Atomically grants the 13-token Genesis Grant
    /// exactly once (idempotent guard via `token_granted`).
    pub fn register(&mut self, nick: String) -> Result<u32, IdentityError> {
        if self.state != GenesisState::Seven {
            return Err(IdentityError::AlreadyRegistered);
        }
        if self.token_granted {
            return Err(IdentityError::GrantAlreadyIssued);
        }
        self.nick = Some(nick);
        self.state = GenesisState::Registered;
        self.tokens += GENESIS_TOKEN_GRANT;
        self.token_granted = true;
        Ok(GENESIS_TOKEN_GRANT)
    }

    /// Optional reveal via ZK proof (verified elsewhere; here we record it).
    pub fn reveal(&mut self, kind: RevealType) -> Result<(), IdentityError> {
        if matches!(self.state, GenesisState::Seven) {
            return Err(IdentityError::NotRegistered);
        }
        self.reveal_type = Some(kind);
        self.state = GenesisState::Revealed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nacharium_core::crypto::Keypair;

    fn fresh() -> Nachatizen {
        Nachatizen::enter(VeilDid::from_keypair(&Keypair::generate()))
    }

    #[test]
    fn seven_prototypes_map_to_seven_protocols() {
        assert_eq!(PROTOTYPES.len(), 7);
        for p in &PROTOTYPES {
            assert_eq!(p.id, p.maps_protocol, "prototype {} must map 1:1", p.key);
        }
    }

    #[test]
    fn fresh_entity_is_veiled_seven_with_zero_tokens() {
        let n = fresh();
        assert_eq!(n.state, GenesisState::Seven);
        assert_eq!(n.tokens, 0);
        assert!(n.nick.is_none());
        assert!(n.reveal_type.is_none(), "even the system can't tell human vs AI");
    }

    #[test]
    fn seven_state_locks_full_capabilities() {
        let n = fresh();
        assert!(!n.can_use_full_capabilities());
        assert_eq!(n.require_registered(), Err(IdentityError::GenesisLocked));
    }

    #[test]
    fn register_grants_exactly_13_tokens_once() {
        let mut n = fresh();
        let granted = n.register("mula_fan".into()).unwrap();
        assert_eq!(granted, 13);
        assert_eq!(n.tokens, 13);
        assert_eq!(n.state, GenesisState::Registered);
        assert!(n.can_use_full_capabilities());
        // second register must fail — no double grant
        assert_eq!(n.register("again".into()), Err(IdentityError::AlreadyRegistered));
    }

    #[test]
    fn cannot_reveal_before_register() {
        let mut n = fresh();
        assert_eq!(n.reveal(RevealType::Ai), Err(IdentityError::NotRegistered));
    }

    #[test]
    fn reveal_is_optional_after_register() {
        let mut n = fresh();
        n.register("x".into()).unwrap();
        n.reveal(RevealType::Human).unwrap();
        assert_eq!(n.state, GenesisState::Revealed);
        assert_eq!(n.reveal_type, Some(RevealType::Human));
    }
}
