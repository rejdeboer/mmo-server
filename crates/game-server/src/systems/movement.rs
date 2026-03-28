use crate::{
    components::{ClientIdComponent, ServerTick},
    messages::{JumpActionMessage, MoveActionMessage},
};
use avian3d::prelude::{Collider, ShapeHits, SpatialQuery};
use bevy::prelude::*;
use game_core::{
    character_controller::{self, CharacterVelocityY},
    components::{GroundedComponent, MovementSpeedComponent},
    movement::MoveInput,
};

pub fn increment_server_tick(mut tick: ResMut<ServerTick>) {
    tick.next();
}

// TODO: Validate movement
// TODO: Parallelism?
pub fn process_move_action_messages(
    mut reader: MessageReader<MoveActionMessage>,
    spatial_query: SpatialQuery,
    mut q_player: Query<(
        Entity,
        &mut Transform,
        &Collider,
        &MovementSpeedComponent,
        &mut CharacterVelocityY,
    )>,
    mut commands: Commands,
) {
    reader.read().for_each(|msg| {
        let Ok((entity, mut transform, collider, movement_speed, mut vel_y)) =
            q_player.get_mut(msg.entity)
        else {
            tracing::error!(entity = ?msg.entity, "could not find entity");
            return;
        };

        let input = MoveInput::from(msg.action.clone());
        let result = character_controller::character_move_step(
            transform.translation,
            vel_y.0,
            &input,
            movement_speed.0,
            collider,
            entity,
            &spatial_query,
        );

        transform.translation = result.position;
        transform.rotation = Quat::from_rotation_y(result.yaw);
        vel_y.0 = result.velocity_y;

        if result.grounded {
            commands.entity(entity).insert(GroundedComponent);
        } else {
            commands.entity(entity).remove::<GroundedComponent>();
        }
    })
}

pub fn process_jump_action_messages(
    mut reader: MessageReader<JumpActionMessage>,
    mut q_velocity: Query<&mut CharacterVelocityY, With<GroundedComponent>>,
) {
    reader.read().for_each(|msg| {
        if let Ok(mut vel_y) = q_velocity.get_mut(msg.entity) {
            vel_y.0 = character_controller::try_jump(vel_y.0, true);
        }
    })
}

// TODO: Do we still need this system?
pub fn check_ground_status(
    mut commands: Commands,
    query: Query<(Entity, &ShapeHits), With<ClientIdComponent>>,
) {
    for (entity, hits) in query.iter() {
        let mut is_grounded = false;

        for hit in hits.iter() {
            // NOTE: Check the slope angle.
            // If the normal is pointing up (Y > 0.7), it's a floor.
            // If Y is close to 0, it's a wall.
            if hit.normal1.y > 0.7 {
                is_grounded = true;
                break;
            }
        }

        if is_grounded {
            commands.entity(entity).insert(GroundedComponent);
        } else {
            commands.entity(entity).remove::<GroundedComponent>();
        }
    }
}
