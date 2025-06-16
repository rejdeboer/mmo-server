use crate::auth::encode_jwt;
use crate::domain::{EmailAddress, LoginPassword};
use crate::{error::ApiError, ApplicationState};
use argon2::PasswordHash;
use axum::extract::State;
use axum::Json;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

struct LoginAttempt {
    pub email: EmailAddress,
    pub password: LoginPassword,
}

impl TryInto<LoginAttempt> for LoginBody {
    type Error = String;

    fn try_into(self) -> Result<LoginAttempt, Self::Error> {
        let email = EmailAddress::parse(self.email)?;
        let password = LoginPassword::parse(self.password)?;

        Ok(LoginAttempt { email, password })
    }
}

pub async fn login(
    State(state): State<ApplicationState>,
    Json(payload): Json<LoginBody>,
) -> Result<Json<TokenResponse>, ApiError> {
    let attempt: LoginAttempt = payload.try_into().map_err(ApiError::BadRequest)?;

    let row = sqlx::query!(
        r#"
        SELECT id, username, passhash
        FROM accounts
        WHERE email = $1
        LIMIT 1
        "#,
        attempt.email.as_ref(),
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, email = attempt.email.as_ref(), "account not found");
        ApiError::AuthError("incorrect credentials".to_string())
    })?;

    // TODO: Prevent timing attacks by having this route always taking the same time
    let passhash = row.passhash;
    tokio::task::spawn_blocking(move || {
        let passhash = PasswordHash::new(&passhash)?;
        attempt.password.verify(&passhash)
    })
    .await
    .map_err(|err| {
        tracing::error!(?err, "failed to spawn blocking task");
        ApiError::UnexpectedError
    })?
    .map_err(|err| match err {
        argon2::password_hash::Error::Password => {
            ApiError::AuthError("incorrect credentials".to_string())
        }
        _ => {
            tracing::error!(?err, "failed to verify password");
            ApiError::UnexpectedError
        }
    })?;

    let token = encode_jwt(row.id, row.username, state.jwt_signing_key.expose_secret()).map_err(
        |error| {
            tracing::error!(?error, "failed to encode jwt");
            ApiError::UnexpectedError
        },
    )?;

    Ok(Json(TokenResponse { token }))
}
