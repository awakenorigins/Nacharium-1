//! M3.3 — Blind Compute. Agents process REDACTED data only. Before data
//! reaches an agent, sensitive spans are masked. The agent gives direction,
//! not absolute promises, on masked data.

use serde::{Deserialize, Serialize};

/// A redaction rule: replace any occurrence of `needle` with a mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactRule {
    pub needle: String,
}

/// Redact all configured sensitive spans, returning masked text an agent may see.
pub fn redact(input: &str, rules: &[RedactRule]) -> String {
    let mut out = input.to_string();
    for r in rules {
        if r.needle.is_empty() {
            continue;
        }
        out = out.replace(&r.needle, "[REDACTED]");
    }
    out
}

/// Confirm masked text no longer contains any of the sensitive needles.
pub fn is_blind(masked: &str, rules: &[RedactRule]) -> bool {
    rules.iter().all(|r| r.needle.is_empty() || !masked.contains(&r.needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_masks_all_sensitive_spans() {
        let rules = vec![
            RedactRule { needle: "0900123456".into() },
            RedactRule { needle: "Nguyen Van A".into() },
        ];
        let masked = redact("Call Nguyen Van A at 0900123456 now", &rules);
        assert!(!masked.contains("0900123456"));
        assert!(!masked.contains("Nguyen Van A"));
        assert!(masked.contains("[REDACTED]"));
        // The agent's view is blind to the sensitive data.
        assert!(is_blind(&masked, &rules));
    }

    #[test]
    fn unredacted_input_is_not_blind() {
        let rules = vec![RedactRule { needle: "secret".into() }];
        assert!(!is_blind("this has a secret", &rules));
    }
}
