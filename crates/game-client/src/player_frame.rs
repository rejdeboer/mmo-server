use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::application::{NameComponent, PlayerComponent};
use crate::social::SocialSender;
use crate::ui::{ContextMenu, UnitFrameConfig, context_menu, unit_frame};
use game_core::components::{LevelComponent, Vitals};

/// Marker to identify the player-specific unit frame.
#[derive(Component)]
pub(crate) struct PlayerUnitFrame;

/// Spawns the player unit frame. Called once when entering the game.
pub(crate) fn spawn_player_unit_frame(
    player: Query<(Entity, &NameComponent, &LevelComponent, &Vitals), With<PlayerComponent>>,
    existing_frame: Query<Entity, With<PlayerUnitFrame>>,
    mut commands: Commands,
) {
    if !existing_frame.is_empty() {
        return;
    }

    let Ok((player_entity, name, level, vitals)) = player.single() else {
        return;
    };

    let health_pct = if vitals.max_hp > 0 {
        (vitals.hp as f32 / vitals.max_hp as f32) * 100.0
    } else {
        0.0
    };

    let config = UnitFrameConfig::player(player_entity);
    let frame_entity =
        unit_frame::spawn_unit_frame(&mut commands, &config, &name.0, level.0, health_pct);
    commands.entity(frame_entity).insert(PlayerUnitFrame);
}

/// Shows a context menu on right-click of the player unit frame.
pub(crate) fn handle_player_context_menu(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    unit_frame_interaction: Query<&Interaction, With<PlayerUnitFrame>>,
    existing_menu: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    if !mouse_button.just_pressed(MouseButton::Right) {
        return;
    }

    // Close any existing context menu first
    context_menu::despawn_context_menu(&mut commands, &existing_menu);

    // Check if cursor is hovering over the player unit frame
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

    let leave_party_btn =
        context_menu::spawn_context_menu_button(&mut commands, menu_entity, "Leave Party");
    commands.entity(leave_party_btn).observe(on_leave_party_click);

    let logout_btn = context_menu::spawn_context_menu_button(&mut commands, menu_entity, "Logout");
    commands.entity(logout_btn).observe(on_logout_click);
}

fn on_leave_party_click(
    _event: On<Pointer<Click>>,
    social_sender: Res<SocialSender>,
    context_menu_q: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    if let Some(ref sender) = social_sender.0 {
        if let Err(e) = sender.try_send(web_client::SocialAction::PartyLeave) {
            tracing::error!("failed to send party leave: {}", e);
        } else {
            tracing::info!("sent party leave request");
        }
    }

    context_menu::despawn_context_menu(&mut commands, &context_menu_q);
}

fn on_logout_click(
    _event: On<Pointer<Click>>,
    context_menu_q: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    context_menu::despawn_context_menu(&mut commands, &context_menu_q);
    // TODO: Transition to character select / disconnect properly
    std::process::exit(0);
}
