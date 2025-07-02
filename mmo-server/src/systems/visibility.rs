use bevy::{platform::collections::HashSet, prelude::*};

use crate::{
    application::SpatialGrid,
    components::{ClientIdComponent, GridCell, InterestedClients, VisibleEntities},
    events::OutgoingMessage,
};

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
    mut writer: EventWriter<OutgoingMessage>,
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

                        if let Ok(other_transform) = q_transform.get(other_entity) {
                            if player_pos.distance(other_transform.translation) < VIEW_RADIUS {
                                new_visible_set.insert(other_entity);
                            }
                        }
                    }
                }
            }
        }

        for &entity_to_spawn in new_visible_set.difference(&visible.entities) {
            if let Ok(mut interested) = q_interest.get_mut(entity_to_spawn) {
                interested.clients.insert(client_id.0);
            }
            if let Ok(transform) = q_transform.get(entity_to_spawn) {
                writer.write(OutgoingMessage {
                    client_id: client_id.0,
                    data: crate::events::OutgoingMessageData::Spawn(
                        entity_to_spawn,
                        transform.clone(),
                    ),
                });
            }
        }

        for &entity_to_despawn in visible.entities.difference(&new_visible_set) {
            if let Ok(mut interested) = q_interest.get_mut(entity_to_despawn) {
                interested.clients.remove(&client_id.0);
            }
            writer.write(OutgoingMessage {
                client_id: client_id.0,
                data: crate::events::OutgoingMessageData::Despawn(entity_to_despawn),
            });
        }

        visible.entities = new_visible_set;
    }
}
