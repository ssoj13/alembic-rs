//! Embedded full-screen shaders for SSAO and composition.

pub const SSAO_SHADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let pos = positions[index];
    var out: VsOut;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
    return out;
}

@group(0) @binding(0) var gbuffer_normals: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var samp: sampler;

struct SsaoParams {
    strength: vec4<f32>,
}
@group(0) @binding(3) var<uniform> params: SsaoParams;

@fragment
fn fs_ssao(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let n = textureSample(gbuffer_normals, samp, uv).xyz * 2.0 - vec3<f32>(1.0);
    let depth = textureSample(depth_tex, samp, uv);

    // Simple SSAO: sample 4 taps around pixel in screen space.
    let radius = 0.004;
    var occlusion = 0.0;
    let offsets = array<vec2<f32>, 4>(
        vec2<f32>(radius, 0.0),
        vec2<f32>(-radius, 0.0),
        vec2<f32>(0.0, radius),
        vec2<f32>(0.0, -radius)
    );
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let duv = offsets[i];
        let sample_depth = textureSample(depth_tex, samp, uv + duv);
        let delta = sample_depth - depth;
        if delta < -0.01 {
            occlusion = occlusion + 0.25;
        }
    }

    // Use normal to reduce occlusion on grazing surfaces.
    let ndotv = max(n.z, 0.0);
    let ao = 1.0 - occlusion * params.strength.x * (1.0 - ndotv);
    return vec4<f32>(ao, ao, ao, 1.0);
}
"#;

pub const COMPOSITE_SHADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let pos = positions[index];
    var out: VsOut;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
    return out;
}

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var occlusion_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let color = textureSample(color_tex, samp, uv);
    let occlusion = textureSample(occlusion_tex, samp, uv).r;
    return vec4<f32>(color.rgb * occlusion, color.a);
}
"#;
