use bevy::prelude::*;

use super::SpatialGrid;
use crate::core::GridCell;

const GRID_CELL_SIZE: u8 = 128;

fn pos_to_grid_coords(pos: &Vec3) -> IVec2 {
    IVec2 {
        x: (pos.x / GRID_CELL_SIZE as f32).floor() as i32,
        y: (pos.z / GRID_CELL_SIZE as f32).floor() as i32,
    }
}

pub fn update_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    mut commands: Commands,
    q_entities: Query<(Entity, &Transform)>,
) {
    // TODO: Don't clear the grid every frame
    grid.cells.clear();

    for (entity, transform) in q_entities.iter() {
        let coords = pos_to_grid_coords(&transform.translation);
        commands.entity(entity).insert(GridCell(coords));
        grid.cells.entry(coords).or_default().push(entity);
    }
}
