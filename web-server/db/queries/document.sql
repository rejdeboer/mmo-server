-- name: GetDocumnetByID :one
SELECT * FROM documents WHERE id=$1;

-- name: GetDocumentWithContributorsByID :many
SELECT d.id as id, d.owner_id as owner_id, d.name as name,
    c.user_id as contributor_id
FROM documents d
JOIN document_contributors c on d.id = c.document_id
WHERE d.id = $1;

-- name: GetDocumentsByOwnerID :many
SELECT * FROM documents WHERE owner_id = $1;

-- name: CreateDocument :one
INSERT INTO documents (name, owner_id)
    VALUES ($1, $2)
RETURNING *;

-- name: DeleteDocument :exec
DELETE FROM documents 
WHERE id=$1;
