pub const BASE_MOVEMENT_SPEED: f32 = 7.5;

/// The fixed simulation and network tick rate in Hz, shared by client and server.
pub const TICK_RATE_HZ: f64 = 30.0;

/// Actor capsule collider radius.
pub const ACTOR_COLLIDER_RADIUS: f32 = 1.0;

/// Actor capsule collider segment length (cylindrical part, excludes hemisphere caps).
pub const ACTOR_COLLIDER_LENGTH: f32 = 2.0;

/// Distance from the capsule center to the bottom (feet).
/// Used to convert between ground-level positions (DB, spawn points) and
/// physics-center positions (Transform).
pub const ACTOR_HALF_HEIGHT: f32 = ACTOR_COLLIDER_LENGTH / 2.0 + ACTOR_COLLIDER_RADIUS;
