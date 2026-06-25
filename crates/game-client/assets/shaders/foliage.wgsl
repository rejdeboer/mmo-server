#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::morph,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput as PrepassVertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::FragmentOutput,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct FoliageExtensionUniform {
    leaf_base_color: vec4<f32>,
    leaf_noise_color: vec4<f32>,
    leaf_large_noise_color: vec4<f32>,
    trunk_base_color: vec4<f32>,
    trunk_noise_color: vec4<f32>,
    // x = small_freq, y = large_freq, z = wind_strength, w = wind_frequency
    params: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> foliage: FoliageExtensionUniform;

// ─── Vertex shader ──────────────────────────────────────────────────────────

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    // Wind displacement using vertex color R as weight
#ifdef VERTEX_COLORS
    let wind_strength = foliage.params.z;
    let wind_freq = foliage.params.w;

    if wind_strength > 0.0 {
        let wind_weight = vertex.color.r;
        let world_x = out.world_position.x;
        let sway = wind_weight * wind_strength * sin(globals.time * wind_freq + world_x * 0.5);
        out.world_position.x += sway;
        out.world_position.z += sway * 0.3;
    }
#endif

    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}

// ─── Fragment shader ────────────────────────────────────────────────────────

// Simple value noise for world-position-based color variation
fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Build standard PBR input from base StandardMaterial
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Determine leaf vs trunk from vertex color B channel.
    // The FBX asset stores B > 0.5 for leaf vertices and B = 0 for trunk.
    let world_xz = in.world_position.xz;
    let small_freq = foliage.params.x;
    let large_freq = foliage.params.y;
    let noise_small = noise2d(world_xz * small_freq);
    let noise_large = noise2d(world_xz * large_freq);

#ifdef VERTEX_COLORS
    let leaf_factor = step(0.5, in.color.b);
#else
    let leaf_factor = 1.0;
#endif

    var leaf_color = mix(foliage.leaf_noise_color.rgb, foliage.leaf_base_color.rgb, noise_small);
    leaf_color = mix(leaf_color, foliage.leaf_large_noise_color.rgb, noise_large * 0.4);

    let trunk_color = mix(foliage.trunk_noise_color.rgb, foliage.trunk_base_color.rgb, noise_small);

    let final_color = mix(trunk_color, leaf_color, leaf_factor);
    pbr_input.material.base_color = vec4<f32>(final_color, pbr_input.material.base_color.a);

    // Alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
