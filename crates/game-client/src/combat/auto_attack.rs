use super::IsAttacking;
use crate::input::EscapePressed;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use game_core::components::NetworkId;
use protocol::client::PlayerAction;

/// Triggered when the player right-clicks a remote actor to start auto-attacking.
#[derive(Event)]
pub struct AttackTarget(pub Entity);

pub fn on_attack_target(
    event: On<AttackTarget>,
    network_ids: Query<&NetworkId>,
    mut is_attacking: ResMut<IsAttacking>,
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
    is_attacking.0 = true;
}

pub fn on_escape(
    _event: On<Start<EscapePressed>>,
    mut is_attacking: ResMut<IsAttacking>,
    mut client: ResMut<RenetClient>,
) {
    if is_attacking.0 {
        let action = PlayerAction::StopAttack;
        let encoded = bitcode::encode(&action);
        client.send_message(DefaultChannel::ReliableOrdered, encoded);
        is_attacking.0 = false;
    } else {
        // TODO: Open options menu
    }
}
