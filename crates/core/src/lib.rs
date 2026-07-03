//! nacharium-core — L0 Foundation.
//!
//! Holds the immutable Nacharium Protocol (7 laws) + 7 Physics laws as
//! machine-readable constants, the wire envelope codec (CBOR), core crypto
//! (Ed25519 keypair = "Key is Crown"), and the Veil DID.
//!
//! These constants are COMPILED-IN and cannot be changed by any endpoint —
//! this is how the "System core" stays immutable (Protocol 3: No King).

#![forbid(unsafe_code)]

pub mod protocol;
pub mod crypto;
pub mod did;
pub mod envelope;

pub use did::VeilDid;
pub use protocol::{PhysicsLaw, Protocol, PROTOCOLS, PHYSICS_LAWS};
