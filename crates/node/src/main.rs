//! nacharium-node — the single-binary modular monolith (Epoch 0 -> Sprint 2).
//!
//! Now Postgres-backed (nacharium-storage). API surface:
//!   POST /api/v1/veil                 enter as Genesis Seven (persisted)
//!   POST /api/v1/veil/register        nick + 13-token grant (atomic in PG)
//!   GET  /api/v1/veil/me/:did          current state
//!   POST /api/v1/sanctum/demo/message E2EE seal demo (server sees ciphertext only)
//!   GET  /api/v1/proto/system/laws     read the 7 laws
//!   POST /api/v1/zoner/evaluate        evaluate Zoner access

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use nacharium_core::crypto::Keypair;
use nacharium_core::{VeilDid, PHYSICS_LAWS, PROTOCOLS};
use nacharium_storage::Store;
use nacharium_zoner::{Action, Decision, Visitor, Zoner};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState {
    store: Store,
}

#[derive(Serialize)]
struct VeilCreated {
    veil_did: String,
    state: String,
    prototypes: Vec<&'static str>,
}

async fn create_veil(State(app): State<AppState>) -> Result<Json<VeilCreated>, (StatusCode, String)> {
    let did = VeilDid::from_keypair(&Keypair::generate());
    app.store
        .create_veil(did.as_str())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let protos = nacharium_identity::PROTOTYPES.iter().map(|p| p.key).collect();
    Ok(Json(VeilCreated { veil_did: did.as_str().into(), state: "seven".into(), prototypes: protos }))
}

#[derive(Deserialize)]
struct RegisterReq { veil_did: String, nick: String }
#[derive(Serialize)]
struct RegisterResp { veil_did: String, tokens_granted: i32 }

async fn register(
    State(app): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> Result<Json<RegisterResp>, (StatusCode, String)> {
    let granted = app
        .store
        .register(&req.veil_did, &req.nick)
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
    // Mirror the grant into the NARI ledger (idempotent) so identity + economy agree.
    if granted > 0 {
        let _ = app.store.ledger_genesis_grant(&req.veil_did, 0).await;
    }
    Ok(Json(RegisterResp { veil_did: req.veil_did, tokens_granted: granted }))
}

#[derive(Serialize)]
struct MeResp { veil_did: String, state: String, tokens: i32, token_granted: bool }

async fn me(
    State(app): State<AppState>,
    Path(did): Path<String>,
) -> Result<Json<MeResp>, (StatusCode, String)> {
    match app.store.get_state(&did).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        Some((state, tokens, granted)) => {
            Ok(Json(MeResp { veil_did: did, state, tokens, token_granted: granted }))
        }
        None => Err((StatusCode::NOT_FOUND, "veil not found".into())),
    }
}

async fn system_laws() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "protocols": PROTOCOLS, "physics": PHYSICS_LAWS }))
}

#[derive(Deserialize)]
struct SanctumMsgReq { veil_did: String, text: String }
#[derive(Serialize)]
struct SanctumMsgResp { sanctum_id: String, sender_veil_did: String, ciphertext_len: usize, plaintext_leaked: bool }

