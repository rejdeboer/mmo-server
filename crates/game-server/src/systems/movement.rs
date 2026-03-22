use crate::{
    components::ServerTick,
    messages::{JumpActionMessage, MoveActionMessage},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use game_core::{
    character_controller::{self, CharacterVelocityY},
    components::{GroundedComponent, MovementSpeedComponent},
    movement::MoveInput,
};

pub fn increment_server_tick(mut tick: ResMut<ServerTick>) {
    tick.next();
}

/// Process movement actions using the shared kinematic character controller.
///
/// Instead of setting `LinearVelocity` and letting the physics solver move
/// the entity, we call `character_move_step` directly. This produces the
/// same deterministic result as the client's prediction, making reconciliation
/// cheap (corrections only happen on actual desync, not floating-point drift).
pub fn process_move_action_messages(
    mut reader: MessageReader<MoveActionMessage>,
    spatial_query: SpatialQuery,
    mut q_player: Query<(
        Entity,
        &mut Transform,
        &Collider,
        &MovementSpeedComponent,
        &mut CharacterVelocityY,
        Has<GroundedComponent>,
    )>,
    mut commands: Commands,
) {
    reader.read().for_each(|msg| {
        let Ok((entity, mut transform, collider, movement_speed, mut vel_y, grounded)) =
            q_player.get_mut(msg.entity)
        else {
            tracing::error!(entity = ?msg.entity, "could not find entity for move action");
            return;
        };

        let input = MoveInput::from(msg.action.clone());
        let result = character_controller::character_move_step(
            transform.translation,
            vel_y.0,
            &input,
            movement_speed.0,
            grounded,
            collider,
            entity,
            &spatial_query,
        );

        // Apply the result.
        transform.translation = result.position;
        transform.rotation = Quat::from_rotation_y(result.yaw);
        vel_y.0 = result.velocity_y;

        // Update grounded status.
        if result.grounded {
            commands.entity(entity).insert(GroundedComponent);
        } else {
            commands.entity(entity).remove::<GroundedComponent>();
        }
    })
}

/// Process jump actions by setting the vertical velocity on the character.
///
/// The actual jump displacement is applied on the next tick when
/// `process_move_action_messages` calls `character_move_step`.
pub fn process_jump_action_messages(
    mut reader: MessageReader<JumpActionMessage>,
    mut q_player: Query<&mut CharacterVelocityY, With<GroundedComponent>>,
) {
    reader.read().for_each(|msg| {
        if let Ok(mut vel_y) = q_player.get_mut(msg.entity) {
            vel_y.0 = character_controller::JUMP_VELOCITY;
        }
    })
}
