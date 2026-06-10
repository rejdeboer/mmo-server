use avian3d::prelude::SpatialQuery;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::application::NameComponent;
use crate::movement::RemoteInterpolation;
use crate::social::SocialSender;
use crate::ui::{
    ContextMenu, UnitFrame, UnitFrameConfig, context_menu, unit_frame,
};
use game_core::components::{LevelComponent, Vitals};

/// The currently selected target entity.
#[derive(Resource, Default)]
pub struct SelectedTarget(pub Option<Entity>);

/// Whether the cursor is currently over a game UI element.
/// Camera input should be suppressed when this is true.
#[derive(Resource, Default)]
pub struct CursorOverUi(pub bool);

/// Tracks whether the left mouse button was a click (not a drag).
#[derive(Resource, Default)]
pub struct ClickTracker {
    pressed: bool,
    start_position: Vec2,
    dragged: bool,
}

/// Marker to identify the target-specific unit frame among all unit frames.
#[derive(Component)]
pub(crate) struct TargetUnitFrame;

const DRAG_THRESHOLD: f32 = 5.0;

/// Updates the CursorOverUi resource based on UI interaction state.
pub(crate) fn update_cursor_over_ui(
    interactables: Query<&Interaction, Or<(With<UnitFrame>, With<Button>)>>,
    mut cursor_over_ui: ResMut<CursorOverUi>,
) {
    cursor_over_ui.0 = interactables.iter().any(|i| *i != Interaction::None);
}

/// Detects left-click (non-drag) on remote actors to select them as target.
pub(crate) fn handle_target_selection(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    rapier_context: SpatialQuery,
    remote_actors: Query<Entity, With<RemoteInterpolation>>,
    mut selected: ResMut<SelectedTarget>,
    mut click_tracker: ResMut<ClickTracker>,
    cursor_over_ui: Res<CursorOverUi>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    // Track click vs drag
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

    if mouse_button.just_released(MouseButton::Left) {
        let was_click = click_tracker.pressed && !click_tracker.dragged;
        click_tracker.pressed = false;

        if !was_click || cursor_over_ui.0 {
            return;
        }

        // Raycast from cursor position
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        let Ok((camera, camera_transform)) = cameras.single() else {
            return;
        };
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
            return;
        };

        if let Some(hit) =
            rapier_context.cast_ray(ray.origin, ray.direction, 100.0, true, &default())
        {
            if remote_actors.contains(hit.entity) {
                selected.0 = Some(hit.entity);
            } else {
                selected.0 = None;
            }
        } else {
            selected.0 = None;
        }
    }
}

/// Manages the target unit frame lifecycle based on the currently selected target.
pub(crate) fn manage_target_unit_frame(
    selected: Res<SelectedTarget>,
    targets: Query<(&NameComponent, &LevelComponent, &Vitals)>,
    existing_frame: Query<Entity, With<TargetUnitFrame>>,
    mut commands: Commands,
) {
    match selected.0 {
        Some(target_entity) => {
            let Ok((name, level, vitals)) = targets.get(target_entity) else {
                // Target entity no longer valid, despawn frame
                for entity in existing_frame.iter() {
                    commands.entity(entity).despawn();
                }
                return;
            };

            // Only spawn if no frame exists yet; updates are handled by
            // the generic `update_unit_frames` system.
            if existing_frame.is_empty() {
                let health_pct = if vitals.max_hp > 0 {
                    (vitals.hp as f32 / vitals.max_hp as f32) * 100.0
                } else {
                    0.0
                };
                let config = UnitFrameConfig::target(target_entity);
                let entity =
                    unit_frame::spawn_unit_frame(&mut commands, &config, &name.0, level.0, health_pct);
                commands.entity(entity).insert(TargetUnitFrame);
            }
        }
        None => {
            // No target: despawn the target unit frame
            for entity in existing_frame.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// When the selected target changes, update the tracked entity on the existing
/// frame so the generic update system picks up the new data.
pub(crate) fn sync_target_unit_frame(
    selected: Res<SelectedTarget>,
    mut frames: Query<&mut UnitFrame, With<TargetUnitFrame>>,
) {
    if !selected.is_changed() {
        return;
    }

    let Some(target_entity) = selected.0 else {
        return;
    };

    if let Ok(mut frame) = frames.single_mut() {
        frame.tracked_entity = target_entity;
    }
}

/// Shows a context menu on right-click of the target unit frame.
pub(crate) fn handle_target_context_menu(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    selected: Res<SelectedTarget>,
    unit_frame_interaction: Query<&Interaction, With<TargetUnitFrame>>,
    existing_menu: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    if !mouse_button.just_pressed(MouseButton::Right) {
        return;
    }

    // Close any existing context menu first
    context_menu::despawn_context_menu(&mut commands, &existing_menu);

    if selected.0.is_none() {
        return;
    }

    // Check if cursor is hovering over the unit frame
    let is_hovering = unit_frame_interaction
        .iter()
        .any(|i| *i != Interaction::None);
    if !is_hovering {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let menu_entity = context_menu::spawn_context_menu(&mut commands, cursor_pos);
    let button =
        context_menu::spawn_context_menu_button(&mut commands, menu_entity, "Invite to Party");
    commands.entity(button).observe(on_invite_click);
}

fn on_invite_click(
    _event: On<Pointer<Click>>,
    selected: Res<SelectedTarget>,
    targets: Query<&NameComponent>,
    social_sender: Res<SocialSender>,
    context_menu_q: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    if let Some(target_entity) = selected.0
        && let Ok(name) = targets.get(target_entity)
    {
        if let Some(ref sender) = social_sender.0 {
            let action = web_client::SocialAction::PartyInviteByName {
                target_name: name.0.clone(),
            };
            if let Err(e) = sender.try_send(action) {
                tracing::error!("failed to send party invite: {}", e);
            } else {
                tracing::info!("sent party invite to {}", name.0);
            }
        }
    }

    // Close the context menu
    context_menu::despawn_context_menu(&mut commands, &context_menu_q);
}

/// Clears selected target if the entity is despawned.
pub(crate) fn clear_despawned_target(
    mut selected: ResMut<SelectedTarget>,
    remote_actors: Query<Entity, With<RemoteInterpolation>>,
) {
    if let Some(entity) = selected.0 && remote_actors.get(entity).is_err() {
        selected.0 = None;
    }
}
