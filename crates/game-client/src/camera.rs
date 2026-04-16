use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy::window::CursorOptions;

use crate::application::PlayerComponent;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default distance from the camera to the player pivot.
const DEFAULT_DISTANCE: f32 = 12.0;

/// Minimum zoom distance.
const MIN_DISTANCE: f32 = 3.0;

/// Maximum zoom distance.
const MAX_DISTANCE: f32 = 30.0;

/// Scroll wheel zoom speed (distance units per scroll line).
const ZOOM_SPEED: f32 = 1.5;

/// Mouse sensitivity (radians per pixel of mouse movement).
const MOUSE_SENSITIVITY: f32 = 0.003;

/// Minimum pitch (looking up). Slightly above zero to prevent gimbal issues.
const MIN_PITCH: f32 = -std::f32::consts::FRAC_PI_2 + 0.05;

/// Maximum pitch (looking down).
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;

/// Vertical offset above the player's position for the camera target.
/// Points roughly at shoulder/head height on a capsule(1, 2) character.
const CAMERA_TARGET_OFFSET: Vec3 = Vec3::new(0.0, 2.0, 0.0);

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Third-person orbit camera state (WoW-style).
///
/// - **Right-click drag**: orbits the camera around the character without
///   turning the character.
/// - **Left-click drag**: orbits the camera AND turns the character to face
///   the camera's forward direction.
/// - **Both buttons held**: character moves forward (like auto-run) while
///   the mouse steers.
/// - **Scroll wheel**: zoom in/out.
/// - Cursor is free when no mouse button is held.
#[derive(Component, Debug)]
pub struct ThirdPersonCamera {
    /// Horizontal orbit angle in radians.
    pub yaw: f32,
    /// Vertical orbit angle in radians (positive = looking down).
    pub pitch: f32,
    /// Distance from the camera to the player.
    pub distance: f32,
    /// Whether the character should turn to face the camera yaw this frame.
    /// Set by left-click drag or both-buttons-held.
    pub turn_character: bool,
    /// Whether both mouse buttons are held (triggers forward movement).
    pub both_buttons_move: bool,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.3, // Slightly looking down at the character
            distance: DEFAULT_DISTANCE,
            turn_character: false,
            both_buttons_move: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Handles WoW-style mouse input for the third-person camera.
///
/// - Right-click drag: orbit camera only.
/// - Left-click drag: orbit camera + turn character to face camera direction.
/// - Both buttons held: orbit + turn character + inject forward movement.
/// - Scroll wheel: zoom.
pub fn camera_input(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut q_camera: Query<&mut ThirdPersonCamera>,
) {
    let Ok(mut cam) = q_camera.single_mut() else {
        return;
    };

    let left = mouse_button.pressed(MouseButton::Left);
    let right = mouse_button.pressed(MouseButton::Right);
    let either = left || right;

    // Rotate camera when any mouse button is held.
    if either {
        cam.yaw -= mouse_motion.delta.x * MOUSE_SENSITIVITY;
        cam.pitch -= mouse_motion.delta.y * MOUSE_SENSITIVITY;
        cam.pitch = cam.pitch.clamp(MIN_PITCH, MAX_PITCH);
    }

    // Left-click (with or without right) turns the character to face camera yaw.
    cam.turn_character = left;

    // Both buttons held = move forward.
    cam.both_buttons_move = left && right;

    // Zoom with scroll wheel.
    if mouse_scroll.delta.y.abs() > 0.0 {
        cam.distance -= mouse_scroll.delta.y * ZOOM_SPEED;
        cam.distance = cam.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

/// Updates the camera transform to orbit around the player.
///
/// Runs after movement so the camera follows the post-prediction position.
pub fn update_camera_transform(
    q_player: Query<&Transform, (With<PlayerComponent>, Without<ThirdPersonCamera>)>,
    mut q_camera: Query<(&ThirdPersonCamera, &mut Transform), Without<PlayerComponent>>,
) {
    let Ok(player_transform) = q_player.single() else {
        return;
    };
    let Ok((cam, mut camera_transform)) = q_camera.single_mut() else {
        return;
    };

    let target = player_transform.translation + CAMERA_TARGET_OFFSET;

    // Compute camera position on the orbit sphere.
    let orbit_rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let offset = orbit_rotation * Vec3::new(0.0, 0.0, cam.distance);
    let camera_pos = target + offset;

    camera_transform.translation = camera_pos;
    camera_transform.look_at(target, Vec3::Y);
}

/// Grabs/releases the cursor based on mouse button state.
///
/// Cursor is locked and hidden while either mouse button is held,
/// free otherwise.
pub fn manage_cursor_grab(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut q_cursor: Query<&mut CursorOptions>,
) {
    let Ok(mut cursor) = q_cursor.single_mut() else {
        return;
    };

    let any_pressed =
        mouse_button.pressed(MouseButton::Left) || mouse_button.pressed(MouseButton::Right);

    if any_pressed && cursor.grab_mode != CursorGrabMode::Locked {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else if !any_pressed && cursor.grab_mode != CursorGrabMode::None {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}
