//! nacharium-storage — S1.6 Postgres persistence (sqlx).
//!
//! Replaces the in-memory HashMap with a real Postgres store for Nachatizen
//! records and Sanctum ciphertext messages. Uses runtime (not compile-time)
//! queries so a live DB isn't required to build.

#![forbid(unsafe_code)]

use nacharium_core::envelope::MessageEnvelope;
use sqlx::postgres::{PgPool, PgPoolOptions};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),
}

/// The Nacharium persistence store.
#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    /// Connect to Postgres and run migrations.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new().max_connections(8).connect(url).await?;
        sqlx::migrate!("../../migrations").run(&pool).await.map_err(|e| {
            StoreError::Db(sqlx::Error::Protocol(format!("migrate: {e}")))
        })?;
        Ok(Self { pool })
    }

    /// Insert a fresh Veiled entity (Genesis Seven state).
    pub async fn create_veil(&self, veil_did: &str) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO nachatizen (veil_did, state) VALUES ($1, 'seven') ON CONFLICT DO NOTHING")
            .bind(veil_did)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Register nick + grant 13 tokens atomically (idempotent on token_granted).
    /// Returns tokens granted (13) or 0 if already granted.
    pub async fn register(&self, veil_did: &str, nick: &str) -> Result<i32, StoreError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT token_granted FROM nachatizen WHERE veil_did = $1 FOR UPDATE")
                .bind(veil_did)
                .fetch_optional(&mut *tx)
                .await?;
        let already = matches!(row, Some((true,)));
        let granted = if already { 0 } else { 13 };
        sqlx::query(
            "UPDATE nachatizen SET nick=$2, state='registered', tokens=tokens+$3, token_granted=TRUE WHERE veil_did=$1",
        )
        .bind(veil_did)
        .bind(nick)
        .bind(granted)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(granted)
    }

    /// Persist a Sanctum ciphertext message (never plaintext).
    pub async fn store_message(&self, env: &MessageEnvelope) -> Result<i64, StoreError> {
        let hint = serde_json::to_string(&env.content_hint).unwrap_or_else(|_| "\"text\"".into());
        let rec: (i64,) = sqlx::query_as(
            "INSERT INTO message (sanctum_id, sender_veil_did, content_hint, ciphertext, sent_at)
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(&env.sanctum_id)
        .bind(&env.sender_veil_did)
        .bind(hint.trim_matches('"'))
        .bind(&env.ciphertext)
        .bind(env.sent_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(rec.0)
    }

    /// Fetch a Nachatizen's public state (state, tokens, whether granted).
    pub async fn get_state(
        &self,
        veil_did: &str,
    ) -> Result<Option<(String, i32, bool)>, StoreError> {
        let row: Option<(String, i32, bool)> = sqlx::query_as(
            "SELECT state, tokens, token_granted FROM nachatizen WHERE veil_did = $1",
        )
        .bind(veil_did)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Count messages in a sanctum (for tests / pagination bootstrap).
    pub async fn message_count(&self, sanctum_id: &str) -> Result<i64, StoreError> {
        let rec: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM message WHERE sanctum_id = $1")
                .bind(sanctum_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(rec.0)
    }

    // ---- S3.5 NARI ledger persistence ----

    /// Append one balanced ledger delta (never mutate existing rows).
    pub async fn ledger_append(
        &self,
        account: &str,
        delta: i64,
        kind: &str,
        at: i64,
    ) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO nari_ledger (account, delta, kind, at) VALUES ($1,$2,$3,$4)")
            .bind(account)
            .bind(delta)
            .bind(kind)
            .bind(at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Balance = SUM(delta) for the account (verifiable against the ledger).
    pub async fn ledger_balance(&self, account: &str) -> Result<i64, StoreError> {
        let rec: (Option<i64>,) =
            sqlx::query_as("SELECT COALESCE(SUM(delta),0)::bigint FROM nari_ledger WHERE account = $1")
                .bind(account)
                .fetch_one(&self.pool)
                .await?;
        Ok(rec.0.unwrap_or(0))
    }

    /// Idempotent 13-NARI genesis grant recorded in the ledger.
    /// Returns granted micro-NARI (13_000_000) or 0 if already granted.
    pub async fn ledger_genesis_grant(&self, account: &str, at: i64) -> Result<i64, StoreError> {
        use nacharium_economy::GENESIS_GRANT;
        let mut tx = self.pool.begin().await?;
        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM nari_ledger WHERE account=$1 AND kind='genesis13'",
        )
        .bind(account)
        .fetch_one(&mut *tx)
        .await?;
        if existing.0 > 0 {
            tx.commit().await?;
            return Ok(0);
        }
        sqlx::query("INSERT INTO nari_ledger (account, delta, kind, at) VALUES ($1,$2,'genesis13',$3)")
            .bind(account)
            .bind(GENESIS_GRANT)
            .bind(at)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(GENESIS_GRANT)
    }
}
