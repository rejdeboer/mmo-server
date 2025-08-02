CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    email VARCHAR(50) NOT NULL UNIQUE,
    username VARCHAR(50) NOT NULL UNIQUE,
    passhash VARCHAR(256) NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_accounts_id" ON "accounts" ("id");

CREATE TABLE IF NOT EXISTS guilds (
    id SERIAL PRIMARY KEY,
    -- TODO: Once we implement realms this uniqueness constraint should change
    name VARCHAR(50) NOT NULL UNIQUE
);

CREATE INDEX IF NOT EXISTS "idx_guilds_id" ON "guilds" ("id");

CREATE TABLE IF NOT EXISTS characters (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    account_id INT NOT NULL REFERENCES accounts(id)
        ON DELETE CASCADE,
    guild_id INT REFERENCES guilds(id),

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
CREATE INDEX IF NOT EXISTS idx_characters_guild_id ON characters(guild_id);
