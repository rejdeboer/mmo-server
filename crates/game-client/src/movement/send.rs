use crate::core::PlayerComponent;
use super::prediction::PredictionHistory;
use bevy::prelude::*;

pub fn send_player_input(
    mut client: ResMut<bevy_renet::RenetClient>,
    query: Query<&PredictionHistory, With<PlayerComponent>>,
) {
    let Ok(history) = query.single() else {
        return;
    };

    let Some(action) = history.input_buffer.back() else {
        return;
    };

    let should_send = history.latest_has_movement() || history.previous_had_movement();
    if !should_send {
        return;
    }

    let encoded = bitcode::encode(action);
    client.send_message(bevy_renet::renet::DefaultChannel::Unreliable, encoded);
}
