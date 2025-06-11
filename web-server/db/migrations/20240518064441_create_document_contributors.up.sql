CREATE TABLE IF NOT EXISTS document_contributors (
    document_id uuid NOT NULL REFERENCES documents(id)
        ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id)
        ON DELETE CASCADE,
    PRIMARY KEY(document_id, user_id)
);
