use bevy::ecs::resource::Resource;
use bevy_renet::netcode::NETCODE_KEY_BYTES;
use config::ConfigError;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgSslMode},
};

pub enum Environment {
    Local,
    Staging,
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
    #[serde(default, deserialize_with = "deserialize_netcode_key")]
    pub netcode_private_key: Option<[u8; NETCODE_KEY_BYTES]>,
    pub metrics_path: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub name: String,
    pub require_ssl: bool,
}

impl Settings {
    pub fn validate(&self, environment: &Environment) -> Result<(), ConfigError> {
        if !matches!(environment, Environment::Local) && self.server.netcode_private_key.is_none() {
            return Err(ConfigError::Message(
                "private key is required outside of local env".to_string(),
            ));
        }
        Ok(())
    }
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Staging => "staging",
            Environment::Production => "production",
        }
    }

    pub fn read() -> Self {
        std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "local".into())
            .try_into()
            .expect("parsed ENVIRONMENT")
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. Use either `local`, `staging`, or `production`."
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
            .database(&self.name)
            .log_statements(bevy::log::tracing::log::LevelFilter::Trace)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("determined current directory");
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

    let settings: Settings = settings.try_into()?;
    settings.validate(&environment)?;
    Ok(settings)
}

fn deserialize_netcode_key<'de, D>(
    deserializer: D,
) -> Result<Option<[u8; NETCODE_KEY_BYTES]>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded: Option<String> = Deserialize::deserialize(deserializer)?;
    let encoded = match encoded {
        None => return Ok(None),
        Some(s) if s.trim().is_empty() => return Ok(None),
        Some(s) => s,
    };

    let mut buf = [0u8; NETCODE_KEY_BYTES];
    let n = base64::decode_config_slice(&encoded, base64::STANDARD, &mut buf)
        .map_err(serde::de::Error::custom)?;

    if n != NETCODE_KEY_BYTES {
        return Err(serde::de::Error::custom(format!(
            "invalid netcode key length: expected {} bytes, got {}",
            NETCODE_KEY_BYTES, n
        )));
    }
    Ok(Some(buf))
}
