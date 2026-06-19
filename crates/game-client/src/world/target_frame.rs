use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::core::NameComponent;
use crate::web::SocialSender;
use crate::theme::widgets::{self, ContextMenu, UnitFrame, UnitFrameConfig};
use super::selection::SelectedTarget;
use game_core::components::{LevelComponent, Vitals};

#[derive(Component)]
pub(crate) struct TargetUnitFrame;

pub(crate) fn manage_target_unit_frame(
    selected: Res<SelectedTarget>,
    targets: Query<(&NameComponent, &LevelComponent, &Vitals)>,
    existing_frame: Query<Entity, With<TargetUnitFrame>>,
    mut commands: Commands,
) {
    match selected.0 {
        Some(target_entity) => {
            let Ok((name, level, vitals)) = targets.get(target_entity) else {
                for entity in existing_frame.iter() {
                    commands.entity(entity).despawn();
                }
                return;
            };

            if existing_frame.is_empty() {
                let health_pct = if vitals.max_hp > 0 {
                    (vitals.hp as f32 / vitals.max_hp as f32) * 100.0
                } else {
                    0.0
                };
                let config = UnitFrameConfig::target(target_entity);
                let entity = widgets::spawn_unit_frame(
                    &mut commands,
                    &config,
                    &name.0,
                    level.0,
                    health_pct,
                );
                commands.entity(entity).insert(TargetUnitFrame);
            }
        }
        None => {
            for entity in existing_frame.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

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

    widgets::despawn_context_menu(&mut commands, &existing_menu);

    if selected.0.is_none() {
        return;
    }

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

    let menu_entity = widgets::spawn_context_menu(&mut commands, cursor_pos);
    let button =
        widgets::spawn_context_menu_button(&mut commands, menu_entity, "Invite to Party");
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

    widgets::despawn_context_menu(&mut commands, &context_menu_q);
}
