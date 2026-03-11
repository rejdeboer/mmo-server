use crate::domain::{EmailAddress, SafePassword, Username};
use crate::{ApplicationState, error::ApiError};
use argon2::password_hash::PasswordHashString;
use axum::Json;
use axum::extract::State;
use tracing::instrument;
use web_types::AccountCreate;

pub struct NewAccount {
    pub username: Username,
    pub email: EmailAddress,
    pub passhash: PasswordHashString,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccountRow {
    pub username: String,
    pub email: String,
}

impl From<AccountRow> for web_types::Account {
    fn from(value: AccountRow) -> Self {
        web_types::Account {
            username: value.username,
            email: value.email,
        }
    }
}

impl TryInto<NewAccount> for AccountCreate {
    type Error = String;

    fn try_into(self) -> Result<NewAccount, Self::Error> {
        let username = Username::parse(self.username)?;
        let email = EmailAddress::parse(self.email)?;
        let password = SafePassword::parse(self.password)?;
        let passhash = password.hash().map_err(|e| {
            // NOTE: This should never happen
            tracing::error!(?e, "error hashing password");
            "invalid password".to_string()
        })?;
        Ok(NewAccount {
            username,
            email,
            passhash,
        })
    }
}

#[instrument(skip_all, fields(email = payload.email, username = payload.username))]
pub async fn account_create(
    State(state): State<ApplicationState>,
    Json(payload): Json<AccountCreate>,
) -> Result<Json<web_types::Account>, ApiError> {
    let new_account: NewAccount = payload.try_into().map_err(ApiError::BadRequest)?;

    let row = sqlx::query_as!(
        AccountRow,
        r#"
        INSERT INTO accounts (username, email, passhash)
        VALUES ($1, $2, $3)
        RETURNING username, email 
        "#,
        new_account.username.as_ref(),
        new_account.email.as_ref(),
        new_account.passhash.as_str(),
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, "error creating account");
        // TODO: Handle specific error cases like account already exists
        ApiError::UnexpectedError
    })?;

    Ok(Json(row.into()))
}
