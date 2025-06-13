use crate::domain::{EmailAddress, Password, Username};
use crate::{error::ApiError, ApplicationState};
use axum::Json;
use axum::{extract::State, response::Response};

pub struct AccountCreate {
    pub username: String,
    pub email: String,
    pub password: String,
}

pub struct NewAccount {
    pub username: Username,
    pub email: EmailAddress,
    pub password: Password,
}

impl TryInto<NewAccount> for AccountCreate {
    type Error = String;

    fn try_into(self) -> Result<NewAccount, Self::Error> {
        let username = Username::parse(self.username)?;
        let email = EmailAddress::parse(self.email)?;
        let password = Password::parse(self.password)?;
        Ok(NewAccount {
            username,
            email,
            password,
        })
    }
}

pub async fn account_create(
    State(state): State<ApplicationState>,
    Json(payload): Json<AccountCreate>,
) -> Result<Response, ApiError> {
    let new_account = payload.try_into().map_err(ApiError::BadRequest)?;
}
