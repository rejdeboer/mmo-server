use bevy::prelude::*;
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use bevy_renet::renet::{ClientId, ConnectionConfig, RenetServer};
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Instant, SystemTime};

use crate::configuration::Settings;

#[derive(Resource, Clone)]
struct DatabasePool(PgPool);

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct EnterGamePayload {
    token: String,
    character_id: u32,
}

pub fn build(settings: Settings) -> Result<App, std::io::Error> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(RenetServerPlugin);
    app.add_plugins(TokioTasksPlugin::default());

    let ip_addr = IpAddr::V4(
        settings
            .server
            .host
            .parse()
            .expect("host should be IPV4 addr"),
    );
    let server_addr: SocketAddr = SocketAddr::new(ip_addr, settings.server.port);
    let socket = UdpSocket::bind(server_addr)?;
    let mut public_addresses: Vec<SocketAddr> = Vec::new();
    bevy::log::info!("listening on {}", server_addr);
    public_addresses.push(server_addr);

    let netcode_server = RenetServer::new(ConnectionConfig::default());
    let netcode_transport = NetcodeServerTransport::new(
        ServerConfig {
            current_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            max_clients: 100,
            protocol_id: 0,
            public_addresses,
            // TODO: Implement secure server using `settings.server.is_secure`
            authentication: ServerAuthentication::Unsecure,
        },
        socket,
    )?;

    app.insert_resource(netcode_server);
    app.insert_resource(netcode_transport);
    app.insert_resource(settings);
    app.add_systems(Startup, setup_database_pool);

    return Ok(app);
}

pub fn setup_database_pool(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>,
    settings: Res<Settings>,
) {
    let pool = runtime.runtime().block_on(async move {
        PgPoolOptions::new().connect_lazy_with(settings.database.with_db())
    });
    commands.insert_resource(DatabasePool(pool));
}
