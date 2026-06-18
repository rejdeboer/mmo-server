use bevy::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use game_core::{
    components::NetworkId,
    spells::{SpellLibrary, SpellLibraryHandle},
};
use protocol::client::PlayerAction;
use std::collections::HashMap;

use crate::application::KnownAbilities;
use crate::target::SelectedTarget;

use super::ChatInputState;

/// Client-side cooldown tracking per spell_id.
#[derive(Resource, Default)]
pub struct AbilityCooldowns(pub HashMap<u32, Timer>);

/// Marker on the root action bar UI node.
#[derive(Component)]
pub struct ActionBar;

/// Attached to each ability slot button. Holds the spell_id and slot index.
#[derive(Component)]
pub struct AbilitySlot {
    pub spell_id: u32,
    pub slot_index: usize,
}

/// Marker on the keybind label inside a slot.
#[derive(Component)]
struct AbilitySlotKeybind;

const SLOT_SIZE: f32 = 50.0;
const SLOT_GAP: f32 = 4.0;
const BAR_PADDING: f32 = 8.0;
pub const BAR_BOTTOM: f32 = 20.0;

const KEYBINDS: &[KeyCode] = &[
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

const SLOT_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.9);
const SLOT_HOVER_COLOR: Color = Color::srgba(0.3, 0.3, 0.3, 0.9);
const SLOT_COOLDOWN_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);

/// Spawns the action bar when KnownAbilities and SpellLibrary are available.
pub fn spawn_action_bar(
    existing: Query<Entity, With<ActionBar>>,
    known: Res<KnownAbilities>,
    library_handle: Res<SpellLibraryHandle>,
    libraries: Res<Assets<SpellLibrary>>,
    assets: Res<AssetServer>,
    mut commands: Commands,
) {
    if !existing.is_empty() {
        return;
    }

    let Some(library) = libraries.get(&library_handle.0) else {
        return;
    };

    let spells: Vec<(usize, u32, u32)> = known
        .0
        .iter()
        .enumerate()
        .filter_map(|(i, &spell_id)| {
            library
                .spells
                .get(&spell_id)
                .map(|def| (i, spell_id, def.visual_id))
        })
        .collect();

    if spells.is_empty() {
        return;
    }

    let bar_width =
        spells.len() as f32 * SLOT_SIZE + (spells.len() - 1) as f32 * SLOT_GAP + BAR_PADDING * 2.0;

    commands
        .spawn((
            ActionBar,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(BAR_BOTTOM),
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-bar_width / 2.0),
                    ..default()
                },
                width: Val::Px(bar_width),
                height: Val::Px(SLOT_SIZE + BAR_PADDING * 2.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(SLOT_GAP),
                padding: UiRect::all(Val::Px(BAR_PADDING)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
        ))
        .with_children(|bar| {
            for (slot_index, spell_id, visual_id) in &spells {
                let keybind_label = format!("{}", slot_index + 1);
                let icon_handle: Handle<Image> =
                    assets.load(format!("icons/{}.jpg", visual_id));

                bar.spawn((
                    AbilitySlot {
                        spell_id: *spell_id,
                        slot_index: *slot_index,
                    },
                    Button,
                    Interaction::None,
                    Node {
                        width: Val::Px(SLOT_SIZE),
                        height: Val::Px(SLOT_SIZE),
                        justify_content: JustifyContent::FlexEnd,
                        align_items: AlignItems::FlexEnd,
                        padding: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    ImageNode::new(icon_handle),
                ))
                .with_children(|slot| {
                    // Keybind number in the bottom-right corner
                    slot.spawn((
                        AbilitySlotKeybind,
                        Text::new(keybind_label.clone()),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });
            }
        });
}

/// Reads number keys 1-9 and sends CastSpell to the server.
pub fn handle_ability_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    chat_state: Res<ChatInputState>,
    selected: Res<SelectedTarget>,
    known: Res<KnownAbilities>,
    targets: Query<&NetworkId>,
    library_handle: Res<SpellLibraryHandle>,
    libraries: Res<Assets<SpellLibrary>>,
    mut cooldowns: ResMut<AbilityCooldowns>,
    mut client: ResMut<RenetClient>,
) {
    if chat_state.active {
        return;
    }

    let Some(library) = libraries.get(&library_handle.0) else {
        return;
    };

    for (slot_index, &key) in KEYBINDS.iter().enumerate() {
        if !keyboard.just_pressed(key) {
            continue;
        }

        let Some(&spell_id) = known.0.get(slot_index) else {
            continue;
        };

        if let Some(timer) = cooldowns.0.get(&spell_id) {
            if !timer.is_finished() {
                continue;
            }
        }

        cast_spell(
            spell_id,
            &selected,
            &targets,
            library,
            &mut cooldowns,
            &mut client,
        );
    }
}

/// Handles mouse clicks on ability slot buttons.
pub fn handle_ability_click(
    slots: Query<(&Interaction, &AbilitySlot), Changed<Interaction>>,
    selected: Res<SelectedTarget>,
    targets: Query<&NetworkId>,
    library_handle: Res<SpellLibraryHandle>,
    libraries: Res<Assets<SpellLibrary>>,
    mut cooldowns: ResMut<AbilityCooldowns>,
    mut client: ResMut<RenetClient>,
) {
    let Some(library) = libraries.get(&library_handle.0) else {
        return;
    };

    for (interaction, slot) in slots.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if let Some(timer) = cooldowns.0.get(&slot.spell_id) {
            if !timer.is_finished() {
                continue;
            }
        }

        cast_spell(
            slot.spell_id,
            &selected,
            &targets,
            library,
            &mut cooldowns,
            &mut client,
        );
    }
}

fn cast_spell(
    spell_id: u32,
    selected: &SelectedTarget,
    targets: &Query<&NetworkId>,
    library: &SpellLibrary,
    cooldowns: &mut AbilityCooldowns,
    client: &mut RenetClient,
) {
    let Some(target_entity) = selected.0 else {
        return;
    };

    let Ok(network_id) = targets.get(target_entity) else {
        tracing::warn!("selected target has no NetworkId");
        return;
    };

    let action = PlayerAction::CastSpell {
        spell_id,
        target_entity_id: network_id.0,
    };
    let encoded = bitcode::encode(&action);
    client.send_message(DefaultChannel::ReliableOrdered, encoded);

    if let Some(spell_def) = library.spells.get(&spell_id) {
        if spell_def.cooldown > 0.0 {
            cooldowns.0.insert(
                spell_id,
                Timer::from_seconds(spell_def.cooldown, TimerMode::Once),
            );
        }
    }
}

/// Ticks all client-side ability cooldown timers.
pub fn tick_cooldowns(time: Res<Time>, mut cooldowns: ResMut<AbilityCooldowns>) {
    for timer in cooldowns.0.values_mut() {
        timer.tick(time.delta());
    }
}

/// Updates slot background color based on cooldown and hover state.
pub fn update_slot_visuals(
    cooldowns: Res<AbilityCooldowns>,
    mut slots: Query<(&AbilitySlot, &Interaction, &mut BackgroundColor)>,
) {
    for (slot, interaction, mut bg) in slots.iter_mut() {
        let on_cooldown = cooldowns
            .0
            .get(&slot.spell_id)
            .is_some_and(|t| !t.is_finished());

        if on_cooldown {
            *bg = BackgroundColor(SLOT_COOLDOWN_COLOR);
        } else if *interaction == Interaction::Hovered {
            *bg = BackgroundColor(SLOT_HOVER_COLOR);
        } else {
            *bg = BackgroundColor(SLOT_COLOR);
        }
    }
}
