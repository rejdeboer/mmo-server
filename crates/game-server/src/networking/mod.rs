mod action;
mod connection;
pub mod messages;
mod sync;
mod visibility;

use crate::configuration::ServerSettings;
use bevy::prelude::*;
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::{
    NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig,
};
use bevy_renet::{RenetServer, renet::ConnectionConfig};
use game_core::networking::{NetworkId, NetworkIdMapping};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::SystemTime;

pub use messages::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkingSet {
    /// Decode raw network packets into typed messages.
    ReceiveInput,
    /// Update visibility and interest management.
    UpdateVisibility,
    /// Send state updates to clients.
    Sync,
}

pub struct NetworkingPlugin {
    settings: ServerSettings,
}

impl NetworkingPlugin {
    pub fn new(settings: &ServerSettings) -> Self {
        Self {
            settings: settings.clone(),
        }
    }
}

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        // Transport setup
        let host_ip_addr = IpAddr::V4(
            self.settings
                .host
                .parse()
                .expect("host should be IPV4 addr"),
        );
        let host_addr = SocketAddr::new(host_ip_addr, self.settings.port);
        let socket = UdpSocket::bind(host_addr).expect("failed to bind UDP socket");

        let public_ip_addr = IpAddr::V4(
            self.settings
                .public_host
                .as_ref()
                .expect("public host should be set")
                .parse()
                .expect("public host should be IPV4 addr"),
        );
        let public_addr = SocketAddr::new(
            public_ip_addr,
            self.settings
                .public_port
                .expect("public port should be set"),
        );

        let authentication = match self.settings.netcode_private_key {
            Some(private_key) => ServerAuthentication::Secure { private_key },
            None => {
                warn!("running in unsecure mode");
                ServerAuthentication::Unsecure
            }
        };

        let netcode_server = RenetServer::new(ConnectionConfig::default());
        info!("listening on {}", socket.local_addr().unwrap());
        let netcode_transport = NetcodeServerTransport::new(
            ServerConfig {
                current_time: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap(),
                max_clients: 100,
                protocol_id: 0,
                public_addresses: vec![public_addr],
                authentication,
            },
            socket,
        )
        .expect("failed to create netcode transport");

        app.add_plugins((RenetServerPlugin, NetcodeServerPlugin));
        app.insert_resource(netcode_server);
        app.insert_resource(netcode_transport);

        // Messages
        app.add_message::<OutgoingMessage>();
        app.add_message::<VisibilityChangedMessage>();

        // Systems
        app.add_systems(
            FixedPreUpdate,
            (
                action::process_client_actions,
                action::process_client_movements,
            )
                .chain()
                .in_set(NetworkingSet::ReceiveInput),
        );

        app.add_systems(
            FixedPostUpdate,
            (visibility::update_player_visibility, sync::sync_visibility)
                .chain()
                .in_set(NetworkingSet::UpdateVisibility),
        );
        app.add_systems(
            FixedPostUpdate,
            (sync::sync_server_events, sync::sync_movement).in_set(NetworkingSet::Sync),
        );

        app.add_observer(connection::on_connection_event);
        app.add_observer(cleanup_network_entity_map);
    }
}

fn cleanup_network_entity_map(
    trigger: On<Remove, NetworkId>,
    q_network_ids: Query<&NetworkId>,
    mut net_entity_map: ResMut<NetworkIdMapping>,
) {
    let entity = trigger.event_target();
    if let Ok(network_id) = q_network_ids.get(entity) {
        net_entity_map.0.remove(network_id);
    }
}
