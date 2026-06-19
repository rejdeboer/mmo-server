mod prediction;
mod reconciliation;
mod interpolation;
mod send;

pub use prediction::{PredictionHistory, PredictedState};
pub use reconciliation::reconcile_with_server;
pub use interpolation::{RemoteInterpolation, interpolate_remote_actors};
pub use send::send_player_input;

use bevy::prelude::*;

use crate::application::AppState;
use crate::networking::NetworkingSet;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MovementSet {
    Predict,
    Send,
    Reconcile,
    Interpolate,
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedPreUpdate,
            prediction::predict_player_movement
                .in_set(MovementSet::Predict)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            FixedPostUpdate,
            send::send_player_input
                .in_set(MovementSet::Send)
                .after(avian3d::prelude::PhysicsSystems::Last)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (
                reconciliation::reconcile_with_server.in_set(MovementSet::Reconcile),
                interpolation::interpolate_remote_actors.in_set(MovementSet::Interpolate),
            )
                .after(NetworkingSet::Receive)
                .run_if(in_state(AppState::InGame)),
        );
    }
}
