use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::{
    NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig,
};
use bevy_renet::renet::{ConnectionConfig, RenetServer};
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use crate::configuration::Settings;
use crate::server::EntityMoveEvent;

#[derive(Resource, Clone)]
pub struct DatabasePool(pub PgPool);

#[derive(Resource)]
pub struct EntityIdCounter(AtomicU32);

impl EntityIdCounter {
    pub fn increment(&mut self) -> u32 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for EntityIdCounter {
    fn default() -> Self {
        Self(AtomicU32::from(0))
    }
}

pub fn build(settings: Settings) -> Result<(App, u16), std::io::Error> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LogPlugin::default());
    app.add_plugins(RenetServerPlugin);
    app.add_plugins(NetcodeServerPlugin);
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
    public_addresses.push(server_addr);

    let port = socket.local_addr()?.port();

    let authentication = match settings.server.is_secure {
        true => ServerAuthentication::Secure {
            private_key: settings.server.netcode_private_key,
        },
        false => ServerAuthentication::Unsecure,
    };

    let netcode_server = RenetServer::new(ConnectionConfig::default());
    bevy::log::info!(
        "listening on {}; secure: {}",
        socket.local_addr()?,
        settings.server.is_secure
    );
    let netcode_transport = NetcodeServerTransport::new(
        ServerConfig {
            current_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            max_clients: 100,
            protocol_id: 0,
            public_addresses,
            authentication,
        },
        socket,
    )?;

    app.insert_resource(netcode_server);
    app.insert_resource(netcode_transport);
    app.insert_resource(settings);
    app.insert_resource(EntityIdCounter::default());

    app.add_event::<EntityMoveEvent>();

    // TODO: Implement server tick of 20Hz?
    app.add_systems(Startup, setup_database_pool);
    app.add_systems(
        Update,
        (
            crate::systems::handle_connection_events,
            crate::server::handle_server_messages,
            crate::server::handle_entity_move_events,
        ),
    );

    return Ok((app, port));
}

pub fn get_connection_pool(settings: &Settings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.database.with_db())
}

fn setup_database_pool(
    mut commands: Commands,
    runtime: Res<TokioTasksRuntime>,
    settings: Res<Settings>,
) {
    let pool = runtime
        .runtime()
        .block_on(async move { get_connection_pool(&settings) });
    commands.insert_resource(DatabasePool(pool));
}
