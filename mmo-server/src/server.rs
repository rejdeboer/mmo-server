use std::time::Instant;

use bevy::prelude::*;
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer, ServerEvent};

#[derive(Debug, Component)]
pub struct Player {
    pub id: ClientId,
}

#[derive(Component)]
pub struct PendingConnection {
    client_id: ClientId,
    initiated_at: Instant,
}

#[derive(bincode::Decode, Debug)]
struct ClientHandshake {
    token: String,
    character_id: u32,
}

#[derive(Event, Debug)]
pub struct ProcessClientHandshake {
    client_id: ClientId,
    token: String,
    character_id: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn update_system(
    mut events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    players: Query<(Entity, &Player, &Transform)>,
) {
    for event in events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                bevy::log::info!("player {} connected", client_id);
                commands.spawn(PendingConnection {
                    client_id: *client_id,
                    initiated_at: Instant::now(),
                });
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                bevy::log::info!("player {} disconnected: {}", client_id, reason);
            }
        }
    }
}

pub fn receive_initial_handshake_messages(
    mut server: ResMut<RenetServer>,
    pending_connections_query: Query<(Entity, &PendingConnection)>,
    mut event_writer: EventWriter<ProcessClientHandshake>,
    mut commands: Commands,
) {
    for (entity, pending_conn) in pending_connections_query.iter() {
        let client_id = pending_conn.client_id;
        while let Some(message_bytes) =
            server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            match bincode::decode_from_slice::<ClientHandshake, _>(
                &message_bytes,
                bincode::config::standard(),
            ) {
                Ok((decoded, _len)) => {
                    bevy::log::info!(
                        "received handshake from client {}: character_id {}, token {}",
                        client_id,
                        decoded.character_id,
                        decoded.token
                    );
                    event_writer.write(ProcessClientHandshake {
                        client_id,
                        character_id: decoded.character_id,
                        token: decoded.token,
                    });
                    commands.entity(entity).despawn();
                }
                Err(e) => {
                    bevy::log::error!(
                        "failed to deserialize handshake from client {}: {}. disconnecting.",
                        client_id,
                        e
                    );
                    server.disconnect(client_id);
                    commands.entity(entity).despawn();
                }
            }
            break;
        }
    }
}
