-- name: CreateDocumentContributor :exec
INSERT INTO document_contributors (document_id, user_id)
VALUES ($1, $2);
