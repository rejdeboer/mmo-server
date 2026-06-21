//! Reusable UI widgets: unit frames and context menus.

use bevy::picking::events::{Out, Over, Pointer};
use bevy::prelude::*;

use crate::core::NameComponent;
use game_core::components::{LevelComponent, Vitals};

use super::palette;

// ---------------------------------------------------------------------------
// Unit Frame
// ---------------------------------------------------------------------------

/// Tracks which entity this unit frame displays.
#[derive(Component)]
pub struct UnitFrame {
    pub tracked_entity: Entity,
}

/// Marker for the name text node within a unit frame.
#[derive(Component)]
pub(crate) struct UnitFrameName;

/// Marker for the level text node within a unit frame.
#[derive(Component)]
pub(crate) struct UnitFrameLevel;

/// Marker for the health bar fill node within a unit frame.
#[derive(Component)]
pub(crate) struct UnitFrameHealthBar;

/// Positioning and styling options for spawning a unit frame.
pub struct UnitFrameConfig {
    pub tracked_entity: Entity,
    pub position_type: PositionType,
    pub top: Val,
    pub left: Val,
    pub bottom: Val,
    pub right: Val,
    pub margin: UiRect,
    pub width: f32,
}

impl UnitFrameConfig {
    /// Config for a target unit frame (top-center of screen).
    pub fn target(tracked_entity: Entity) -> Self {
        Self {
            tracked_entity,
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Percent(50.0),
            bottom: Val::Auto,
            right: Val::Auto,
            margin: UiRect {
                left: Val::Px(-120.0),
                ..default()
            },
            width: 240.0,
        }
    }

    /// Config for a player unit frame (top-left of screen).
    pub fn player(tracked_entity: Entity) -> Self {
        Self {
            tracked_entity,
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            bottom: Val::Auto,
            right: Val::Auto,
            margin: UiRect::ZERO,
            width: 240.0,
        }
    }
}

/// Spawns a unit frame and returns its root entity.
pub fn spawn_unit_frame(
    commands: &mut Commands,
    config: &UnitFrameConfig,
    name: &str,
    level: i32,
    health_pct: f32,
) -> Entity {
    commands
        .spawn((
            UnitFrame {
                tracked_entity: config.tracked_entity,
            },
            Interaction::None,
            Node {
                position_type: config.position_type,
                top: config.top,
                left: config.left,
                bottom: config.bottom,
                right: config.right,
                margin: config.margin,
                width: Val::Px(config.width),
                height: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(palette::FRAME_BG),
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
                        TextColor(palette::LEVEL_COLOR),
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
                    BackgroundColor(palette::HP_BG),
                ))
                .with_children(|bar_bg| {
                    bar_bg.spawn((
                        UnitFrameHealthBar,
                        Node {
                            width: Val::Percent(health_pct),
                            height: Val::Percent(100.0),
                            border_radius: BorderRadius::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(palette::HP_GREEN),
                    ));
                });
        })
        .id()
}

