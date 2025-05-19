use std::time::Instant;

use bevy::prelude::*;
use bevy_renet::renet::{ClientId, RenetServer, ServerEvent};

#[derive(Debug, Component)]
pub struct Player {
    pub id: ClientId,
}

#[derive(Component)]
struct PendingConnection {
    client_id: ClientId,
    initiated_at: Instant,
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
