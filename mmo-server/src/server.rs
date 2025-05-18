use bevy::prelude::*;
use bevy_renet::renet::{ClientId, RenetServer, ServerEvent};

#[derive(Debug, Component)]
pub struct Player {
    pub id: ClientId,
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
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                bevy::log::info!("player {} disconnected: {}", client_id, reason);
            }
        }
    }
}
