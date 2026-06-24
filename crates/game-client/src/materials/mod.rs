pub mod foliage;

use bevy::prelude::*;
use foliage::{FoliageExtension, FoliageMaterial};

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FoliageMaterial>::default());
        app.add_systems(Update, apply_foliage_material);
    }
}

/// Marker indicating this entity has already been processed for material replacement.
#[derive(Component)]
struct FoliageMaterialApplied;

/// Identifies foliage mesh entities and replaces their `StandardMaterial`
/// with the extended `FoliageMaterial`.
///
/// Mesh entities don't have `Name` directly — the name is on their parent node.
/// This system queries mesh entities, traverses up via `ChildOf` to find the
/// ancestor name, and checks if it matches foliage patterns.
fn apply_foliage_material(
    mut commands: Commands,
    mesh_query: Query<
        (Entity, &ChildOf, &MeshMaterial3d<StandardMaterial>),
        Without<FoliageMaterialApplied>,
    >,
    name_query: Query<&Name>,
    mut foliage_materials: ResMut<Assets<FoliageMaterial>>,
    standard_materials: Res<Assets<StandardMaterial>>,
) {
    for (entity, child_of, standard_handle) in &mesh_query {
        let Ok(parent_name) = name_query.get(child_of.0) else {
            continue;
        };

        if !is_foliage_name(parent_name.as_str()) {
            continue;
        }

        // Build extended material: keep the base StandardMaterial for alpha/textures,
        // add foliage extension for noise coloring
        let base = standard_materials
            .get(&standard_handle.0)
            .cloned()
            .unwrap_or_default();

        let handle = foliage_materials.add(FoliageMaterial {
            base,
            extension: FoliageExtension::default(),
        });

        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(handle), FoliageMaterialApplied));
    }
}

/// Returns true if the entity name matches foliage prop naming conventions.
///
/// Synty PolygonNatureBiomes foliage props follow patterns like:
/// - `SM_Env_Tree_*` (trees)
/// - `SM_Env_Bush_*` (bushes)
/// - `SM_Env_Hedge_*` (hedges)
/// - `SM_Env_Plant_*` (plants)
/// - `SM_Env_Vine_*` (vines)
fn is_foliage_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    upper.contains("_TREE_")
        || upper.contains("_BUSH_")
        || upper.contains("_HEDGE_")
        || upper.contains("_PLANT_")
        || upper.contains("_VINE_")
}
