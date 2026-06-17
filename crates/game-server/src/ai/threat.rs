use super::components::{AggroRadius, AiBehavior, AiBrain, AiState, ThreatTable};
use crate::{
    combat::ApplySpellEffectMessage,
    core::{ClientIdComponent, GridCell},
    world::SpatialGrid,
};
use bevy::prelude::*;
use game_core::components::Vitals;

/// Detects players within aggro radius for idle aggressive mobs and adds initial threat.
pub fn detect_players(
    mut q_mobs: Query<(
        &Transform,
        &GridCell,
        &AggroRadius,
        &mut ThreatTable,
        &AiBrain,
    )>,
    q_players: Query<(Entity, &Transform), With<ClientIdComponent>>,
    grid: Res<SpatialGrid>,
) {
    for (mob_transform, mob_cell, aggro_radius, mut threat_table, brain) in q_mobs.iter_mut() {
        if brain.state != AiState::Idle || brain.behavior != AiBehavior::Aggressive {
            continue;
        }

        let mob_pos = mob_transform.translation;
        let aggro_dist_sq = aggro_radius.0 * aggro_radius.0;

        // Check neighboring grid cells for players
        for y in -1..=1 {
            for x in -1..=1 {
                let cell_coords = mob_cell.0 + IVec2::new(x, y);
                let Some(cell_entities) = grid.cells.get(&cell_coords) else {
                    continue;
                };

                for &entity in cell_entities {
                    let Ok((player_entity, player_transform)) = q_players.get(entity) else {
                        continue;
                    };

                    let dist_sq = mob_pos.distance_squared(player_transform.translation);
                    if dist_sq <= aggro_dist_sq {
                        threat_table.add_threat(player_entity, 1.0);
                    }
                }
            }
        }
    }
}

/// Adds threat when a mob takes damage from a player.
pub fn update_threat_on_damage(
    mut reader: MessageReader<ApplySpellEffectMessage>,
    mut q_mobs: Query<&mut ThreatTable>,
    q_vitals: Query<&Vitals>,
) {
    for msg in reader.read() {
        // Only track threat if the target is a mob with a threat table
        let Ok(mut threat_table) = q_mobs.get_mut(msg.target_entity) else {
            continue;
        };

        // Only add threat if target is alive
        let Ok(vitals) = q_vitals.get(msg.target_entity) else {
            continue;
        };
        if vitals.hp <= 0 {
            continue;
        }

        // Add threat equal to damage dealt (spell_id lookup not needed here,
        // since damage is already applied; we use a flat amount per hit for now)
        threat_table.add_threat(msg.caster_entity, 10.0);
    }
}

/// Removes dead or despawned entities from all threat tables.
pub fn cleanup_threat_tables(
    mut q_mobs: Query<&mut ThreatTable>,
    q_alive: Query<Entity, With<Vitals>>,
) {
    for mut threat_table in q_mobs.iter_mut() {
        threat_table
            .entries
            .retain(|entry| q_alive.get(entry.entity).is_ok());
    }
}
