use crate::{
    application::SpatialGrid,
    components::{ClientIdComponent, GridCell, InterestedClients, VisibleEntities},
    messages::VisibilityChangedMessage,
};
use bevy::{platform::collections::HashSet, prelude::*};

const VIEW_RADIUS: f32 = 256.0;

pub fn update_player_visibility(
    mut q_players: Query<(
        Entity,
        &Transform,
        &GridCell,
        &ClientIdComponent,
        &mut VisibleEntities,
    )>,
    mut q_interest: Query<&mut InterestedClients>,
    q_transform: Query<&Transform>,
    grid: Res<SpatialGrid>,
    mut writer: MessageWriter<VisibilityChangedMessage>,
) {
    // TODO: Parallelism?
    for (player_entity, player_transform, player_cell, client_id, mut visible) in
        q_players.iter_mut()
    {
        let mut new_visible_set = HashSet::new();
        let player_pos = player_transform.translation;

        for y in -1..=1 {
            for x in -1..=1 {
                let cell_coords = player_cell.0 + IVec2::new(x, y);

                if let Some(cell_entities) = grid.cells.get(&cell_coords) {
                    for &other_entity in cell_entities {
                        if player_entity == other_entity {
                            continue;
                        }

                        if let Ok(other_transform) = q_transform.get(other_entity)
                            && player_pos.distance(other_transform.translation) < VIEW_RADIUS
                        {
                            new_visible_set.insert(other_entity);
                        }
                    }
                }
            }
        }

        let added_entities = new_visible_set
            .difference(&visible.entities)
            .copied()
            .collect::<Vec<Entity>>();
        for &entity_to_spawn in &added_entities {
            if let Ok(mut interested) = q_interest.get_mut(entity_to_spawn) {
                interested.clients.insert(client_id.0);
            }
        }

        let removed_entities = visible
            .entities
            .difference(&new_visible_set)
            .copied()
            .collect::<Vec<Entity>>();
        for &entity_to_despawn in &removed_entities {
            if let Ok(mut interested) = q_interest.get_mut(entity_to_despawn) {
                interested.clients.remove(&client_id.0);
            }
        }

        if !added_entities.is_empty() || !removed_entities.is_empty() {
            writer.write(VisibilityChangedMessage {
                client_id: client_id.0,
                added: added_entities,
                removed: removed_entities,
            });
        }

        visible.entities = new_visible_set;
    }
}
