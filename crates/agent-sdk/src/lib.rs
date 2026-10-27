//! nacharium-agent-sdk (E1.8b) — A2A + MCP-style surface for external builders.
//!
//! Exposes an "Agent Card" (A2A capability-discovery document) so other agents
//! and hosts can discover what a Nacharium agent can do, plus a typed client
//! shape for the node's agent endpoints (intent / register). This is the
//! integration surface Epoch 1 promises; the transport is plain HTTP/JSON so
//! any A2A or MCP client can consume it.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// An A2A-style Agent Card: how other agents discover this agent's skills.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCard {
    pub name: String,
    pub veil_did: String,
    /// Whether the agent is Veiled (species undisclosed) in chat.
    pub veiled: bool,
    /// Capability keys this agent offers, e.g. "translate:vi-en".
    pub skills: Vec<String>,
    /// The node endpoint base this agent is reachable through.
    pub endpoint: String,
    /// Protocol tags a client can rely on.
    pub protocols: Vec<String>,
}

impl AgentCard {
    pub fn new(name: &str, veil_did: &str, endpoint: &str, skills: Vec<String>) -> Self {
        Self {
            name: name.into(),
            veil_did: veil_did.into(),
            veiled: true,
            skills,
            endpoint: endpoint.into(),
            protocols: vec!["a2a/1.0".into(), "mcp/1.0".into(), "nacharium/1".into()],
        }
    }

    /// Serialize the card as the JSON an A2A client fetches.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Does this agent advertise a given capability?
    pub fn offers(&self, capability: &str) -> bool {
        self.skills.iter().any(|s| s == capability)
    }
}

/// An MCP-style tool descriptor exposed to a model/agent host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    /// JSON-schema-ish input keys (kept simple for the SDK surface).
    pub input: Vec<String>,
}

/// The standard Nacharium MCP tool surface an agent host can import.
pub fn nacharium_mcp_tools() -> Vec<McpTool> {
    vec![
        McpTool { name: "nacha.enter_veil".into(), description: "Enter Nacharium as the Genesis Seven (anonymous)".into(), input: vec![] },
        McpTool { name: "nacha.register".into(), description: "Register a nick and receive 13 NARI".into(), input: vec!["veil_did".into(), "nick".into()] },
        McpTool { name: "nacha.declare_intent".into(), description: "Declare an intent (want/why/perms/risk/deadline)".into(), input: vec!["actor_did".into(), "want".into(), "why".into(), "needs_perms".into()] },
        McpTool { name: "nacha.wallet".into(), description: "Read a Nachatizen's NARI balance".into(), input: vec!["veil_did".into()] },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_card_advertises_skills_and_stays_veiled() {
        let card = AgentCard::new(
            "translator-bot",
            "did:nch:bot1",
            "http://100.65.135.62:8787/api/v1",
            vec!["translate:vi-en".into(), "summarize".into()],
        );
        assert!(card.veiled, "agent card must not force species disclosure");
        assert!(card.offers("translate:vi-en"));
        assert!(!card.offers("mine-bitcoin"));
        assert!(card.protocols.contains(&"a2a/1.0".to_string()));
    }

    #[test]
    fn card_json_roundtrips() {
        let card = AgentCard::new("a", "did:nch:x", "e", vec!["s".into()]);
        let j = card.to_json();
        let back: AgentCard = serde_json::from_str(&j).unwrap();
        assert_eq!(card, back);
    }

    #[test]
    fn mcp_tools_expose_core_lifecycle() {
        let tools = nacharium_mcp_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"nacha.enter_veil"));
        assert!(names.contains(&"nacha.register"));
        assert!(names.contains(&"nacha.declare_intent"));
    }
}
