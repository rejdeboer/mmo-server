use bevy::prelude::*;

use crate::theme::palette;
use super::action_bar::BAR_BOTTOM;

/// Tracks an in-progress cast for the local player.
#[derive(Resource)]
pub struct ActiveCast {
    pub spell_id: u32,
    pub spell_name: String,
    pub timer: Timer,
}

#[derive(Component)]
pub struct CastBar;

#[derive(Component)]
pub struct CastBarFill;

#[derive(Component)]
pub(crate) struct CastBarText;

const CAST_BAR_WIDTH: f32 = 250.0;
const CAST_BAR_HEIGHT: f32 = 24.0;
const CAST_BAR_BOTTOM: f32 = BAR_BOTTOM + 50.0 + 16.0 + 10.0;

pub fn manage_cast_bar(
    active_cast: Option<Res<ActiveCast>>,
    existing: Query<Entity, With<CastBar>>,
    mut commands: Commands,
) {
    match active_cast {
        Some(cast) => {
            if !cast.is_added() {
                return;
            }

            for entity in existing.iter() {
                commands.entity(entity).despawn();
            }

            if cast.timer.duration().as_secs_f32() <= 0.0 {
                return;
            }

            commands
                .spawn((
                    CastBar,
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(CAST_BAR_BOTTOM),
                        left: Val::Percent(50.0),
                        margin: UiRect {
                            left: Val::Px(-CAST_BAR_WIDTH / 2.0),
                            ..default()
                        },
                        width: Val::Px(CAST_BAR_WIDTH),
                        height: Val::Px(CAST_BAR_HEIGHT),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(palette::FRAME_BG),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        CastBarFill,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(palette::CAST_BAR_FILL),
                    ));

                    bar.spawn((
                        CastBarText,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        Text::new(cast.spell_name.clone()),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                    ));
                });
        }
        None => {
            for entity in existing.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn update_cast_bar(
    time: Res<Time>,
    mut active_cast: Option<ResMut<ActiveCast>>,
    mut fill: Query<&mut Node, With<CastBarFill>>,
    mut commands: Commands,
) {
    let Some(ref mut cast) = active_cast else {
        return;
    };

    cast.timer.tick(time.delta());

    let progress = cast.timer.fraction() * 100.0;
    for mut node in fill.iter_mut() {
        node.width = Val::Percent(progress);
    }

    if cast.timer.is_finished() {
        commands.remove_resource::<ActiveCast>();
    }
}
