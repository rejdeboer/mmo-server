CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS users (
    id uuid PRIMARY KEY DEFAULT UUID_GENERATE_V4(),
    email text NOT NULL UNIQUE,
    username text NOT NULL UNIQUE,
    passhash text NOT NULL,
    image_url text
);

CREATE INDEX IF NOT EXISTS "idx_users_id" ON "users" ("id");
