use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// Extension for the standard PBR material that adds foliage noise coloring and wind.
///
/// Used with `ExtendedMaterial<StandardMaterial, FoliageExtension>`.
///
/// Leaf vs trunk coloring is determined by vertex color B channel:
/// B > 0.5 = leaf, B = 0 = trunk.
/// Wind displacement uses vertex color R as sway weight.
///
/// Coloring is derived from world-position noise rather than textures,
/// matching the stylized look of the PolygonNatureBiomes asset pack.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FoliageExtension {
    #[uniform(100)]
    pub leaf_base_color: LinearRgba,
    #[uniform(100)]
    pub leaf_noise_color: LinearRgba,
    #[uniform(100)]
    pub leaf_large_noise_color: LinearRgba,
    #[uniform(100)]
    pub trunk_base_color: LinearRgba,
    #[uniform(100)]
    pub trunk_noise_color: LinearRgba,
    /// x = small_freq, y = large_freq, z = wind_strength, w = wind_frequency
    #[uniform(100)]
    pub params: Vec4,
}

impl MaterialExtension for FoliageExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/foliage.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/foliage.wgsl".into()
    }
}

/// Convenience type alias for the foliage material.
pub type FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageExtension>;
