-- Nacharium Sprint 1 initial schema.
CREATE TABLE IF NOT EXISTS nachatizen (
    veil_did      TEXT PRIMARY KEY,
    nick          TEXT UNIQUE,
    reveal_type   TEXT,                       -- null = Veiled; 'human' | 'ai'
    state         TEXT NOT NULL,              -- 'seven' | 'registered' | 'revealed'
    tokens        INTEGER NOT NULL DEFAULT 0,
    token_granted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sanctum (
    id         UUID PRIMARY KEY,
    kind       TEXT NOT NULL,                 -- 'dm' | 'group' | 'channel'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Messages store CIPHERTEXT ONLY. No plaintext, no human/ai column — by design.
CREATE TABLE IF NOT EXISTS message (
    id              BIGSERIAL PRIMARY KEY,
    sanctum_id      TEXT NOT NULL,
    sender_veil_did TEXT NOT NULL,
    content_hint    TEXT NOT NULL,
    ciphertext      BYTEA NOT NULL,
    sent_at         BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_message_sanctum ON message(sanctum_id, id);
