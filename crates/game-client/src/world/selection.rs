use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::combat::AttackTarget;
use crate::combat::IsAttacking;
use crate::movement::RemoteInterpolation;
use crate::theme::widgets::UnitFrame;

/// The currently selected target entity.
#[derive(Resource, Default)]
pub struct SelectedTarget(pub Option<Entity>);

/// Whether the cursor is currently over a game UI element.
#[derive(Resource, Default)]
pub struct CursorOverUi(pub bool);

/// Tracks whether mouse button presses were clicks (not drags).
#[derive(Resource, Default)]
pub struct ClickTracker {
    pressed: bool,
    start_position: Vec2,
    dragged: bool,
    right_pressed: bool,
    right_start_position: Vec2,
    right_dragged: bool,
}

const DRAG_THRESHOLD: f32 = 5.0;

pub fn update_cursor_over_ui(
    interactables: Query<&Interaction, Or<(With<UnitFrame>, With<Button>)>>,
    mut cursor_over_ui: ResMut<CursorOverUi>,
) {
    cursor_over_ui.0 = interactables.iter().any(|i| *i != Interaction::None);
}

pub fn handle_target_selection(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    rapier_context: SpatialQuery,
    remote_actors: Query<Entity, With<RemoteInterpolation>>,
    mut selected: ResMut<SelectedTarget>,
    mut click_tracker: ResMut<ClickTracker>,
    cursor_over_ui: Res<CursorOverUi>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left)
        && let Some(pos) = window.cursor_position()
    {
        click_tracker.pressed = true;
        click_tracker.start_position = pos;
        click_tracker.dragged = false;
    }

    if click_tracker.pressed
        && let Some(pos) = window.cursor_position()
        && pos.distance(click_tracker.start_position) > DRAG_THRESHOLD
    {
        click_tracker.dragged = true;
    }

    if mouse_button.just_pressed(MouseButton::Right)
        && let Some(pos) = window.cursor_position()
    {
        click_tracker.right_pressed = true;
        click_tracker.right_start_position = pos;
        click_tracker.right_dragged = false;
    }

    if click_tracker.right_pressed
        && let Some(pos) = window.cursor_position()
        && pos.distance(click_tracker.right_start_position) > DRAG_THRESHOLD
    {
        click_tracker.right_dragged = true;
    }

    if mouse_button.just_released(MouseButton::Left) {
        let was_click = click_tracker.pressed && !click_tracker.dragged;
        click_tracker.pressed = false;

        if was_click && !cursor_over_ui.0 {
            if let Some(hit_entity) = raycast_remote_actor(
                window,
                &cameras,
                &rapier_context,
                &remote_actors,
            ) {
                selected.0 = Some(hit_entity);
            } else {
                selected.0 = None;
            }
        }
    }

    if mouse_button.just_released(MouseButton::Right) {
        let was_click = click_tracker.right_pressed && !click_tracker.right_dragged;
        click_tracker.right_pressed = false;

        if was_click && !cursor_over_ui.0 {
            if let Some(hit_entity) = raycast_remote_actor(
                window,
                &cameras,
                &rapier_context,
                &remote_actors,
            ) {
                selected.0 = Some(hit_entity);
                commands.trigger(AttackTarget(hit_entity));
            }
        }
    }
}

fn raycast_remote_actor(
    window: &Window,
    cameras: &Query<(&Camera, &GlobalTransform)>,
    rapier_context: &SpatialQuery,
    remote_actors: &Query<Entity, With<RemoteInterpolation>>,
) -> Option<Entity> {
    let cursor_pos = window.cursor_position()?;
    let (camera, camera_transform) = cameras.single().ok()?;
    let ray = camera.viewport_to_world(camera_transform, cursor_pos).ok()?;

    let hit = rapier_context.cast_ray(ray.origin, ray.direction, 100.0, true, &default())?;
    remote_actors.contains(hit.entity).then_some(hit.entity)
}

pub fn clear_despawned_target(
    mut selected: ResMut<SelectedTarget>,
    mut is_attacking: ResMut<IsAttacking>,
    remote_actors: Query<Entity, With<RemoteInterpolation>>,
) {
    if let Some(entity) = selected.0
        && remote_actors.get(entity).is_err()
    {
        selected.0 = None;
        is_attacking.0 = false;
    }
}
