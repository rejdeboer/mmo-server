use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgSslMode},
};

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ServerSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub db_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.db_name)
            .log_statements(tracing::log::LevelFilter::Trace)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("detirmined current directory");

    settings.merge(config::File::from(base_path.join("settings")).required(false))?;
    settings.merge(config::Environment::new().separator("_"))?;
    settings.try_into()
}
