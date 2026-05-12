CREATE TYPE oauth_provider AS ENUM (
    'discord',
    'reddit'
);

CREATE TYPE voter_kind AS ENUM (
    'account',
    'anon'
);

CREATE TABLE accounts (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    provider oauth_provider NOT NULL,
    provider_subject text NOT NULL,
    display_name text NOT NULL,
    provider_created_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_subject)
);

CREATE TABLE sessions (
    token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    account_id bigint NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX sessions_account_id_idx ON sessions (account_id);

CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

CREATE TABLE anon_voters (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    fingerprint_hmac bytea NOT NULL UNIQUE CHECK (octet_length(fingerprint_hmac) = 32),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE voters (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind voter_kind NOT NULL,
    account_id bigint UNIQUE REFERENCES accounts (id) ON DELETE CASCADE,
    anon_id bigint UNIQUE REFERENCES anon_voters (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((kind = 'account' AND account_id IS NOT NULL AND anon_id IS NULL) OR (kind = 'anon' AND anon_id IS NOT NULL AND account_id IS NULL))
);

