use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::application::NameComponent;
use crate::chat::SocialSender;
use crate::movement::RemoteInterpolation;
use game_core::components::{LevelComponent, Vitals};

/// The currently selected target entity.
#[derive(Resource, Default)]
pub struct SelectedTarget(pub Option<Entity>);

/// Marker for the unit frame root node.
#[derive(Component)]
pub(crate) struct UnitFrame;

/// Marker for the unit frame name text.
#[derive(Component)]
pub(crate) struct UnitFrameName;

/// Marker for the unit frame level text.
#[derive(Component)]
pub(crate) struct UnitFrameLevel;

/// Marker for the unit frame health bar fill.
#[derive(Component)]
pub(crate) struct UnitFrameHealthBar;

/// Marker for the context menu root node.
#[derive(Component)]
pub(crate) struct ContextMenu;

/// Marker for context menu buttons.
#[derive(Component)]
pub(crate) struct ContextMenuButton(ContextMenuAction);

#[derive(Clone, Copy)]
pub(crate) enum ContextMenuAction {
    InviteToParty,
}

/// Tracks whether the left mouse button was a click (not a drag).
#[derive(Resource, Default)]
pub struct ClickTracker {
    pressed: bool,
    start_position: Vec2,
    dragged: bool,
}

const DRAG_THRESHOLD: f32 = 5.0;

/// Detects left-click (non-drag) on remote actors to select them as target.
pub(crate) fn handle_target_selection(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    rapier_context: SpatialQuery,
    remote_actors: Query<Entity, With<RemoteInterpolation>>,
    mut selected: ResMut<SelectedTarget>,
    mut click_tracker: ResMut<ClickTracker>,
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

        if !was_click {
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

/// Spawns or despawns the unit frame based on selected target.
pub(crate) fn update_unit_frame(
    selected: Res<SelectedTarget>,
    targets: Query<(&NameComponent, &LevelComponent, &Vitals)>,
    mut commands: Commands,
    existing_frame: Query<Entity, With<UnitFrame>>,
    mut name_text: Query<&mut Text, (With<UnitFrameName>, Without<UnitFrameLevel>)>,
    mut level_text: Query<&mut Text, (With<UnitFrameLevel>, Without<UnitFrameName>)>,
    mut health_bar: Query<&mut Node, With<UnitFrameHealthBar>>,
) {
    let Some(target_entity) = selected.0 else {
        // No target: despawn unit frame if it exists
        for entity in existing_frame.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let Ok((name, level, vitals)) = targets.get(target_entity) else {
        // Target entity no longer valid
        for entity in existing_frame.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let health_pct = if vitals.max_hp > 0 {
        (vitals.hp as f32 / vitals.max_hp as f32) * 100.0
    } else {
        0.0
    };

    // Update existing frame or spawn new one
    if !existing_frame.is_empty() {
        // Update text
        if let Ok(mut text) = name_text.single_mut() {
            **text = name.0.clone();
        }
        if let Ok(mut text) = level_text.single_mut() {
            **text = format!("Lv. {}", level.0);
        }
        if let Ok(mut node) = health_bar.single_mut() {
            node.width = Val::Percent(health_pct);
        }
    } else {
        spawn_unit_frame(&mut commands, &name.0, level.0, health_pct);
    }
}

fn spawn_unit_frame(commands: &mut Commands, name: &str, level: i32, health_pct: f32) {
    commands
        .spawn((
            UnitFrame,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-120.0),
                    ..default()
                },
                width: Val::Px(240.0),
                height: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
        ))
        .with_children(|parent| {
            // Name + level row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        UnitFrameName,
                        Text::new(name.to_string()),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                    ));
                    row.spawn((
                        UnitFrameLevel,
                        Text::new(format!("Lv. {}", level)),
                        TextColor(Color::srgb(0.8, 0.8, 0.2)),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                    ));
                });

            // Health bar background
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(16.0),
                        border_radius: BorderRadius::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.0, 0.0)),
                ))
                .with_children(|bar_bg| {
                    // Health bar fill
                    bar_bg.spawn((
                        UnitFrameHealthBar,
                        Node {
                            width: Val::Percent(health_pct),
                            height: Val::Percent(100.0),
                            border_radius: BorderRadius::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.7, 0.1)),
                    ));
                });
        });
}

/// Shows a context menu on right-click of the unit frame.
pub(crate) fn handle_unit_frame_context_menu(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    selected: Res<SelectedTarget>,
    unit_frame: Query<(&Node, &GlobalTransform), With<UnitFrame>>,
    existing_menu: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
    computed_node: Query<&ComputedNode, With<UnitFrame>>,
) {
    // Close context menu on left-click anywhere
    if mouse_button.just_pressed(MouseButton::Left) {
        for entity in existing_menu.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    if !mouse_button.just_pressed(MouseButton::Right) {
        return;
    }

    // Close any existing context menu first
    for entity in existing_menu.iter() {
        commands.entity(entity).despawn();
    }

    if selected.0.is_none() {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Check if cursor is over the unit frame
    let Ok((_, frame_global)) = unit_frame.single() else {
        return;
    };
    let Ok(computed) = computed_node.single() else {
        return;
    };

    let frame_pos = frame_global.translation().truncate();
    let frame_size = computed.size();
    let frame_rect = Rect::from_center_size(frame_pos, frame_size);

    if !frame_rect.contains(cursor_pos) {
        return;
    }

    // Spawn context menu at cursor position
    spawn_context_menu(&mut commands, cursor_pos);
}

fn spawn_context_menu(commands: &mut Commands, position: Vec2) {
    commands
        .spawn((
            ContextMenu,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x),
                top: Val::Px(position.y),
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(150.0),
                padding: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.95)),
            // High z-index so it renders on top
            ZIndex(100),
        ))
        .with_children(|parent| {
            // Invite to Party button
            parent
                .spawn((
                    ContextMenuButton(ContextMenuAction::InviteToParty),
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        border_radius: BorderRadius::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Invite to Party"),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                    ));
                });
        });
}

/// Handles context menu button interactions.
pub(crate) fn handle_context_menu_interactions(
    interactions: Query<(&Interaction, &ContextMenuButton), (Changed<Interaction>, With<Button>)>,
    mut bg_query: Query<&mut BackgroundColor, With<ContextMenuButton>>,
    all_interactions: Query<(&Interaction, Entity), With<ContextMenuButton>>,
    selected: Res<SelectedTarget>,
    targets: Query<&NameComponent>,
    social_sender: Res<SocialSender>,
    context_menu: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    // Handle hover highlighting
    for (interaction, entity) in all_interactions.iter() {
        if let Ok(mut bg) = bg_query.get_mut(entity) {
            match interaction {
                Interaction::Hovered => {
                    *bg = BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.5));
                }
                Interaction::None => {
                    *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
                }
                _ => {}
            }
        }
    }

    // Handle clicks
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button.0 {
            ContextMenuAction::InviteToParty => {
                let Some(target_entity) = selected.0 else {
                    continue;
                };
                let Ok(name) = targets.get(target_entity) else {
                    continue;
                };
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
        }

        // Close the context menu
        for entity in context_menu.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Clears selected target if the entity is despawned.
pub(crate) fn clear_despawned_target(
    mut selected: ResMut<SelectedTarget>,
    remote_actors: Query<Entity, With<RemoteInterpolation>>,
) {
    if let Some(entity) = selected.0 {
        if remote_actors.get(entity).is_err() {
            selected.0 = None;
        }
    }
}
