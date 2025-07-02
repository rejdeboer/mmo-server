use bevy::ecs::resource::Resource;
use bevy_renet::netcode::NETCODE_KEY_BYTES;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgSslMode},
};

pub enum Environment {
    Local,
    Production,
}

#[derive(serde::Deserialize, Clone, Resource)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ServerSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub is_secure: bool,
    #[serde(deserialize_with = "deserialize_netcode_key")]
    pub netcode_private_key: [u8; NETCODE_KEY_BYTES],
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

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. Use either `local` or `production`."
            )),
        }
    }
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.db_name)
            .log_statements(tracing::log::LevelFilter::Trace)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("detirmined current directory");
    let configuration_directory = base_path.join("configuration");

    settings.merge(config::File::from(configuration_directory.join("base")).required(true))?;

    let environment: Environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("parsed ENVIRONMENT");

    settings.merge(
        config::File::from(configuration_directory.join(environment.as_str())).required(true),
    )?;

    settings.merge(config::Environment::with_prefix("app").separator("__"))?;

    settings.try_into()
}

fn deserialize_netcode_key<'de, D>(deserializer: D) -> Result<[u8; NETCODE_KEY_BYTES], D::Error>
where
    D: Deserializer<'de>,
{
    let mut netcode_private_key: [u8; 32] = [0; 32];
    let encoded: String = Deserialize::deserialize(deserializer)?;
    base64::decode_config_slice(encoded, base64::STANDARD, &mut netcode_private_key)
        .map_err(serde::de::Error::custom)?;

    Ok(netcode_private_key)
}
