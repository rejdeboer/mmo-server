use bevy::camera::visibility::VisibilityRange;
use bevy::prelude::*;
use game_core::lod::{self, LOD_DISTANCES, LOD_MAX_DISTANCE};

/// Marker component indicating this entity's LOD visibility has been configured.
#[derive(Component)]
pub struct LodConfigured;

/// Applies [`VisibilityRange`] to mesh children of LOD node entities.
///
/// Runs every frame to pick up asynchronously spawned scene children.
/// Named parent nodes (e.g. `SM_Env_Tree_Birch_01_LOD1`) identify the LOD level,
/// but `VisibilityRange` must be placed on the actual `Mesh3d` child entities
/// for rendering to respect it.
pub fn configure_lod_visibility(
    mut commands: Commands,
    lod_nodes: Query<(Entity, &Name, &Children), Without<LodConfigured>>,
    mesh_query: Query<Entity, With<Mesh3d>>,
) {
    for (node_entity, name, children) in &lod_nodes {
        let Some(level) = lod::parse_lod_level(name.as_str()) else {
            continue;
        };

        let level = level as usize;
        let num_levels = LOD_DISTANCES.len();

        if level >= num_levels {
            // LOD level beyond our table — hide the parent (cascades to children)
            commands.entity(node_entity).insert((
                Visibility::Hidden,
                LodConfigured,
            ));
            continue;
        }

        // Fade-in range: for LOD0 starts at 0, for others at this level's threshold
        let start_margin = if level == 0 {
            0.0..0.0
        } else {
            LOD_DISTANCES[level].0..LOD_DISTANCES[level].1
        };

        // Fade-out range: next level's fade-in, or max distance for the last level
        let end_margin = if level + 1 < num_levels {
            LOD_DISTANCES[level + 1].0..LOD_DISTANCES[level + 1].1
        } else {
            LOD_MAX_DISTANCE.0..LOD_MAX_DISTANCE.1
        };

        let visibility_range = VisibilityRange {
            start_margin,
            end_margin,
            use_aabb: false,
        };

        // Apply VisibilityRange to each mesh child
        for child in children.iter() {
            if mesh_query.get(child).is_ok() {
                commands.entity(child).insert(visibility_range.clone());
            }
        }

        commands.entity(node_entity).insert(LodConfigured);
    }
}
