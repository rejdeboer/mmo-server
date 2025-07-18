CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    email VARCHAR(50) NOT NULL UNIQUE,
    username VARCHAR(50) NOT NULL UNIQUE,
    passhash VARCHAR(256) NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_accounts_id" ON "accounts" ("id");

CREATE TABLE IF NOT EXISTS characters (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    account_id INT NOT NULL REFERENCES accounts(id)
        ON DELETE CASCADE,

    level INT NOT NULL DEFAULT 1,
    experience BIGINT NOT NULL DEFAULT 0,

    position_x REAL NOT NULL DEFAULT 0.0,
    position_y REAL NOT NULL DEFAULT 0.0,
    position_z REAL NOT NULL DEFAULT 0.0,
    rotation_yaw REAL NOT NULL DEFAULT 0.0,

    is_online BOOLEAN NOT NULL DEFAULT FALSE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_characters_id" ON "characters" ("id");
CREATE INDEX IF NOT EXISTS idx_characters_account_id ON characters(account_id);
