pub mod foliage;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use game_core::props::{FoliageMaterialDef, PropsConfig, PropsConfigHandle};

use foliage::{FoliageExtension, FoliageMaterial};

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FoliageMaterial>::default());
        app.add_plugins(RonAssetPlugin::<PropsConfig>::new(&["props.ron"]));
        app.add_systems(Startup, load_props_config);
        app.add_systems(Update, (build_material_map, apply_foliage_material).chain());
    }
}

/// Pre-built map from mesh base name to foliage material handle.
#[derive(Resource)]
struct FoliageMaterialMap(HashMap<String, Handle<FoliageMaterial>>);

/// Marker indicating this entity has already been processed for material replacement.
#[derive(Component)]
struct FoliageMaterialApplied;

fn load_props_config(mut commands: Commands, assets: Res<AssetServer>) {
    let handle = assets.load::<PropsConfig>("world/props.ron");
    commands.insert_resource(PropsConfigHandle(handle));
}

/// Once the config is loaded, pre-create all material handles and build the lookup map.
fn build_material_map(
    mut commands: Commands,
    config_handle: Option<Res<PropsConfigHandle>>,
    config_assets: Res<Assets<PropsConfig>>,
    mut foliage_materials: ResMut<Assets<FoliageMaterial>>,
    existing_map: Option<Res<FoliageMaterialMap>>,
) {
    if existing_map.is_some() {
        return;
    }

    let Some(handle) = config_handle else { return };
    let Some(config) = config_assets.get(&handle.0) else {
        return;
    };

    // Create one material handle per unique material definition
    let mut material_handles: HashMap<String, Handle<FoliageMaterial>> = HashMap::new();
    for (mat_name, mat_def) in &config.materials {
        let extension = foliage_extension_from_def(mat_def);
        let handle = foliage_materials.add(FoliageMaterial {
            base: StandardMaterial {
                alpha_mode: AlphaMode::Mask(0.5),
                cull_mode: None,
                ..default()
            },
            extension,
        });
        material_handles.insert(mat_name.clone(), handle);
    }

    // Build the mesh name → material handle map from all prop mesh assignments
    let mut map = HashMap::new();
    for prop_def in config.props.values() {
        for (mesh_name, mat_name) in &prop_def.meshes {
            if let Some(handle) = material_handles.get(mat_name) {
                map.insert(mesh_name.clone(), handle.clone());
            } else {
                tracing::warn!(
                    mesh = %mesh_name,
                    material = %mat_name,
                    "mesh material assignment references undefined material"
                );
            }
        }
    }

    tracing::info!(
        materials = material_handles.len(),
        mesh_assignments = map.len(),
        "foliage material map built"
    );
    commands.insert_resource(FoliageMaterialMap(map));
}

/// Identifies foliage mesh entities by looking up their parent name in the
/// material map, and replaces their `StandardMaterial` with the configured
/// `FoliageMaterial`.
fn apply_foliage_material(
    mut commands: Commands,
    mesh_query: Query<
        (Entity, &ChildOf, &MeshMaterial3d<StandardMaterial>),
        Without<FoliageMaterialApplied>,
    >,
    name_query: Query<&Name>,
    material_map: Option<Res<FoliageMaterialMap>>,
) {
    let Some(map) = material_map else { return };

    for (entity, child_of, _) in &mesh_query {
        let Ok(parent_name) = name_query.get(child_of.0) else {
            continue;
        };

        let base_name = strip_lod_suffix(parent_name.as_str());
        let Some(handle) = map.0.get(base_name) else {
            continue;
        };

        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(handle.clone()), FoliageMaterialApplied));
    }
}

fn foliage_extension_from_def(def: &FoliageMaterialDef) -> FoliageExtension {
    FoliageExtension {
        leaf_base_color: Srgba::new(
            def.leaf_base_color.0,
            def.leaf_base_color.1,
            def.leaf_base_color.2,
            1.0,
        )
        .into(),
        leaf_noise_color: Srgba::new(
            def.leaf_noise_color.0,
            def.leaf_noise_color.1,
            def.leaf_noise_color.2,
            1.0,
        )
        .into(),
        leaf_large_noise_color: Srgba::new(
            def.leaf_large_noise_color.0,
            def.leaf_large_noise_color.1,
            def.leaf_large_noise_color.2,
            1.0,
        )
        .into(),
        trunk_base_color: Srgba::new(
            def.trunk_base_color.0,
            def.trunk_base_color.1,
            def.trunk_base_color.2,
            1.0,
        )
        .into(),
        trunk_noise_color: Srgba::new(
            def.trunk_noise_color.0,
            def.trunk_noise_color.1,
            def.trunk_noise_color.2,
            1.0,
        )
        .into(),
        params: Vec4::new(
            def.noise_small_freq,
            def.noise_large_freq,
            def.wind_strength,
            def.wind_frequency,
        ),
    }
}

/// Strips `_LOD\d+` suffix from a mesh node name to get the base asset name.
fn strip_lod_suffix(name: &str) -> &str {
    if let Some(idx) = name.rfind("_LOD") {
        let suffix = &name[idx + 4..];
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            return &name[..idx];
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_lod_suffix() {
        assert_eq!(
            strip_lod_suffix("SM_Env_Tree_Meadow_02_LOD0"),
            "SM_Env_Tree_Meadow_02"
        );
        assert_eq!(
            strip_lod_suffix("SM_Env_Tree_Meadow_02_Branches_LOD1"),
            "SM_Env_Tree_Meadow_02_Branches"
        );
        assert_eq!(strip_lod_suffix("SM_Env_Bush_01_LOD2"), "SM_Env_Bush_01");
        assert_eq!(
            strip_lod_suffix("SM_Env_Grass_Med_Plane_01"),
            "SM_Env_Grass_Med_Plane_01"
        );
        assert_eq!(strip_lod_suffix("SM_Env_LODge_01"), "SM_Env_LODge_01");
    }
}
