use crate::target::SelectedTarget;
use bevy::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use game_core::components::NetworkId;
use protocol::client::PlayerAction;

/// Sends a StartAttack action when the player presses T while having a target selected.
/// Sends StopAttack when pressing T with no target, or Escape to cancel.
pub fn send_attack_action(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedTarget>,
    targets: Query<&NetworkId>,
    mut client: ResMut<RenetClient>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        let action = PlayerAction::StopAttack;
        let encoded = bitcode::encode(&action);
        client.send_message(DefaultChannel::ReliableOrdered, encoded);
        return;
    }

    if !keyboard.just_pressed(KeyCode::KeyT) {
        return;
    }

    let Some(target_entity) = selected.0 else {
        // No target selected, stop attacking
        let action = PlayerAction::StopAttack;
        let encoded = bitcode::encode(&action);
        client.send_message(DefaultChannel::ReliableOrdered, encoded);
        return;
    };

    let Ok(network_id) = targets.get(target_entity) else {
        tracing::warn!("selected target has no NetworkId");
        return;
    };

    let action = PlayerAction::StartAttack {
        target_entity_id: network_id.0,
    };
    let encoded = bitcode::encode(&action);
    client.send_message(DefaultChannel::ReliableOrdered, encoded);
}
