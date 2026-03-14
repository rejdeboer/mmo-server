use http::StatusCode;
use web_types::{
    Account, AccountCreate, Character, CharacterCreate, GameEntryRequest, GameEntryResponse,
    LoginBody, TokenResponse,
};

#[derive(thiserror::Error, Debug)]
pub enum WebClientError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API returned an error ({status}): {message}")]
    ApiError { status: StatusCode, message: String },
    #[error("Not authenticated. Call login first.")]
    NotAuthenticated,
}

pub struct WebClient {
    base_url: String,
    api: reqwest::Client,
    jwt: Option<String>,
}

impl WebClient {
    pub fn new(base_url: String) -> Self {
        let api = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to build reqwest client; this only happens on bad TLS config");

        Self {
            base_url,
            api,
            jwt: None,
        }
    }

    pub async fn login(&mut self, body: &LoginBody) -> Result<(), WebClientError> {
        let url = format!("{}/token", self.base_url);

        let response = self.api.post(&url).json(body).send().await?;
        if !response.status().is_success() {
            return Err(WebClientError::ApiError {
                status: response.status(),
                message: response.text().await?,
            });
        }
        let login_response = response.json::<TokenResponse>().await?;

        self.jwt = Some(login_response.jwt);

        Ok(())
    }

    pub async fn select_character(&mut self, character_id: i32) -> Result<String, WebClientError> {
        let token = self
            .jwt
            .as_deref()
            .ok_or(WebClientError::NotAuthenticated)?;

        let url = format!("{}/game/request-entry", self.base_url);

        let response = self
            .api
            .post(&url)
            .bearer_auth(token)
            .json(&GameEntryRequest { character_id })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(WebClientError::ApiError {
                status: response.status(),
                message: response.text().await?,
            });
        }

        let login_response = response.json::<GameEntryResponse>().await?;

        self.jwt = Some(login_response.jwt);

        Ok(login_response.connect_token)
    }

    pub async fn create_character(
        &self,
        body: &CharacterCreate,
    ) -> Result<Character, WebClientError> {
        let token = self
            .jwt
            .as_deref()
            .ok_or(WebClientError::NotAuthenticated)?;
        let url = format!("{}/character", self.base_url);

        let response = self
            .api
            .post(&url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;
        let response = response.error_for_status()?;

        let character = response.json::<Character>().await?;

        Ok(character)
    }

    pub async fn create_account(&self, body: &AccountCreate) -> Result<Account, WebClientError> {
        let url = format!("{}/account", self.base_url);

        let response = self.api.post(&url).json(body).send().await?;
        let response = response.error_for_status()?;

        let account = response.json::<Account>().await?;

        Ok(account)
    }
}
