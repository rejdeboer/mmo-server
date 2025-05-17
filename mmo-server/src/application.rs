use bevy::MinimalPlugins;
use bevy::app::App;
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use bevy_renet::renet::{ConnectionConfig, RenetServer};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use crate::configuration::{DatabaseSettings, Settings};

pub fn build(settings: Settings) -> Result<App, std::io::Error> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(RenetServerPlugin);

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

    let netcode_server = RenetServer::new(ConnectionConfig::default());
    let netcode_transport = NetcodeServerTransport::new(
        ServerConfig {
            current_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            max_clients: 100,
            protocol_id: 0,
            public_addresses,
            // TODO: Implement secure server
            authentication: ServerAuthentication::Unsecure,
        },
        socket,
    )?;

    let _connection_pool = get_connection_pool(&settings.database);

    app.insert_resource(netcode_server);
    app.insert_resource(netcode_transport);
    // app.insert_resource(connection_pool);

    return Ok(app);
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(settings.with_db())
}
