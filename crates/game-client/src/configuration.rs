use bevy::prelude::Resource;

#[derive(Resource, serde::Deserialize, Clone)]
pub struct Settings {
    pub web_server: WebServerSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct WebServerSettings {
    pub endpoint: String,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("determined current directory");

    settings.merge(config::File::from(base_path.join("settings")).required(true))?;
    settings.merge(config::Environment::with_prefix("APP").separator("__"))?;
    settings.try_into()
}
