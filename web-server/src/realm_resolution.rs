use std::net::{IpAddr, SocketAddr};

use async_trait::async_trait;
use kube::{Api, Client, CustomResource, api::ListParams};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;
use tracing::instrument;

use crate::{
    configuration::{LocalResolverSettings, RealmResolverSettings},
    error::ApiError,
};

impl From<GameServer> for SocketAddr {
    fn from(value: GameServer) -> Self {
        let status = value
            .status
            .expect("realm resource should have status field");
        let ip_addr = IpAddr::V4(status.address.parse().expect("host should be IPV4 addr"));
        SocketAddr::new(ip_addr, status.ports[0].port)
    }
}

#[async_trait]
pub trait RealmResolver: Send + Sync {
    async fn resolve(&self, realm_id: &str) -> Result<SocketAddr, ApiError>;
}

pub struct LocalResolver {
    host: String,
    port: u16,
}

impl LocalResolver {
    pub fn new(settings: LocalResolverSettings) -> Self {
        Self {
            host: settings.host,
            port: settings.port,
        }
    }
}

#[async_trait]
impl RealmResolver for LocalResolver {
    async fn resolve(&self, _realm_id: &str) -> Result<SocketAddr, ApiError> {
        tracing::info!("locally resolving realm");
        let ip_addr = IpAddr::V4(self.host.parse().expect("host should be IPV4 addr"));
        Ok(SocketAddr::new(ip_addr, self.port))
    }
}

pub struct KubeResolver {
    api: Api<GameServer>,
}

impl KubeResolver {
    pub fn new(client: Client) -> Self {
        let api = Api::default_namespaced(client);
        Self { api }
    }
}

#[async_trait]
impl RealmResolver for KubeResolver {
    #[instrument(skip(self))]
    async fn resolve(&self, realm_id: &str) -> Result<SocketAddr, ApiError> {
        tracing::info!("resolving via kube");
        let params = ListParams::default().labels(&format!("realm={realm_id}"));
        let results = self.api.list(&params).await.map_err(|err| {
            tracing::error!(?err, "failed to fetch realm");
            ApiError::UnexpectedError
        })?;

        if results.items.is_empty() {
            tracing::error!("realm not found");
            return Err(ApiError::UnexpectedError);
        }

        Ok(results.items[0].to_owned().into())
    }
}

#[derive(CustomResource, Serialize, Debug, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "agones.dev",
    version = "v1",
    kind = "GameServer",
    namespaced,
    status = "GameServerStatus"
)]
pub struct GameServerSpec {}

#[derive(Debug, Deserialize, Serialize, Default, Clone, JsonSchema)]
pub struct GameServerStatus {
    address: String,
    ports: Vec<GameServerPort>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct GameServerPort {
    name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    port: u16,
}

pub async fn create_realm_resolver(settings: &RealmResolverSettings) -> Box<dyn RealmResolver> {
    match settings {
        RealmResolverSettings::Kube => {
            let client = Client::try_default()
                .await
                .expect("failed to create kube client");
            Box::new(KubeResolver::new(client))
        }
        RealmResolverSettings::Local(local_settings) => {
            Box::new(LocalResolver::new(local_settings.clone()))
        }
    }
}
