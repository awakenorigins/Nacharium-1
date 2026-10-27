//! Live Postgres integration test. Requires DATABASE_URL to a running PG.
//! Skipped gracefully if the DB is unavailable.

use nacharium_core::envelope::{ContentHint, MessageEnvelope};
use nacharium_storage::Store;

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://nacha:nacha@127.0.0.1:5432/nacharium".to_string())
}

#[tokio::test]
async fn full_persistence_roundtrip() {
    let store = match Store::connect(&db_url()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SKIP: no DB available: {e}");
            return;
        }
    };

    // unique veil for repeatable runs
    let did = format!("did:nch:test-{}", uuid_like());
    store.create_veil(&did).await.unwrap();

    // first register grants 13
    let g1 = store.register(&did, &format!("nick-{}", uuid_like())).await.unwrap();
    assert_eq!(g1, 13, "first register must grant 13 tokens");

    // second register grants 0 (idempotent, no double-grant)
    let g2 = store.register(&did, &format!("nick2-{}", uuid_like())).await.unwrap();
    assert_eq!(g2, 0, "re-register must not grant more tokens");

    // store a ciphertext message
    let sanctum = format!("sanctum-{}", uuid_like());
    let env = MessageEnvelope {
        sanctum_id: sanctum.clone(),
        sender_veil_did: did.clone(),
        content_hint: ContentHint::Text,
        ciphertext: vec![9, 8, 7, 6, 5],
        sent_at: 1_700_000_123,
    };
    let id = store.store_message(&env).await.unwrap();
    assert!(id > 0);
    assert_eq!(store.message_count(&sanctum).await.unwrap(), 1);

    // S3.5 — NARI ledger persistence.
    let acct = format!("did:nch:test-wallet-{}", uuid_like());
    let g = store.ledger_genesis_grant(&acct, 0).await.unwrap();
    assert_eq!(g, 13_000_000, "genesis grant is 13 NARI");
    // idempotent: second grant is 0
    assert_eq!(store.ledger_genesis_grant(&acct, 1).await.unwrap(), 0);
    // balance reflects exactly 13 NARI (not 26)
    assert_eq!(store.ledger_balance(&acct).await.unwrap(), 13_000_000);
    // a transfer-out delta reduces balance
    store.ledger_append(&acct, -3_000_000, "transfer", 2).await.unwrap();
    assert_eq!(store.ledger_balance(&acct).await.unwrap(), 10_000_000);
}

// tiny unique-ish suffix without extra deps
fn uuid_like() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
}
