-- name: GetUserByEmail :one
SELECT * FROM users WHERE email = $1;

-- name: CreateUser :one
INSERT INTO users (email, username, passhash)
    VALUES ($1, $2, $3)
RETURNING *;

-- name: UpdateUserImage :exec
UPDATE users
SET image_url=$2
WHERE id=$1;
