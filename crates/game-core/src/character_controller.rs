use avian3d::prelude::*;
use bevy::prelude::*;

use crate::constants::TICK_RATE_HZ;
use crate::movement::MoveInput;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Gravitational acceleration applied each tick (units/s^2, downward).
pub const GRAVITY: f32 = -19.6;

/// Jump velocity (units/s, upward).
pub const JUMP_VELOCITY: f32 = 8.0;

/// Maximum number of slide iterations per movement step.
/// Prevents infinite loops when wedged in geometry corners.
const MAX_SLIDE_ITERATIONS: u32 = 4;

/// Small skin width to prevent the capsule from getting flush against surfaces.
/// The shape cast stops this distance from the hit surface.
const SKIN_WIDTH: f32 = 0.01;

/// Maximum slope angle (in radians) that the character can walk on.
/// Steeper surfaces are treated as walls (the character slides along them).
pub const MAX_SLOPE_ANGLE: f32 = std::f32::consts::FRAC_PI_4; // 45 degrees

/// Fixed timestep duration in seconds.
pub const FIXED_DT: f32 = 1.0 / TICK_RATE_HZ as f32;

/// Maximum downward cast distance for ground detection.
const GROUND_CHECK_DISTANCE: f32 = 0.15;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// The character's vertical velocity, managed manually since we use a
/// kinematic rigid body (not affected by the physics solver's gravity).
#[derive(Component, Debug, Clone, Copy)]
pub struct CharacterVelocityY(pub f32);

impl Default for CharacterVelocityY {
    fn default() -> Self {
        Self(0.0)
    }
}

// ---------------------------------------------------------------------------
// Character movement step — the core shared function
// ---------------------------------------------------------------------------

/// Result of a single movement step.
#[derive(Debug, Clone)]
pub struct MoveResult {
    pub position: Vec3,
    pub yaw: f32,
    pub velocity_y: f32,
    pub grounded: bool,
}

/// Perform one tick of character movement with collision detection.
///
/// This is the **single source of truth** for character movement, used by both
/// the client (for prediction and replay) and the server (for authoritative
/// simulation). Because it uses stateless `SpatialQuery` calls instead of the
/// physics solver, it can be called multiple times in sequence for input replay
/// without requiring a physics step between calls.
///
/// ## Algorithm
///
/// 1. Apply gravity to vertical velocity.
/// 2. Compute horizontal velocity from the input.
/// 3. Combine into a 3D displacement vector (`velocity * dt`).
/// 4. Run a collide-and-slide loop using capsule shape casts.
/// 5. Check for ground contact below the final position.
/// 6. If grounded, zero out negative vertical velocity and snap to the surface.
///
/// ## Parameters
///
/// - `position`: Current world-space position.
/// - `velocity_y`: Current vertical velocity (positive = up).
/// - `input`: The movement input for this tick.
/// - `movement_speed`: The character's horizontal movement speed.
/// - `grounded`: Whether the character was grounded last tick.
/// - `shape`: The character's collision shape (capsule).
/// - `entity`: The character's entity (excluded from spatial queries).
/// - `spatial_query`: Avian3d's spatial query system parameter.
pub fn character_move_step(
    position: Vec3,
    velocity_y: f32,
    input: &MoveInput,
    movement_speed: f32,
    _grounded: bool,
    shape: &Collider,
    entity: Entity,
    spatial_query: &SpatialQuery,
) -> MoveResult {
    let dt = FIXED_DT;

    // --- 1. Apply gravity ---
    let mut vy = velocity_y + GRAVITY * dt;

    // --- 2. Compute horizontal velocity from input ---
    let horizontal_velocity = input.target_velocity(movement_speed);

    // --- 3. Combine into displacement ---
    let displacement = Vec3::new(
        horizontal_velocity.x * dt,
        vy * dt,
        horizontal_velocity.z * dt,
    );

    // --- 4. Collide-and-slide ---
    let filter = SpatialQueryFilter::from_excluded_entities([entity]);
    let new_position = move_and_slide(position, displacement, shape, &filter, spatial_query);

    // --- 5. Ground check ---
    let ground_check_shape = shape.clone();
    let config = ShapeCastConfig::from_max_distance(GROUND_CHECK_DISTANCE);
    let is_grounded = spatial_query
        .cast_shape(
            &ground_check_shape,
            new_position,
            Quat::IDENTITY,
            Dir3::NEG_Y,
            &config,
            &filter,
        )
        .is_some_and(|hit| {
            // Check slope angle — only count as grounded if the surface is walkable.
            hit.normal1.angle_between(Vec3::Y) <= MAX_SLOPE_ANGLE
        });

    // --- 6. Snap to ground / zero vertical velocity ---
    let mut final_position = new_position;
    if is_grounded && vy <= 0.0 {
        // Snap the character down to the ground surface to prevent floating.
        if let Some(hit) = spatial_query.cast_shape(
            &ground_check_shape,
            new_position,
            Quat::IDENTITY,
            Dir3::NEG_Y,
            &config,
            &filter,
        ) {
            if hit.distance > SKIN_WIDTH {
                final_position.y -= hit.distance - SKIN_WIDTH;
            }
        }
        vy = 0.0;
    }

    MoveResult {
        position: final_position,
        yaw: input.yaw,
        velocity_y: vy,
        grounded: is_grounded,
    }
}

/// Perform a jump if the character is grounded.
///
/// Returns the new vertical velocity. Call this before `character_move_step`
/// in the same tick to apply the jump on the same frame as the input.
pub fn try_jump(velocity_y: f32, grounded: bool) -> f32 {
    if grounded {
        JUMP_VELOCITY
    } else {
        velocity_y
    }
}

// ---------------------------------------------------------------------------
// Collide-and-slide implementation
// ---------------------------------------------------------------------------

/// Move a shape through the world, sliding along surfaces on collision.
///
/// Uses iterative shape casts: sweep the shape along the remaining displacement,
/// and on collision, remove the component of displacement along the hit normal.
/// Repeats up to `MAX_SLIDE_ITERATIONS` times.
fn move_and_slide(
    start: Vec3,
    displacement: Vec3,
    shape: &Collider,
    filter: &SpatialQueryFilter,
    spatial_query: &SpatialQuery,
) -> Vec3 {
    let mut position = start;
    let mut remaining = displacement;

    for _ in 0..MAX_SLIDE_ITERATIONS {
        let distance = remaining.length();
        if distance < 0.001 {
            break;
        }

        let Ok(direction) = Dir3::new(remaining) else {
            break;
        };

        let config = ShapeCastConfig::from_max_distance(distance);

        if let Some(hit) =
            spatial_query.cast_shape(shape, position, Quat::IDENTITY, direction, &config, filter)
        {
            // Move to just before the hit surface.
            let safe_distance = (hit.distance - SKIN_WIDTH).max(0.0);
            position += direction.as_vec3() * safe_distance;

            // Remove the displacement we've already covered.
            let moved = direction.as_vec3() * hit.distance;
            remaining -= moved;

            // Slide: project the remaining displacement onto the hit surface.
            // This removes the component that would push us into the surface.
            let normal = hit.normal1.normalize_or_zero();
            remaining -= normal * remaining.dot(normal);
        } else {
            // No collision — move the full remaining distance.
            position += remaining;
            break;
        }
    }

    position
}
