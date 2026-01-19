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

struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    position: vec3<f32>,
    xray_alpha: f32,
    flat_shading: f32,
    auto_normals: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) var gbuffer_normals: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var samp: sampler;

struct SsaoParams {
    strength: vec4<f32>,
}
@group(0) @binding(3) var<uniform> params: SsaoParams;
@group(0) @binding(4) var<uniform> camera: Camera;

fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    let world = camera.inv_view_proj * ndc;
    let world_pos = world.xyz / world.w;
    let view_pos4 = camera.view * vec4<f32>(world_pos, 1.0);
    return view_pos4.xyz;
}

@fragment
fn fs_ssao(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let n = textureSample(gbuffer_normals, samp, uv).xyz * 2.0 - vec3<f32>(1.0);
    let depth = textureSample(depth_tex, samp, uv);

    // If there's no geometry (far plane), keep background masked out.
    if depth >= 0.999 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let p = reconstruct_view_pos(uv, depth);

    // Simple SSAO: sample 4 taps around pixel in screen space.
    let radius = 0.002 * clamp(abs(p.z), 0.5, 10.0);
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
        let sample_pos = reconstruct_view_pos(uv + duv, sample_depth);
        let delta = sample_pos.z - p.z;
        if delta < -0.02 {
            occlusion = occlusion + 0.25;
        }
    }

    // Use normal to reduce occlusion on grazing surfaces.
    let ndotv = max(n.z, 0.0);
    let ao = 1.0 - occlusion * params.strength.x * (1.0 - ndotv);
    return vec4<f32>(ao, ao, ao, 1.0);
}
"#;

pub const SSAO_BLUR_SHADER: &str = r#"
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

@group(0) @binding(0) var occlusion_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct BlurParams {
    direction: vec2<f32>,
    _pad: vec2<f32>,
}
@group(0) @binding(2) var<uniform> blur: BlurParams;

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let dims = vec2<f32>(textureDimensions(occlusion_tex));
    let texel = blur.direction / dims;

    let c0 = textureSample(occlusion_tex, samp, uv).r * 0.4;
    let c1 = textureSample(occlusion_tex, samp, uv + texel).r * 0.15;
    let c2 = textureSample(occlusion_tex, samp, uv - texel).r * 0.15;
    let c3 = textureSample(occlusion_tex, samp, uv + texel * 2.0).r * 0.15;
    let c4 = textureSample(occlusion_tex, samp, uv - texel * 2.0).r * 0.15;
    let blurred = c0 + c1 + c2 + c3 + c4;
    return vec4<f32>(blurred, blurred, blurred, 1.0);
}
"#;

pub const LIGHTING_SHADER: &str = r#"
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

struct Light {
    direction: vec3<f32>,
    _pad1: f32,
    color: vec3<f32>,
    intensity: f32,
}

struct LightRig {
    key: Light,
    fill: Light,
    rim: Light,
    ambient: vec3<f32>,
    _pad: f32,
}

@group(0) @binding(0) var gbuffer_albedo: texture_2d<f32>;
@group(0) @binding(1) var gbuffer_normals: texture_2d<f32>;
@group(0) @binding(2) var gbuffer_mask: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> lights: LightRig;

struct LightingParams {
    background: vec4<f32>,
}
@group(0) @binding(5) var<uniform> params: LightingParams;

@fragment
fn fs_lighting(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let occlusion = textureSample(gbuffer_mask, samp, uv).r;
    if occlusion < 0.5 {
        return params.background;
    }

    let albedo_rgba = textureSample(gbuffer_albedo, samp, uv);
    let normal_rgba = textureSample(gbuffer_normals, samp, uv);

    let albedo = albedo_rgba.rgb;
    let roughness = clamp(albedo_rgba.a, 0.02, 1.0);
    let metalness = clamp(normal_rgba.a, 0.0, 1.0);
    let n = normalize(normal_rgba.rgb * 2.0 - vec3<f32>(1.0));

    // Stage 1: simple view-independent lighting (no position reconstruction yet).
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);

    let key_l = normalize(-lights.key.direction);
    let fill_l = normalize(-lights.fill.direction);
    let rim_l = normalize(-lights.rim.direction);

    let key_ndotl = max(dot(n, key_l), 0.0);
    let fill_ndotl = max(dot(n, fill_l), 0.0);
    let rim_ndotl = max(dot(n, rim_l), 0.0);

    let diffuse = albedo * (lights.ambient +
        key_ndotl * lights.key.color * lights.key.intensity +
        fill_ndotl * lights.fill.color * lights.fill.intensity +
        rim_ndotl * lights.rim.color * lights.rim.intensity);

    // Cheap specular: Blinn-Phong with roughness-based exponent.
    let spec_exp = mix(8.0, 128.0, 1.0 - roughness);
    let spec_color = mix(vec3<f32>(0.04), albedo, metalness);
    let half_key = normalize(key_l + view_dir);
    let spec = pow(max(dot(n, half_key), 0.0), spec_exp) * spec_color * key_ndotl;

    return vec4<f32>((diffuse + spec) * occlusion, 1.0);
}
"#;
