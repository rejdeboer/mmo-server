use bevy::prelude::*;
use web_client::WebClient;

pub struct WebApiPlugin {
    client: WebClient,
}

impl WebApiPlugin {
    pub fn new(base_url: String) -> Self {
        let client = WebClient::new(base_url);

        Self { client }
    }

    pub fn from_web_client(client: WebClient) -> Self {
        Self { client }
    }
}

impl Plugin for WebApiPlugin {
    fn build(&self, app: &mut App) {}
}
