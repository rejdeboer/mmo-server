use crate::{
    components::{ClientIdComponent, GroundedComponent, MovementSpeedComponent},
    messages::{JumpActionMessage, MoveActionMessage},
};
use avian3d::prelude::{LinearVelocity, ShapeHits};
use bevy::prelude::*;
use std::f32::consts::TAU;

const YAW_QUANTIZATION_FACTOR: f32 = 65535.0;
const MOVEMENT_QUANTIZATION_FACTOR: f32 = 127.0;
const JUMP_VELOCITY: f32 = 3.;

// TODO: Validate movement
// TODO: Parallelism?
pub fn process_move_action_messages(
    mut reader: MessageReader<MoveActionMessage>,
    mut q_transform: Query<(&mut Transform, &mut LinearVelocity, &MovementSpeedComponent)>,
) {
    reader.read().for_each(|msg| {
        let Ok((mut transform, mut velocity, movement_speed)) = q_transform.get_mut(msg.entity)
        else {
            tracing::error!(entity = ?msg.entity, "could not find entity");
            return;
        };

        let yaw = msg.yaw as f32 / YAW_QUANTIZATION_FACTOR * TAU;
        transform.rotation = Quat::from_rotation_y(yaw);

        let forward = transform.forward();
        let right = transform.right();

        let forward_input = msg.forward as f32 / MOVEMENT_QUANTIZATION_FACTOR;
        let sideways_input = msg.sideways as f32 / MOVEMENT_QUANTIZATION_FACTOR;

        let direction = (forward * forward_input) + (right * sideways_input);
        let target_velocity = direction.normalize_or_zero() * movement_speed.0;
        velocity.x = target_velocity.x;
        velocity.z = target_velocity.z;
    })
}

pub fn process_jump_action_messages(
    mut reader: MessageReader<JumpActionMessage>,
    mut q_velocity: Query<&mut LinearVelocity, With<GroundedComponent>>,
) {
    reader.read().for_each(|msg| {
        if let Ok(mut velocity) = q_velocity.get_mut(msg.entity) {
            velocity.y = JUMP_VELOCITY;
        }
    })
}

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
