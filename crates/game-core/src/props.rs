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

/// Serializable alpha mode for materials.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum AlphaModeDef {
    Opaque,
    Blend,
    Mask(f32),
}

/// A material definition that can be one of several types.
#[derive(Deserialize, Debug, Clone)]
pub enum MaterialDef {
    /// Custom foliage shader with noise coloring and wind displacement.
    Foliage(FoliageMaterialDef),
    /// Override fields on the standard PBR material. Unset fields keep the GLB defaults.
    Standard(StandardMaterialDef),
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
    /// Optional base color texture path (relative to assets root).
    /// Used for alpha masking (e.g., grass blade shape).
    #[serde(default)]
    pub base_color_texture: Option<String>,
}

/// Override fields on the standard PBR material.
///
/// All fields are optional — only specified values override the GLB defaults.
#[derive(Deserialize, Debug, Clone, Default)]
pub struct StandardMaterialDef {
    pub base_color: Option<(f32, f32, f32)>,
    pub perceptual_roughness: Option<f32>,
    pub metallic: Option<f32>,
    pub emissive: Option<(f32, f32, f32, f32)>,
    pub reflectance: Option<f32>,
    /// Optional base color texture path (relative to assets root).
    #[serde(default)]
    pub base_color_texture: Option<String>,
    /// Override alpha blending mode.
    #[serde(default)]
    pub alpha_mode: Option<AlphaModeDef>,
    /// Override backface culling (true = render both sides).
    #[serde(default)]
    pub double_sided: Option<bool>,
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
    pub materials: HashMap<String, MaterialDef>,
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
