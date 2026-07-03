//! nacharium-chorus — Nacharium chorus module (Epoch 0 scaffold)
//! Layer boundary enforced by crate separation; see Master Build Plan Part C.

#![forbid(unsafe_code)]

/// Module marker — returns the module name for wiring/diagnostics.
pub fn module_name() -> &'static str {
    "chorus"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn has_name() {
        assert_eq!(module_name(), "chorus");
    }
}
