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
/// B > 0.5 = leaf (green), B = 0 = trunk (brown).
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

impl Default for FoliageExtension {
    fn default() -> Self {
        Self {
            leaf_base_color: LinearRgba::new(0.20, 0.50, 0.10, 1.0),
            leaf_noise_color: LinearRgba::new(0.10, 0.35, 0.05, 1.0),
            leaf_large_noise_color: LinearRgba::new(0.40, 0.55, 0.08, 1.0),
            trunk_base_color: LinearRgba::new(0.30, 0.18, 0.08, 1.0),
            trunk_noise_color: LinearRgba::new(0.18, 0.10, 0.04, 1.0),
            params: Vec4::new(
                0.3,  // color_noise_small_freq
                0.08, // color_noise_large_freq
                0.4,  // wind_strength
                1.5,  // wind_frequency
            ),
        }
    }
}

impl FoliageExtension {
    /// Variant for branch-only meshes: brown coloring + wind.
    /// Branch meshes have B=0 for all vertices, so only trunk colors are
    /// used. Both leaf and trunk are set to brown for consistency.
    pub fn branches() -> Self {
        Self {
            leaf_base_color: LinearRgba::new(0.35, 0.22, 0.12, 1.0),
            leaf_noise_color: LinearRgba::new(0.25, 0.15, 0.08, 1.0),
            leaf_large_noise_color: LinearRgba::new(0.40, 0.25, 0.12, 1.0),
            trunk_base_color: LinearRgba::new(0.30, 0.18, 0.08, 1.0),
            trunk_noise_color: LinearRgba::new(0.18, 0.10, 0.04, 1.0),
            params: Vec4::new(
                0.3,  // color_noise_small_freq
                0.08, // color_noise_large_freq
                0.4,  // wind_strength
                1.5,  // wind_frequency
            ),
        }
    }
}

/// Convenience type alias for the foliage material.
pub type FoliageMaterial = ExtendedMaterial<StandardMaterial, FoliageExtension>;