/// Updates all unit frames to reflect their tracked entity's current stats.
/// Despawns frames whose tracked entity no longer exists.
pub fn update_unit_frames(
    frames: Query<(Entity, &UnitFrame, &Children)>,
    targets: Query<(&NameComponent, &LevelComponent, &Vitals)>,
    children_query: Query<&Children>,
    mut name_texts: Query<&mut Text, (With<UnitFrameName>, Without<UnitFrameLevel>)>,
    mut level_texts: Query<&mut Text, (With<UnitFrameLevel>, Without<UnitFrameName>)>,
    mut health_bars: Query<&mut Node, With<UnitFrameHealthBar>>,
    mut commands: Commands,
) {
    for (frame_entity, frame, frame_children) in frames.iter() {
        let Ok((name, level, vitals)) = targets.get(frame.tracked_entity) else {
            commands.entity(frame_entity).despawn();
            continue;
        };

        let health_pct = if vitals.max_hp > 0 {
            (vitals.hp as f32 / vitals.max_hp as f32) * 100.0
        } else {
            0.0
        };

        for child in frame_children.iter() {
            if let Ok(mut text) = name_texts.get_mut(child) {
                **text = name.0.clone();
            }
            if let Ok(mut text) = level_texts.get_mut(child) {
                **text = format!("Lv. {}", level.0);
            }

            if let Ok(grandchildren) = children_query.get(child) {
                for grandchild in grandchildren.iter() {
                    if let Ok(mut text) = name_texts.get_mut(grandchild) {
                        **text = name.0.clone();
                    }
                    if let Ok(mut text) = level_texts.get_mut(grandchild) {
                        **text = format!("Lv. {}", level.0);
                    }
                    if let Ok(mut node) = health_bars.get_mut(grandchild) {
                        node.width = Val::Percent(health_pct);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Context Menu
// ---------------------------------------------------------------------------

/// Marker for the context menu root node.
#[derive(Component)]
pub struct ContextMenu;

/// Marker for context menu buttons.
#[derive(Component)]
pub struct ContextMenuButton;

/// Despawns any existing context menu.
pub fn despawn_context_menu(commands: &mut Commands, existing: &Query<Entity, With<ContextMenu>>) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
}

/// Spawns a context menu at the given screen position.
pub fn spawn_context_menu(commands: &mut Commands, position: Vec2) -> Entity {
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
            BackgroundColor(palette::CONTEXT_MENU_BG),
            ZIndex(100),
            Pickable::IGNORE,
        ))
        .id()
}

/// Spawns a button inside a context menu. Returns the button entity so the
/// caller can attach their own click observer.
pub fn spawn_context_menu_button(
    commands: &mut Commands,
    menu_entity: Entity,
    label: &str,
) -> Entity {
    commands
        .spawn((
            ContextMenuButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(menu_entity),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        })
        .observe(on_button_hover_start)
        .observe(on_button_hover_end)
        .id()
}

fn on_button_hover_start(event: On<Pointer<Over>>, mut bg_query: Query<&mut BackgroundColor>) {
    if let Ok(mut bg) = bg_query.get_mut(event.event_target()) {
        *bg = BackgroundColor(palette::CONTEXT_MENU_HOVER);
    }
}

fn on_button_hover_end(event: On<Pointer<Out>>, mut bg_query: Query<&mut BackgroundColor>) {
    if let Ok(mut bg) = bg_query.get_mut(event.event_target()) {
        *bg = BackgroundColor(Color::NONE);
    }
}

// ---------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------

/// Marker for a dialog root node.
#[derive(Component)]
pub struct Dialog;

/// Marker for dialog buttons.
#[derive(Component)]
pub struct DialogButton;

/// Stores the base background color so hover observers can restore it.
#[derive(Component)]
struct DialogButtonBaseColor(Color);

/// Despawns all entities matching a given marker component.
pub fn despawn_dialog<M: Component>(
    commands: &mut Commands,
    existing: &Query<Entity, With<M>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
}

/// Spawns a centered dialog with a text message. Returns the root entity
/// so the caller can add additional children (e.g. button rows).
pub fn spawn_dialog(commands: &mut Commands, message: &str) -> Entity {
    commands
        .spawn((
            Dialog,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(30.0),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-160.0),
                    ..default()
                },
                width: Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(12.0),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_BG),
            ZIndex(101),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(message),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
            ));
        })
        .id()
}

/// Spawns a button inside a dialog. Returns the button entity so the caller
/// can attach a click observer.
pub fn spawn_dialog_button(
    commands: &mut Commands,
    parent_entity: Entity,
    label: &str,
    bg_color: Color,
) -> Entity {
    commands
        .spawn((
            DialogButton,
            DialogButtonBaseColor(bg_color),
            Button,
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            ChildOf(parent_entity),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        })
        .observe(on_dialog_button_hover_start)
        .observe(on_dialog_button_hover_end)
        .id()
}

fn on_dialog_button_hover_start(
    event: On<Pointer<Over>>,
    mut bg_query: Query<&mut BackgroundColor>,
) {
    if let Ok(mut bg) = bg_query.get_mut(event.event_target()) {
        *bg = BackgroundColor(palette::DIALOG_BUTTON_HOVER);
    }
}

fn on_dialog_button_hover_end(
    event: On<Pointer<Out>>,
    mut query: Query<(&mut BackgroundColor, &DialogButtonBaseColor)>,
) {
    if let Ok((mut bg, base)) = query.get_mut(event.event_target()) {
        *bg = BackgroundColor(base.0);
    }
}
