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

/// Third-person orbit camera state.
///
/// The camera orbits around the player character at a configurable distance.
/// Yaw and pitch are controlled by mouse movement (right-click held).
/// Scroll wheel controls zoom distance.
///
/// The camera's yaw is also used as the movement direction reference —
/// WASD moves relative to where the camera is facing.
#[derive(Component, Debug)]
pub struct ThirdPersonCamera {
    /// Horizontal orbit angle in radians.
    pub yaw: f32,
    /// Vertical orbit angle in radians (positive = looking down).
    pub pitch: f32,
    /// Distance from the camera to the player.
    pub distance: f32,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.3, // Slightly looking down at the character
            distance: DEFAULT_DISTANCE,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Handles mouse input for camera rotation and scroll for zoom.
///
/// Camera rotation is active when right mouse button is held.
/// Scroll wheel adjusts zoom distance.
pub fn camera_input(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut q_camera: Query<&mut ThirdPersonCamera>,
) {
    let Ok(mut cam) = q_camera.single_mut() else {
        return;
    };

    // Rotate camera on right-mouse-button drag.
    if mouse_button.pressed(MouseButton::Right) {
        cam.yaw -= mouse_motion.delta.x * MOUSE_SENSITIVITY;
        cam.pitch -= mouse_motion.delta.y * MOUSE_SENSITIVITY;
        cam.pitch = cam.pitch.clamp(MIN_PITCH, MAX_PITCH);
    }

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

/// Grabs the cursor when right mouse button is pressed, releases it when
/// released. This provides a standard MMO camera feel.
pub fn manage_cursor_grab(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut q_cursor: Query<&mut CursorOptions>,
) {
    let Ok(mut cursor) = q_cursor.single_mut() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Right) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    if mouse_button.just_released(MouseButton::Right) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}
