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

        let name = parent_name.as_str();
        if !is_foliage_name(name) {
            continue;
        }

        let Some(base) = standard_materials.get(&standard_handle.0).cloned() else {
            continue;
        };

        // Branches mesh: B=0, gets brown trunk coloring with wind.
        // Tree/bush mesh: B>0.5 = leaf (green), B=0 = trunk (brown).
        let extension = if is_branch_name(name) {
            FoliageExtension::branches()
        } else {
            FoliageExtension::default()
        };

        let handle = foliage_materials.add(FoliageMaterial { base, extension });

        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(handle), FoliageMaterialApplied));
    }
}

/// Returns true if the entity name matches foliage prop naming conventions.
///
/// Synty PolygonNatureBiomes foliage props follow patterns like:
/// - `SM_Env_Tree_*` (trees — mesh contains both trunk and leaves)
/// - `SM_Env_Tree_*_Branches_*` (separate branch meshes)
/// - `SM_Env_Bush_*` (bushes)
/// - `SM_Env_Grass_*` (grass clumps/patches)
/// - `SM_Env_Hedge_*` (hedges)
/// - `SM_Env_Plant_*` (plants)
/// - `SM_Env_Vine_*` (vines)
fn is_foliage_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    upper.contains("_TREE_")
        || upper.contains("_BUSH_")
        || upper.contains("_GRASS_")
        || upper.contains("_HEDGE_")
        || upper.contains("_PLANT_")
        || upper.contains("_VINE_")
}

/// Returns true if the name indicates a branch-only mesh.
///
/// Branch meshes are entirely foliage (all vertices sway) but colored
/// light brown rather than green.
fn is_branch_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    upper.contains("BRANCH")
}
