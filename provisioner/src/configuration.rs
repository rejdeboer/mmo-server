use secrecy::{ExposeSecret, SecretString};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{ConnectOptions, postgres::PgConnectOptions};
use web_server::configuration::{NetcodePrivateKey, deserialize_netcode_key};

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub server: Option<ServerSettings>,
    pub database: Option<DatabaseSettings>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ServerSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    #[serde(deserialize_with = "deserialize_netcode_key")]
    pub netcode_private_key: NetcodePrivateKey,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub name: String,
}

impl DatabaseSettings {
    pub fn with_db(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .database(&self.name)
            .log_statements(tracing::log::LevelFilter::Trace)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("determined current directory");

    settings.merge(config::File::from(base_path.join("settings")).required(false))?;
    settings.merge(config::Environment::with_prefix("SEEDER").separator("_"))?;
    settings.try_into()
}
