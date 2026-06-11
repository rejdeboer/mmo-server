use super::components::{AiBrain, AiMovement, AiState};
use crate::components::Casting;
use avian3d::prelude::{Collider, SpatialQuery};
use bevy::prelude::*;
use game_core::{
    character_controller::{self, CharacterVelocityY},
    components::{GroundedComponent, MovementSpeedComponent},
};

/// Computes and applies AI movement using the same character_move_step as players.
#[allow(clippy::type_complexity)]
pub fn apply_ai_movement(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    mut q_mobs: Query<(
        Entity,
        &AiBrain,
        &AiMovement,
        &mut Transform,
        &Collider,
        &MovementSpeedComponent,
        &mut CharacterVelocityY,
        Option<&Casting>,
    )>,
) {
    for (entity, brain, movement, mut transform, collider, speed, mut vel_y, casting) in
        q_mobs.iter_mut()
    {
        // Don't move while casting a non-movable spell
        if let Some(cast) = casting
            && !cast.castable_while_moving
        {
            continue;
        }

        // Only move if we have a target position
        let Some(target_pos) = movement.target_position else {
            continue;
        };

        let mob_pos = transform.translation;
        let to_target = target_pos - mob_pos;
        let horizontal_dist = Vec3::new(to_target.x, 0.0, to_target.z).length();

        // Already close enough
        if horizontal_dist <= movement.stop_distance {
            continue;
        }

        // Calculate direction toward target
        let direction = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();
        let yaw = (-direction.x).atan2(-direction.z);

        // Speed varies by state: wander is slower, evade is faster
        let move_speed = match brain.state {
            AiState::Idle => speed.0 * 0.4,
            AiState::Evading => speed.0 * 2.0,
            _ => speed.0,
        };

        let input = game_core::movement::MoveInput {
            yaw,
            forward: 1.0,
            sideways: 0.0,
        };

        let result = character_controller::character_move_step(
            mob_pos,
            vel_y.0,
            &input,
            move_speed,
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
    }
}
