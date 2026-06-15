//! # Unit Frame UI
//!
//! A reusable unit frame widget that displays an entity's name, level, and
//! health bar. Used for target frames, player frames, party frames, etc.
//!
//! ## Usage
//!
//! Spawn a unit frame by calling [`spawn_unit_frame`] with a
//! [`UnitFrameConfig`] describing the tracked entity and positioning. The
//! [`update_unit_frames`] system automatically keeps all frames in sync with
//! their tracked entity's current stats. If the tracked entity is despawned,
//! the frame is automatically removed.

use bevy::prelude::*;

use crate::application::NameComponent;
use game_core::components::{LevelComponent, Vitals};

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

        // Walk the frame's descendants to find the marker components
        for child in frame_children.iter() {
            if let Ok(mut text) = name_texts.get_mut(child) {
                **text = name.0.clone();
            }
            if let Ok(mut text) = level_texts.get_mut(child) {
                **text = format!("Lv. {}", level.0);
            }

            // The health bar is nested deeper (frame -> bar_bg -> bar_fill)
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
