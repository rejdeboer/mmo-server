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

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub realm_resolver: RealmResolverSettings,
    pub telemetry: TelemetrySettings,
    pub metrics: Option<MetricsSettings>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub jwt_signing_key: SecretString,
    #[serde(default, deserialize_with = "deserialize_netcode_key")]
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
    pub require_ssl: bool,
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TracingFormat {
    Pretty,
    Json,
}

#[derive(serde::Deserialize, Clone)]
pub struct TelemetrySettings {
    pub tracing_format: TracingFormat,
    pub otel_exporter_endpoint: Option<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct MetricsSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(serde::Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum RealmResolverSettings {
    Kube,
    Local(LocalResolverSettings),
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct LocalResolverSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(Clone, Default)]
pub struct NetcodePrivateKey([u8; 32]);

impl AsRef<[u8; 32]> for NetcodePrivateKey {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
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
            .log_statements(tracing::log::LevelFilter::Trace)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("determined current directory");
    let configuration_directory = base_path.join("configuration");

    settings.merge(config::File::from(configuration_directory.join("base")).required(true))?;

    let environment = Environment::read();
    settings.merge(
        config::File::from(configuration_directory.join(environment.as_str())).required(true),
    )?;

    settings.merge(config::Environment::with_prefix("app").separator("__"))?;

    settings.try_into()
}

pub fn deserialize_netcode_key<'de, D>(deserializer: D) -> Result<NetcodePrivateKey, D::Error>
where
    D: Deserializer<'de>,
{
    let mut netcode_private_key: [u8; 32] = [0; 32];
    let encoded: String = Deserialize::deserialize(deserializer)?;
    base64::decode_config_slice(encoded, base64::STANDARD, &mut netcode_private_key)
        .map_err(serde::de::Error::custom)?;

    Ok(NetcodePrivateKey(netcode_private_key))
}
