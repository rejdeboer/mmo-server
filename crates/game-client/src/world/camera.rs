use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy::window::CursorOptions;

use crate::core::PlayerComponent;
use super::selection::CursorOverUi;

const DEFAULT_DISTANCE: f32 = 12.0;
const MIN_DISTANCE: f32 = 3.0;
const MAX_DISTANCE: f32 = 30.0;
const ZOOM_SPEED: f32 = 1.5;
const MOUSE_SENSITIVITY: f32 = 0.003;
const MIN_PITCH: f32 = -std::f32::consts::FRAC_PI_2 + 0.05;
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const CAMERA_TARGET_OFFSET: Vec3 = Vec3::new(0.0, 2.0, 0.0);

#[derive(Component, Debug)]
pub struct ThirdPersonCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub turn_character: bool,
    pub both_buttons_move: bool,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.3,
            distance: DEFAULT_DISTANCE,
            turn_character: false,
            both_buttons_move: false,
        }
    }
}

pub fn camera_input(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut q_camera: Query<&mut ThirdPersonCamera>,
    cursor_over_ui: Res<CursorOverUi>,
) {
    let Ok(mut cam) = q_camera.single_mut() else {
        return;
    };

    let left = mouse_button.pressed(MouseButton::Left) && !cursor_over_ui.0;
    let right = mouse_button.pressed(MouseButton::Right) && !cursor_over_ui.0;
    let either = left || right;

    if either {
        cam.yaw -= mouse_motion.delta.x * MOUSE_SENSITIVITY;
        cam.pitch -= mouse_motion.delta.y * MOUSE_SENSITIVITY;
        cam.pitch = cam.pitch.clamp(MIN_PITCH, MAX_PITCH);
    }

    cam.turn_character = left;
    cam.both_buttons_move = left && right;

    if mouse_scroll.delta.y.abs() > 0.0 {
        cam.distance -= mouse_scroll.delta.y * ZOOM_SPEED;
        cam.distance = cam.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

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

    let orbit_rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let offset = orbit_rotation * Vec3::new(0.0, 0.0, cam.distance);
    let camera_pos = target + offset;

    camera_transform.translation = camera_pos;
    camera_transform.look_at(target, Vec3::Y);
}

pub fn manage_cursor_grab(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut q_cursor: Query<&mut CursorOptions>,
    cursor_over_ui: Res<CursorOverUi>,
) {
    let Ok(mut cursor) = q_cursor.single_mut() else {
        return;
    };

    let any_pressed = !cursor_over_ui.0
        && (mouse_button.pressed(MouseButton::Left) || mouse_button.pressed(MouseButton::Right));

    if any_pressed && cursor.grab_mode != CursorGrabMode::Locked {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else if !any_pressed && cursor.grab_mode != CursorGrabMode::None {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}
