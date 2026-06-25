pub mod foliage;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use game_core::props::{
    AlphaModeDef, FoliageMaterialDef, MaterialDef, PropsConfig, PropsConfigHandle,
    StandardMaterialDef,
};

use foliage::{FoliageExtension, FoliageMaterial};

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FoliageMaterial>::default());
        app.add_plugins(RonAssetPlugin::<PropsConfig>::new(&["props.ron"]));
        app.add_systems(Startup, load_props_config);
        app.add_systems(Update, (build_material_map, apply_materials).chain());
    }
}

/// Resolved material assignment for a mesh, ready to apply.
#[derive(Clone)]
enum ResolvedMaterial {
    Foliage {
        extension: FoliageExtension,
        texture: Option<Handle<Image>>,
    },
    Standard {
        def: StandardMaterialDef,
        texture: Option<Handle<Image>>,
    },
}

/// Maps mesh base name to its resolved material assignment.
#[derive(Resource)]
struct MaterialMap(HashMap<String, ResolvedMaterial>);

/// Marker indicating this entity has already been processed for material replacement.
#[derive(Component)]
struct MaterialApplied;

fn load_props_config(mut commands: Commands, assets: Res<AssetServer>) {
    let handle = assets.load::<PropsConfig>("world/props.ron");
    commands.insert_resource(PropsConfigHandle(handle));
}

/// Once the config is loaded, resolve all mesh name → material assignments.
fn build_material_map(
    mut commands: Commands,
    config_handle: Option<Res<PropsConfigHandle>>,
    config_assets: Res<Assets<PropsConfig>>,
    existing_map: Option<Res<MaterialMap>>,
    assets: Res<AssetServer>,
) {
    if existing_map.is_some() {
        return;
    }

    let Some(handle) = config_handle else { return };
    let Some(config) = config_assets.get(&handle.0) else {
        return;
    };

    // Resolve each material definition into its ready-to-apply form
    let mut resolved: HashMap<String, ResolvedMaterial> = HashMap::new();
    for (mat_name, mat_def) in &config.materials {
        let r = match mat_def {
            MaterialDef::Foliage(def) => ResolvedMaterial::Foliage {
                extension: foliage_extension_from_def(def),
                texture: def
                    .base_color_texture
                    .as_ref()
                    .map(|p| assets.load::<Image>(p.clone())),
            },
            MaterialDef::Standard(def) => ResolvedMaterial::Standard {
                def: def.clone(),
                texture: def
                    .base_color_texture
                    .as_ref()
                    .map(|p| assets.load::<Image>(p.clone())),
            },
        };
        resolved.insert(mat_name.clone(), r);
    }

    // Build mesh name → resolved material map from all prop mesh assignments
    let mut map = HashMap::new();
    for prop_def in config.props.values() {
        for (mesh_name, mat_name) in &prop_def.meshes {
            if let Some(r) = resolved.get(mat_name) {
                map.insert(mesh_name.clone(), r.clone());
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
        materials = resolved.len(),
        mesh_assignments = map.len(),
        "material map built"
    );
    commands.insert_resource(MaterialMap(map));
}

/// Identifies mesh entities by looking up their parent name in the material map,
/// then applies the configured material — either replacing with a FoliageMaterial
/// or patching the existing StandardMaterial.
fn apply_materials(
    mut commands: Commands,
    mesh_query: Query<
        (Entity, &ChildOf, &MeshMaterial3d<StandardMaterial>),
        Without<MaterialApplied>,
    >,
    name_query: Query<&Name>,
    material_map: Option<Res<MaterialMap>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut foliage_materials: ResMut<Assets<FoliageMaterial>>,
    mut foliage_cache: Local<HashMap<(AssetId<StandardMaterial>, String), Handle<FoliageMaterial>>>,
) {
    let Some(map) = material_map else { return };

    for (entity, child_of, std_mat) in &mesh_query {
        let Ok(parent_name) = name_query.get(child_of.0) else {
            continue;
        };

        let base_name = strip_lod_suffix(parent_name.as_str());
        let Some(resolved) = map.0.get(base_name) else {
            continue;
        };

        match resolved {
            ResolvedMaterial::Foliage { extension, texture } => {
                let std_id = std_mat.0.id();
                let cache_key = (std_id, base_name.to_string());

                let foliage_handle = foliage_cache.entry(cache_key).or_insert_with(|| {
                    let mut base = standard_materials
                        .get(&std_mat.0)
                        .cloned()
                        .unwrap_or_default();
                    base.cull_mode = None;
                    if let Some(tex) = texture {
                        base.base_color_texture = Some(tex.clone());
                        base.alpha_mode = AlphaMode::Blend;
                    }
                    foliage_materials.add(FoliageMaterial {
                        base,
                        extension: extension.clone(),
                    })
                });

                commands.entity(entity).remove::<MeshMaterial3d<StandardMaterial>>().insert((
                    MeshMaterial3d(foliage_handle.clone()),
                    MaterialApplied,
                ));
            }
            ResolvedMaterial::Standard { def, texture } => {
                if let Some(mat) = standard_materials.get_mut(&std_mat.0) {
                    apply_standard_overrides(mat, def, texture);
                }
                commands.entity(entity).insert(MaterialApplied);
            }
        }
    }
}

/// Applies config overrides to a StandardMaterial, preserving unset fields.
fn apply_standard_overrides(
    mat: &mut StandardMaterial,
    def: &StandardMaterialDef,
    texture: &Option<Handle<Image>>,
) {
    if let Some((r, g, b)) = def.base_color {
        mat.base_color = Srgba::new(r, g, b, 1.0).into();
    }
    if let Some(v) = def.perceptual_roughness {
        mat.perceptual_roughness = v;
    }
    if let Some(v) = def.metallic {
        mat.metallic = v;
    }
    if let Some((r, g, b, intensity)) = def.emissive {
        mat.emissive = LinearRgba::new(r, g, b, intensity);
    }
    if let Some(v) = def.reflectance {
        mat.reflectance = v;
    }
    if let Some(tex) = texture {
        mat.base_color_texture = Some(tex.clone());
    }
    if let Some(alpha) = def.alpha_mode {
        mat.alpha_mode = match alpha {
            AlphaModeDef::Opaque => AlphaMode::Opaque,
            AlphaModeDef::Blend => AlphaMode::Blend,
            AlphaModeDef::Mask(cutoff) => AlphaMode::Mask(cutoff),
        };
    }
    if let Some(double_sided) = def.double_sided {
        mat.cull_mode = if double_sided {
            None
        } else {
            Some(bevy::render::render_resource::Face::Back)
        };
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
