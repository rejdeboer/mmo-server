CREATE TABLE IF NOT EXISTS document_updates (
    document_id uuid NOT NULL REFERENCES documents(id)
        ON DELETE CASCADE,
    clock integer NOT NULL,
    value bytea NOT NULL,
    PRIMARY KEY(document_id, clock),
    UNIQUE(document_id, value)
);
