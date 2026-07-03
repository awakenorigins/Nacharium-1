-- Nacharium Sprint 3 economy schema.
-- Append-only ledger: balance = SUM(delta) per account. Never mutate rows.
CREATE TABLE IF NOT EXISTS nari_ledger (
    id       BIGSERIAL PRIMARY KEY,
    account  TEXT NOT NULL,
    delta    BIGINT NOT NULL,        -- micro-NARI (6 decimals)
    kind     TEXT NOT NULL,
    at       BIGINT NOT NULL,
    created  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ledger_account ON nari_ledger(account);
