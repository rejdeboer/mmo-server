use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollisionType {
    #[default]
    None,
    ConvexHull,
    TrimeshFromMesh,
}

/// Material color definition for the foliage shader.
#[derive(Deserialize, Debug, Clone)]
pub struct FoliageMaterialDef {
    pub leaf_base_color: (f32, f32, f32),
    pub leaf_noise_color: (f32, f32, f32),
    pub leaf_large_noise_color: (f32, f32, f32),
    pub trunk_base_color: (f32, f32, f32),
    pub trunk_noise_color: (f32, f32, f32),
    pub noise_small_freq: f32,
    pub noise_large_freq: f32,
    pub wind_strength: f32,
    pub wind_frequency: f32,
}

/// Per-model configuration: collision + mesh material assignments.
#[derive(Deserialize, Debug, Clone)]
pub struct PropDef {
    pub collision: CollisionType,
    #[serde(default)]
    pub meshes: HashMap<String, String>,
}

/// Root config loaded from `props.ron`.
///
/// Defines material palettes and per-model properties (collision, materials).
/// Shared by both client and server — the server ignores `materials` and mesh
/// assignments, reading only collision types.
#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct PropsConfig {
    pub materials: HashMap<String, FoliageMaterialDef>,
    pub props: HashMap<String, PropDef>,
}

#[derive(Resource)]
pub struct PropsConfigHandle(pub Handle<PropsConfig>);

/// Extracts the model base name from an asset path.
///
/// `"world/props/meadow/Models/SM_Env_Tree_Meadow_02.glb"` → `"SM_Env_Tree_Meadow_02"`
pub fn model_name_from_asset_path(path: &str) -> &str {
    let filename = path.rsplit('/').next().unwrap_or(path);
    filename.strip_suffix(".glb").unwrap_or(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_name_from_asset_path() {
        assert_eq!(
            model_name_from_asset_path("world/props/meadow/Models/SM_Env_Tree_Meadow_02.glb"),
            "SM_Env_Tree_Meadow_02"
        );
        assert_eq!(
            model_name_from_asset_path("SM_Env_Bush_01.glb"),
            "SM_Env_Bush_01"
        );
        assert_eq!(model_name_from_asset_path("foo"), "foo");
    }
}