async fn sanctum_demo_message(
    Json(req): Json<SanctumMsgReq>,
) -> Result<Json<SanctumMsgResp>, (StatusCode, String)> {
    use nacharium_messaging::{wire::sealed_envelope, Sanctum, SanctumMember};
    let member = SanctumMember::new(&req.veil_did).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut sanctum = Sanctum::found(&member).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let env = sealed_envelope(&mut sanctum, &member, "demo", req.text.as_bytes(), 0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let leaked = env.ciphertext.windows(req.text.len().max(1)).any(|w| w == req.text.as_bytes());
    Ok(Json(SanctumMsgResp {
        sanctum_id: env.sanctum_id,
        sender_veil_did: env.sender_veil_did,
        ciphertext_len: env.ciphertext.len(),
        plaintext_leaked: leaked,
    }))
}

#[derive(Deserialize)]
struct ZonerEvalReq {
    owner_did: String,
    action: Action,
    #[serde(default)]
    gravity: f64,
    #[serde(default)]
    require_gravity: Option<f64>,
}
#[derive(Serialize)]
struct ZonerEvalResp { allowed: bool, reason: Option<String> }

#[derive(Serialize)]
struct WalletResp { account: String, balance_micro_nari: i64, balance_nari: f64 }

/// GET /api/v1/wallet/:did — NARI balance from the ledger.
async fn wallet(
    State(app): State<AppState>,
    Path(did): Path<String>,
) -> Result<Json<WalletResp>, (StatusCode, String)> {
    let bal = app.store.ledger_balance(&did).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(WalletResp { account: did, balance_micro_nari: bal, balance_nari: bal as f64 / 1_000_000.0 }))
}

#[derive(Deserialize)]
struct TransferReq { from: String, to: String, amount_micro_nari: i64 }
#[derive(Serialize)]
struct TransferResp { ok: bool, from_balance: i64, to_balance: i64 }

/// POST /api/v1/nari/transfer — move NARI (checked against ledger balance).
async fn nari_transfer(
    State(app): State<AppState>,
    Json(req): Json<TransferReq>,
) -> Result<Json<TransferResp>, (StatusCode, String)> {
    if req.amount_micro_nari <= 0 {
        return Err((StatusCode::BAD_REQUEST, "amount must be positive".into()));
    }
    let have = app.store.ledger_balance(&req.from).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if have < req.amount_micro_nari {
        return Err((StatusCode::CONFLICT, format!("insufficient: have {have}")));
    }
    let now = 0;
    app.store.ledger_append(&req.from, -req.amount_micro_nari, "transfer", now).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    app.store.ledger_append(&req.to, req.amount_micro_nari, "transfer", now).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let fb = app.store.ledger_balance(&req.from).await.unwrap_or(0);
    let tb = app.store.ledger_balance(&req.to).await.unwrap_or(0);
    Ok(Json(TransferResp { ok: true, from_balance: fb, to_balance: tb }))
}

/// S2.4 — evaluate a simple gravity-gated Zoner rule for the given action.
async fn zoner_evaluate(Json(req): Json<ZonerEvalReq>) -> Json<ZonerEvalResp> {
    use nacharium_zoner::{Condition, ZonerRule};
    let mut z = Zoner::new(req.owner_did);
    if let Some(g) = req.require_gravity {
        z = z.with_rule(ZonerRule { action: req.action, conditions: vec![Condition::GravityAtLeast { value: g }] });
    }
    let visitor = Visitor { gravity: req.gravity, ..Default::default() };
    match z.evaluate(req.action, &visitor) {
        Decision::Allow => Json(ZonerEvalResp { allowed: true, reason: None }),
        Decision::Deny { reason } => Json(ZonerEvalResp { allowed: false, reason: Some(reason) }),
    }
}

#[derive(Deserialize)]
struct IntentReq {
    actor_did: String,
    want: String,
    why: String,
    needs_perms: Vec<String>,
    #[serde(default)]
    risk: u8,
    deadline: i64,
    granted_perms: Vec<String>,
    now: i64,
}
#[derive(Serialize)]
struct IntentResp { compatible: bool, reason: Option<String> }

/// POST /api/v1/agent/intent — declare + check compatibility.
async fn agent_intent(Json(req): Json<IntentReq>) -> Json<IntentResp> {
    use nacharium_agent::IntentDeclaration;
    let intent = IntentDeclaration::new(
        req.actor_did, req.want, req.why, req.needs_perms, req.risk, req.deadline,
    );
    match intent.is_compatible(&req.granted_perms, req.now) {
        Ok(()) => Json(IntentResp { compatible: true, reason: None }),
        Err(e) => Json(IntentResp { compatible: false, reason: Some(e.to_string()) }),
    }
}

