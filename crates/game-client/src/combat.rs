use crate::ui::ChatInputState;
use bevy::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use game_core::components::NetworkId;
use protocol::client::PlayerAction;

/// Triggered when the player right-clicks a remote actor to start auto-attacking.
#[derive(Event)]
pub struct AttackTarget(pub Entity);

/// Sends StopAttack when the player presses Escape.
pub fn send_stop_attack(
    keyboard: Res<ButtonInput<KeyCode>>,
    chat_state: Res<ChatInputState>,
    mut client: ResMut<RenetClient>,
) {
    if chat_state.active {
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        let action = PlayerAction::StopAttack;
        let encoded = bitcode::encode(&action);
        client.send_message(DefaultChannel::ReliableOrdered, encoded);
    }
}

/// Observer that sends StartAttack to the server when AttackTarget is triggered.
pub fn on_attack_target(
    event: On<AttackTarget>,
    network_ids: Query<&NetworkId>,
    mut client: ResMut<RenetClient>,
) {
    let Ok(network_id) = network_ids.get(event.0) else {
        tracing::warn!("attack target has no NetworkId");
        return;
    };

    let action = PlayerAction::StartAttack {
        target_entity_id: network_id.0,
    };
    let encoded = bitcode::encode(&action);
    client.send_message(DefaultChannel::ReliableOrdered, encoded);
}
