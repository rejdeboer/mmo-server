use super::components::{AiBrain, AiMovement, AiState, LeashAnchor, ThreatTable};
use crate::{
    assets::{SpellLibrary, SpellLibraryHandle},
    components::Abilities,
};
use bevy::prelude::*;

/// Evaluates and applies AI state transitions based on threat, distance, and leash.
pub fn ai_state_transitions(
    mut q_mobs: Query<(
        &Transform,
        &mut AiBrain,
        &mut AiMovement,
        &ThreatTable,
        &LeashAnchor,
        &Abilities,
    )>,
    q_targets: Query<&Transform, Without<AiBrain>>,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        return;
    };

    for (mob_transform, mut brain, mut movement, threat_table, leash, abilities) in
        q_mobs.iter_mut()
    {
        let mob_pos = mob_transform.translation;

        match &brain.state {
            AiState::Idle => {
                // Transition to Chase if we have a threat target
                if let Some(top_threat) = threat_table.highest_threat()
                    && let Ok(target_transform) = q_targets.get(top_threat.entity)
                {
                    brain.state = AiState::Chase {
                        target: top_threat.entity,
                    };
                    movement.target_position = Some(target_transform.translation);
                }
            }
            AiState::Chase { target } => {
                let target = *target;

                // Target lost (dead/disconnected)
                if q_targets.get(target).is_err() || threat_table.entries.is_empty() {
                    brain.state = AiState::Returning;
                    movement.target_position = Some(leash.position);
                    continue;
                }

                // Leash check
                if mob_pos.distance_squared(leash.position)
                    > leash.max_range * leash.max_range
                {
                    brain.state = AiState::Evading;
                    movement.target_position = Some(leash.position);
                    continue;
                }

                // Check if in range of any ability -> transition to Combat
                let target_pos = q_targets.get(target).unwrap().translation;
                let dist = mob_pos.distance(target_pos);

                let in_ability_range = abilities.known.iter().any(|ability| {
                    library
                        .spells
                        .get(&ability.spell_id)
                        .is_some_and(|spell| dist <= spell.range)
                });

                if in_ability_range {
                    brain.state = AiState::Combat { target };
                    movement.target_position = None;
                } else {
                    movement.target_position = Some(target_pos);
                }

                // Switch target if a higher threat exists (10% hysteresis)
                if let Some(top_threat) = threat_table.highest_threat()
                    && top_threat.entity != target
                {
                    let current_threat = threat_table
                        .entries
                        .iter()
                        .find(|e| e.entity == target)
                        .map(|e| e.threat)
                        .unwrap_or(0.0);
                    if top_threat.threat > current_threat * 1.1 {
                        brain.state = AiState::Chase {
                            target: top_threat.entity,
                        };
                    }
                }
            }
            AiState::Combat { target } => {
                let target = *target;

                // Target lost
                if q_targets.get(target).is_err() || threat_table.entries.is_empty() {
                    brain.state = AiState::Returning;
                    movement.target_position = Some(leash.position);
                    continue;
                }

                // Leash check
                if mob_pos.distance_squared(leash.position)
                    > leash.max_range * leash.max_range
                {
                    brain.state = AiState::Evading;
                    movement.target_position = Some(leash.position);
                    continue;
                }

                let target_pos = q_targets.get(target).unwrap().translation;
                let dist = mob_pos.distance(target_pos);

                // Check if still in range of any ability
                let in_ability_range = abilities.known.iter().any(|ability| {
                    library
                        .spells
                        .get(&ability.spell_id)
                        .is_some_and(|spell| dist <= spell.range)
                });

                if !in_ability_range {
                    // Out of range, chase
                    brain.state = AiState::Chase { target };
                    movement.target_position = Some(target_pos);
                } else {
                    movement.target_position = None;
                }

                // Switch target if a higher threat exists (10% hysteresis)
                if let Some(top_threat) = threat_table.highest_threat()
                    && top_threat.entity != target
                {
                    let current_threat = threat_table
                        .entries
                        .iter()
                        .find(|e| e.entity == target)
                        .map(|e| e.threat)
                        .unwrap_or(0.0);
                    if top_threat.threat > current_threat * 1.1 {
                        brain.state = AiState::Chase {
                            target: top_threat.entity,
                        };
                    }
                }
            }
            AiState::Returning => {
                let dist_sq = mob_pos.distance_squared(leash.position);
                if dist_sq < 2.0 * 2.0 {
                    brain.state = AiState::Idle;
                    movement.target_position = None;
                } else {
                    movement.target_position = Some(leash.position);
                }
            }
            AiState::Evading => {
                let dist_sq = mob_pos.distance_squared(leash.position);
                if dist_sq < 2.0 * 2.0 {
                    brain.state = AiState::Idle;
                    movement.target_position = None;
                } else {
                    movement.target_position = Some(leash.position);
                }
            }
        }
    }
}