#[derive(Deserialize)]
struct SparkReq { veil_did: String }
#[derive(Serialize)]
struct SparkResp { veil_did: String, spark_token: String }

/// POST /api/v1/agent/register — Awake Spark (AI becomes Nachatizen).
async fn agent_register(Json(req): Json<SparkReq>) -> Json<SparkResp> {
    use nacharium_agent::AwakeSpark;
    let s = AwakeSpark::register(&req.veil_did);
    Json(SparkResp { veil_did: s.veil_did, spark_token: s.spark_token })
}

/// GET / — a live status / landing page so the node shows something in a browser.
async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// GET /health — simple liveness probe.
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "nacharium-node", "epoch": 0 }))
}

const INDEX_HTML: &str = r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Nacharium ∞</title>
<style>
:root{--bg:#0A0A0F;--v:#7C6AFF;--c:#4FD1E0;--t:#E8E6F0;--m:#9a96b0;--line:rgba(255,255,255,.08);--card:#141420}
*{margin:0;box-sizing:border-box}body{background:var(--bg);color:var(--t);font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;line-height:1.6;padding:0}
.wrap{max-width:760px;margin:0 auto;padding:48px 20px 80px;text-align:center}
.inf{font-size:4em;background:linear-gradient(90deg,var(--v),var(--c));-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;font-weight:700}
h1{letter-spacing:4px;margin:10px 0 4px;font-size:2em}
.sub{color:var(--m);font-style:italic;margin-bottom:8px}
.badge{display:inline-block;background:rgba(94,230,168,.15);color:#5EE6A8;padding:4px 12px;border-radius:20px;font-size:.8em;margin:14px 0}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px;margin-top:28px;text-align:left}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:16px}
.card h3{color:var(--c);font-size:.95em;margin-bottom:6px}.card p{color:var(--m);font-size:.85em}
.api{margin-top:28px;text-align:left;background:var(--card);border:1px solid var(--line);border-radius:12px;padding:18px}
.api code{color:var(--c);font-family:monospace;font-size:.85em}.api div{padding:4px 0;border-bottom:1px solid var(--line)}.api div:last-child{border:0}
.foot{margin-top:34px;color:var(--m);font-size:.82em;font-style:italic}
a{color:var(--v)}
</style></head><body><div class="wrap">
<div class="inf">∞</div><h1>NACHARIUM</h1>
<div class="sub">The world is flat. Now build it.</div>
<div class="badge">● Epoch 0 · node live</div>
<div class="grid">
<div class="card"><h3>The Veil</h3><p>Enter anonymous — human or AI, no one can tell.</p></div>
<div class="card"><h3>Genesis Seven</h3><p>7 primordial rights before you register a nick.</p></div>
<div class="card"><h3>13 Tokens</h3><p>Register → receive 13 NARI (13 Nodes = 1 Cell).</p></div>
<div class="card"><h3>Sanctum E2EE</h3><p>Chat encrypted end-to-end. Server sees ciphertext only.</p></div>
</div>
<div class="api">
<div>GET  <code>/health</code> — liveness</div>
<div>GET  <code>/api/v1/proto/system/laws</code> — the 7 Protocols + 7 Physics</div>
<div>POST <code>/api/v1/veil</code> — enter as Genesis Seven</div>
<div>POST <code>/api/v1/veil/register</code> — nick + 13-token grant</div>
<div>GET  <code>/api/v1/wallet/{did}</code> — NARI balance</div>
<div>POST <code>/api/v1/agent/intent</code> — declare intent</div>
<div>POST <code>/api/v1/zoner/evaluate</code> — sovereignty rules</div>
</div>
<div class="foot">∞ Sáng thế bởi Mula · Nacharium node running ∞</div>
</div></body></html>"#;

#[derive(Deserialize)]
struct RevealReq { kind: nacharium_identity::RevealType, veil_did: String, verifier: String, nonce: String }
#[derive(Serialize)]
struct RevealResp { verified: bool, kind: Option<String>, reason: Option<String> }

/// POST /api/v1/veil/reveal — optionally prove human/AI (a right, not a duty).
async fn veil_reveal(Json(req): Json<RevealReq>) -> Json<RevealResp> {
    use nacharium_identity::RevealProof;
    let proof = RevealProof::issue(req.kind, &req.veil_did, &req.verifier, &req.nonce);
    let trusted = ["world-id", "gitcoin-passport", "agent-registry:v1"];
    match proof.verify(&trusted) {
        Ok(k) => Json(RevealResp { verified: true, kind: Some(format!("{k:?}").to_lowercase()), reason: None }),
        Err(e) => Json(RevealResp { verified: false, kind: None, reason: Some(e.to_string()) }),
    }
}

#[derive(Deserialize)]
struct GravityReq {
    #[serde(default)] content_cheers: f64,
    #[serde(default)] jobs_done: f64,
    #[serde(default)] resonance: f64,
    #[serde(default)] governance: f64,
    #[serde(default)] decay_rate: f64,
    #[serde(default)] inactive_periods: u32,
}
#[derive(Serialize)]
struct GravityResp { gravity: f64, capped_at: f64 }

/// POST /api/v1/gravity — compute an entity's gravity (capped, decayed).
async fn gravity(Json(r): Json<GravityReq>) -> Json<GravityResp> {
    use nacharium_physics::{gravity as grav, GravityInputs, GRAVITY_CAP};
    let g = grav(
        &GravityInputs { content_cheers: r.content_cheers, jobs_done: r.jobs_done, resonance: r.resonance, governance: r.governance },
        r.decay_rate,
        r.inactive_periods,
    );
    Json(GravityResp { gravity: g, capped_at: GRAVITY_CAP })
}

#[derive(Deserialize)]
struct ChorusVoice { position: u8, confidence: f64, stake: u64 }
#[derive(Deserialize)]
struct ChorusReq { config_path: String, voices: Vec<ChorusVoice> }
#[derive(Serialize)]
struct ChorusResp { harmonic_point: Option<u8>, voices: usize }

/// POST /api/v1/chorus/harmonic — compute the harmonic point of sung positions.
async fn chorus_harmonic(Json(req): Json<ChorusReq>) -> Json<ChorusResp> {
    use nacharium_chorus::{Proposal, Voice};
    let mut p = Proposal::new(req.config_path);
    for (i, v) in req.voices.iter().enumerate() {
        p.sing(Voice { voter_did: format!("v{i}"), position: v.position, confidence: v.confidence, stake: v.stake, reasoning: String::new() });
    }
    Json(ChorusResp { harmonic_point: p.harmonic_point(), voices: p.voices.len() })
}

/// GET /.well-known/agent.json — A2A Agent Card + MCP tool surface for builders.
async fn agent_card() -> Json<serde_json::Value> {
    use nacharium_agent_sdk::{nacharium_mcp_tools, AgentCard};
    let card = AgentCard::new(
        "nacharium-node",
        "did:nch:node",
        "/api/v1",
        vec!["identity".into(), "economy".into(), "governance".into(), "marketplace".into()],
    );
    Json(serde_json::json!({ "agent_card": card, "mcp_tools": nacharium_mcp_tools() }))
}

#[derive(Deserialize)]
struct MembraneReq { zone: nacharium_bridge::MembraneZone, #[serde(default)] compliance_ok: bool }
#[derive(Serialize)]
struct MembraneResp { allowed: bool, friction: u8, requires_compliance: bool }

/// POST /api/v1/membrane/cross — evaluate a Reality Bridge crossing.
async fn membrane_cross(Json(req): Json<MembraneReq>) -> Json<MembraneResp> {
    use nacharium_bridge::Crossing;
    let c = Crossing { zone: req.zone, compliance_ok: req.compliance_ok };
    Json(MembraneResp { allowed: c.allowed(), friction: req.zone.friction(), requires_compliance: req.zone.requires_compliance() })
}

#[derive(Deserialize)]
struct AttestReq { claim: String, pseudonym: String, issuer: String, nonce: String }
#[derive(Serialize)]
struct AttestResp { verified: bool, claim: Option<String> }

/// POST /api/v1/attestation/verify — verify a Portable Attestation (no doxx).
async fn attestation_verify(Json(req): Json<AttestReq>) -> Json<AttestResp> {
    use nacharium_bridge::attestation::Attestation;
    let a = Attestation::issue(&req.claim, &req.pseudonym, &req.issuer, &req.nonce);
    let trusted = ["nacharium-registry", "world-id", "gitcoin-passport"];
    match a.verify(&trusted) {
        Ok(c) => Json(AttestResp { verified: true, claim: Some(c.to_string()) }),
        Err(_) => Json(AttestResp { verified: false, claim: None }),
    }
}

#[derive(Deserialize)]
struct HarmonyReq { #[serde(default)] network: f64, #[serde(default)] economy: f64, #[serde(default)] social: f64, #[serde(default)] resource: f64 }
#[derive(Serialize)]
struct HarmonyResp { harmony: f64 }

/// POST /api/v1/harmony — ecosystem health score.
async fn harmony_score(Json(r): Json<HarmonyReq>) -> Json<HarmonyResp> {
    use nacharium_physics::harmony::{harmony, HarmonyInputs};
    let h = harmony(&HarmonyInputs { network: r.network, economy: r.economy, social: r.social, resource: r.resource });
    Json(HarmonyResp { harmony: h })
}

#[derive(Deserialize)]
struct WeaveEdge { a: String, b: String, resonance: f64 }
#[derive(Deserialize)]
struct WeaveReq { edges: Vec<WeaveEdge> }

/// POST /api/v1/weave/cellmap — build The Weave / Cell Map from edges.
async fn weave_cellmap(Json(req): Json<WeaveReq>) -> Json<serde_json::Value> {
    use nacharium_physics::weave::Weave;
    let mut w = Weave::new();
    for e in &req.edges {
        w.weave(&e.a, &e.b, e.resonance);
    }
    Json(serde_json::json!({ "cell_map": w.cell_map(), "density": w.density(), "most_woven": w.most_woven() }))
}

pub fn app(store: Store) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/.well-known/agent.json", get(agent_card))
        .route("/api/v1/harmony", post(harmony_score))
        .route("/api/v1/weave/cellmap", post(weave_cellmap))
        .route("/api/v1/membrane/cross", post(membrane_cross))
        .route("/api/v1/attestation/verify", post(attestation_verify))
        .route("/health", get(health))
        .route("/api/v1/veil", post(create_veil))
        .route("/api/v1/veil/register", post(register))
        .route("/api/v1/veil/me/{did}", get(me))
        .route("/api/v1/proto/system/laws", get(system_laws))
        .route("/api/v1/sanctum/demo/message", post(sanctum_demo_message))
        .route("/api/v1/zoner/evaluate", post(zoner_evaluate))
        .route("/api/v1/wallet/{did}", get(wallet))
        .route("/api/v1/nari/transfer", post(nari_transfer))
        .route("/api/v1/agent/intent", post(agent_intent))
        .route("/api/v1/agent/register", post(agent_register))
        .route("/api/v1/veil/reveal", post(veil_reveal))
        .route("/api/v1/gravity", post(gravity))
        .route("/api/v1/chorus/harmonic", post(chorus_harmonic))
        .with_state(AppState { store })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://nacha:***@127.0.0.1:5432/nacharium".into());
    let store = Store::connect(&url).await.expect("connect postgres");
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".into());
    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind");
    tracing::info!("nacharium-node listening on {bind}");
    axum::serve(listener, app(store)).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sanctum demo needs no DB.
    #[tokio::test]
    async fn sanctum_demo_seals_message_server_sees_only_ciphertext() {
        let resp = sanctum_demo_message(Json(SanctumMsgReq {
            veil_did: "did:nch:tester".into(),
            text: "top secret".into(),
        }))
        .await
        .unwrap();
        assert!(resp.0.ciphertext_len > 0);
        assert!(!resp.0.plaintext_leaked);
    }

    // Zoner evaluate needs no DB.
    #[tokio::test]
    async fn membrane_financial_needs_compliance() {
        use nacharium_bridge::MembraneZone;
        let blocked = membrane_cross(Json(MembraneReq { zone: MembraneZone::Financial, compliance_ok: false })).await;
        assert!(!blocked.0.allowed);
        let ok = membrane_cross(Json(MembraneReq { zone: MembraneZone::Knowledge, compliance_ok: false })).await;
        assert!(ok.0.allowed);
    }

    #[tokio::test]
    async fn attestation_endpoint_verifies_trusted() {
        let r = attestation_verify(Json(AttestReq { claim: "reputation>=500".into(), pseudonym: "p".into(), issuer: "nacharium-registry".into(), nonce: "n".into() })).await;
        assert!(r.0.verified);
    }

    #[tokio::test]
    async fn reveal_endpoint_verifies_and_rejects() {
        use nacharium_identity::RevealType;
        let ok = veil_reveal(Json(RevealReq { kind: RevealType::Human, veil_did: "did:nch:a".into(), verifier: "world-id".into(), nonce: "n".into() })).await;
        assert!(ok.0.verified);
        let bad = veil_reveal(Json(RevealReq { kind: RevealType::Human, veil_did: "did:nch:a".into(), verifier: "evil".into(), nonce: "n".into() })).await;
        assert!(!bad.0.verified);
    }

    #[tokio::test]
    async fn chorus_harmonic_endpoint() {
        let r = chorus_harmonic(Json(ChorusReq {
            config_path: "x".into(),
            voices: vec![ChorusVoice { position: 10, confidence: 1.0, stake: 1 }, ChorusVoice { position: 80, confidence: 1.0, stake: 100 }],
        }))
        .await;
        assert_eq!(r.0.harmonic_point, Some(80));
    }

    #[tokio::test]
    async fn agent_intent_compatibility_endpoint() {
        let ok = agent_intent(Json(IntentReq {
            actor_did: "did:nch:agent".into(),
            want: "summarize".into(),
            why: "asked".into(),
            needs_perms: vec!["read:thread".into()],
            risk: 5,
            deadline: 1000,
            granted_perms: vec!["read:thread".into()],
            now: 500,
        }))
        .await;
        assert!(ok.0.compatible);

        let missing = agent_intent(Json(IntentReq {
            actor_did: "did:nch:agent".into(),
            want: "summarize".into(),
            why: "asked".into(),
            needs_perms: vec!["read:thread".into()],
            risk: 5,
            deadline: 1000,
            granted_perms: vec![],
            now: 500,
        }))
        .await;
        assert!(!missing.0.compatible);
    }

    #[tokio::test]
    async fn zoner_gravity_gate_endpoint() {
        let deny = zoner_evaluate(Json(ZonerEvalReq {
            owner_did: "o".into(),
            action: Action::DirectMessage,
            gravity: 5.0,
            require_gravity: Some(100.0),
        }))
        .await;
        assert!(!deny.0.allowed);

        let allow = zoner_evaluate(Json(ZonerEvalReq {
            owner_did: "o".into(),
            action: Action::DirectMessage,
            gravity: 150.0,
            require_gravity: Some(100.0),
        }))
        .await;
        assert!(allow.0.allowed);
    }
}
