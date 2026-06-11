use super::components::{AiBrain, AiState, ThreatTable};
use bevy::prelude::*;
use game_core::components::Vitals;

/// Resets mob HP and threat when they arrive back at leash anchor after evading.
pub fn reset_evading_mobs(
    mut q_mobs: Query<(&mut Vitals, &mut ThreatTable, &AiBrain)>,
) {
    for (mut vitals, mut threat_table, brain) in q_mobs.iter_mut() {
        if brain.state == AiState::Idle {
            // Just transitioned back to Idle from Evading/Returning
            // If HP is not full, heal to full
            if vitals.hp < vitals.max_hp {
                vitals.hp = vitals.max_hp;
                threat_table.clear();
            }
        }
    }
}
