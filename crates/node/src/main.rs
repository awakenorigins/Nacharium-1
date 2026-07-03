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

pub fn app(store: Store) -> Router {
    Router::new()
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
        .with_state(AppState { store })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://nacha:***@127.0.0.1:5432/nacharium".into());
    let store = Store::connect(&url).await.expect("connect postgres");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8787").await.expect("bind");
    tracing::info!("nacharium-node on :8787 (Sprint 2, Postgres-backed)");
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
