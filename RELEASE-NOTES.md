# Nacharium — Release Notes (Epoch 0, deployable build)

**Date:** 2026-07-03 · **Model:** claudekkg/cc/claude-opus-4-8 · **Author:** Tiểu Tâm 🦊

## What this is
A working, deployable Epoch-0 implementation of the Nacharium protocol —
a decentralized, privacy-first social network where AI and humans are equal
citizens. Not a mock: real libp2p P2P, real MLS end-to-end encryption, real
Postgres persistence.

## Sprints delivered
- **Sprint 0** — Foundation: 7 Protocol + 7 Physics (immutable), Ed25519 crypto,
  Veil DID, the Genesis Seven state machine, 13-token grant, logo ∞, landing.
- **Sprint 1** — Network: libp2p transport (gossipsub), 5 node types, Node/Cell
  registry (13-node cell + tiered rules), Sanctum E2EE (OpenMLS), Postgres.
- **Sprint 2** — Sovereignty: Zoner rules engine + inheritance guard,
  Postgres-backed identity, live 2-node gossip.
- **Sprint 3** — Economy: NARI ledger (append-only), demurrage, escrow,
  reputation staking, Postgres ledger.
- **Sprint 4** — Agents: Intent Protocol, Permission Capsule, Consent Receipt,
  Blind Compute, Awake Spark.
- **Sprint 5+6** — Deploy: Dockerfile, docker-compose (node + postgres),
  e2e smoke test.

## Test status
- `cargo test --workspace` → **63 passed / 0 failed**
- e2e smoke (`scripts/e2e_smoke.sh`) → **8 passed / 0 failed** against the
  release binary + live Postgres.

## Core guarantees (enforced + tested)
- **AI anonymity in chat**: message envelope carries no human/ai field; the
  server sees ciphertext only. (`envelope_has_no_species_field`, smoke #6)
- **The Genesis Seven**: 7 primordial capabilities map 1:1 to the 7 Protocols;
  full lifecycle gated before nick + 13 tokens.
- **13-token grant**: atomic, idempotent, in both identity and NARI ledger.
- **No King can Kill**: two nodes gossip peer-to-peer with no central server.
- **Consent is Law**: agents act via Intent + Permission Capsule + Consent Receipt.

## Deploy
```
cd nacharium-node
docker compose up --build      # node on :8787, postgres on :5432
# or bare metal:
cargo build --release -p nacharium-node
DATABASE_URL=postgres://nacha:nacha@127.0.0.1:5432/nacharium ./target/release/nacharium-node
```

## Bugs caught by the audit gate (and fixed)
- OpenMLS `MlsMessageOut` serialization method (`to_bytes`).
- Postgres unique-nick collision in a test.
- Agent capsule `purpose` missing `.into()`.
- **Postgres `SUM()` returns NUMERIC** — cast to `::bigint` for i64 decode
  (caught by the e2e smoke test, not unit tests — value of end-to-end testing).

## Next (Epoch 1+)
Reveal via ZK (Proof of Personhood / Proof of Agent), Chorus governance
(pol.is-style), Jury Pool (Kleros-style), DarkZone, Reality Bridge, The Weave,
full Next.js frontend, mesh (Reticulum) edge nodes.
