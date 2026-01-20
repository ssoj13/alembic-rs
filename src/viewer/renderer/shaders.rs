//! Fullscreen shaders for post-processing (SSAO, lighting composition).
//!
//! Coordinate conventions (wgpu):
//! - NDC: X [-1,+1] left→right, Y [-1,+1] bottom→top, Z [0,1] near→far
//! - UV:  U [0,1] left→right, V [0,1] top→bottom (Y flipped vs NDC)
//!
//! We pass BOTH uv and ndc from vertex shader to avoid any conversions in fragment shader.

/// Common vertex shader and struct for all fullscreen passes
pub const FULLSCREEN_VERTEX: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) ndc: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    // Fullscreen triangle covering [-1,1] NDC
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let p = positions[index];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.ndc = p;
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}
"#;

/// SSAO fragment shader - simple screen-space ambient occlusion
pub const SSAO_FRAGMENT: &str = r#"
const PI: f32 = 3.141592653589793;

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

// Reconstruct view-space position from depth
fn reconstruct_view_pos(ndc_xy: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(ndc_xy, depth, 1.0);
    let world = camera.inv_view_proj * ndc;
    let world_pos = world.xyz / world.w;
    // Transform to view space
    let view_pos = camera.view * vec4<f32>(world_pos, 1.0);
    return view_pos.xyz;
}

// Transform world normal to view space
fn normal_to_view(world_normal: vec3<f32>) -> vec3<f32> {
    // Extract rotation part of view matrix (upper 3x3)
    let view_rot = mat3x3<f32>(
        camera.view[0].xyz,
        camera.view[1].xyz,
        camera.view[2].xyz
    );
    return normalize(view_rot * world_normal);
}

fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
}

@fragment
fn fs_ssao(in: VsOut) -> @location(0) vec4<f32> {
    let center_depth = textureSample(depth_tex, samp, in.uv);
    
    // Skip background
    if center_depth >= 0.999 {
        return vec4<f32>(1.0, 1.0, 1.0, 0.0);
    }
    
    // Load normal and convert to view space
    let world_normal = normalize(textureSample(gbuffer_normals, samp, in.uv).xyz * 2.0 - 1.0);
    let view_normal = normal_to_view(world_normal);
    
    // Reconstruct view-space position
    let view_pos = reconstruct_view_pos(in.ndc, center_depth);
    
    // Adaptive radius: larger samples for distant pixels
    // In view space, -Z is forward, so abs(view_pos.z) is distance
    let pixel_depth = abs(view_pos.z);
    let base_radius = 0.05; // UV space radius at depth=1
    let screen_radius = base_radius * clamp(pixel_depth * 0.02, 0.5, 4.0);
    
    // 8 samples in Poisson disc pattern
    let samples = array<vec2<f32>, 8>(
        vec2<f32>(-0.326, -0.406),
        vec2<f32>(-0.840, -0.074),
        vec2<f32>(-0.696,  0.457),
        vec2<f32>(-0.203,  0.621),
        vec2<f32>( 0.962, -0.195),
        vec2<f32>( 0.473, -0.480),
        vec2<f32>( 0.519,  0.767),
        vec2<f32>( 0.185, -0.893)
    );
    
    var occlusion = 0.0;
    let falloff_start = pixel_depth * 0.01;  // Start falloff at 1% of depth
    let falloff_end = pixel_depth * 0.1;     // End falloff at 10% of depth
    
    for (var i = 0u; i < 8u; i = i + 1u) {
        let offset = samples[i] * screen_radius;
        let sample_uv = clamp(in.uv + offset, vec2<f32>(0.001), vec2<f32>(0.999));
        let sample_depth = textureSample(depth_tex, samp, sample_uv);
        
        // Skip background
        if sample_depth >= 0.999 {
            continue;
        }
        
        // Reconstruct sample position in view space
        let sample_ndc = uv_to_ndc(sample_uv);
        let sample_view_pos = reconstruct_view_pos(sample_ndc, sample_depth);
        
        // Vector from center to sample in view space
        let diff = sample_view_pos - view_pos;
        let dist = length(diff);
        
        // Hemisphere test: is sample below tangent plane?
        // In view space, normal points toward camera (positive Z component expected)
        let diff_normalized = diff / max(dist, 0.0001);
        let cos_angle = dot(view_normal, diff_normalized);
        
        // Sample contributes occlusion if:
        // 1. It's below the tangent plane (cos_angle < bias)
        // 2. It's within reasonable range (prevents halos on distant objects)
        let bias = 0.05;
        if cos_angle < -bias {
            // Range check with smooth falloff
            let range = smoothstep(falloff_end, falloff_start, dist);
            // Angle-based falloff (more occlusion when sample is directly below)
            let angle_factor = clamp(-cos_angle, 0.0, 1.0);
            occlusion = occlusion + range * angle_factor;
        }
    }
    
    // Normalize and apply strength
    occlusion = occlusion / 8.0;
    let ao = 1.0 - saturate(occlusion * params.strength.x * 2.0);
    
    return vec4<f32>(ao, ao, ao, 1.0);
}
"#;

/// SSAO blur fragment shader
pub const SSAO_BLUR_FRAGMENT: &str = r#"
@group(0) @binding(0) var occlusion_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct BlurParams {
    direction: vec2<f32>,
    _pad: vec2<f32>,
}
@group(0) @binding(2) var<uniform> blur: BlurParams;

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(occlusion_tex));
    let texel = blur.direction / dims;

    let c0 = textureSample(occlusion_tex, samp, in.uv);
    let c1 = textureSample(occlusion_tex, samp, in.uv + texel).r * 0.15;
    let c2 = textureSample(occlusion_tex, samp, in.uv - texel).r * 0.15;
    let c3 = textureSample(occlusion_tex, samp, in.uv + texel * 2.0).r * 0.15;
    let c4 = textureSample(occlusion_tex, samp, in.uv - texel * 2.0).r * 0.15;
    let blurred = c0.r * 0.4 + c1 + c2 + c3 + c4;
    return vec4<f32>(blurred, blurred, blurred, c0.a);
}
"#;

/// Lighting/composition fragment shader
pub const LIGHTING_FRAGMENT: &str = r#"
const PI: f32 = 3.141592653589793;

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
    hdr_visible: f32,
    _pad: vec3<f32>,
}
@group(0) @binding(5) var<uniform> params: LightingParams;

struct EnvParams {
    intensity: f32,
    rotation: f32,
    enabled: f32,
    _pad: f32,
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

@group(0) @binding(6) var env_map: texture_2d<f32>;
@group(0) @binding(7) var env_sampler: sampler;
@group(0) @binding(8) var<uniform> env: EnvParams;
@group(0) @binding(9) var<uniform> camera: Camera;
@group(0) @binding(10) var depth_tex: texture_depth_2d;

fn dir_to_equirect_uv(dir: vec3<f32>, rotation: f32) -> vec2<f32> {
    let d = normalize(dir);
    let phi = atan2(-d.z, d.x) - rotation;
    let theta = acos(clamp(d.y, -1.0, 1.0));
    let u = (phi + PI) / (2.0 * PI);
    let v = theta / PI;
    return vec2<f32>(u, v);
}

fn sample_background(ndc_xy: vec2<f32>) -> vec4<f32> {
    if env.enabled < 0.5 || env.intensity <= 0.0 || params.hdr_visible < 0.5 {
        return params.background;
    }
    let ndc = vec4<f32>(ndc_xy, 1.0, 1.0);
    let world = camera.inv_view_proj * ndc;
    let world_pos = world.xyz / world.w;
    let dir = normalize(world_pos - camera.position);
    let env_uv = dir_to_equirect_uv(dir, env.rotation);
    let color = textureSample(env_map, env_sampler, env_uv).rgb * env.intensity;
    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_lighting(in: VsOut) -> @location(0) vec4<f32> {
    let mask = textureSample(gbuffer_mask, samp, in.uv);
    let depth = textureSample(depth_tex, samp, in.uv);
    
    if depth >= 0.999 || mask.a < 0.5 {
        return sample_background(in.ndc);
    }

    let occlusion = mask.r;
    let albedo_rgba = textureSample(gbuffer_albedo, samp, in.uv);
    let normal_rgba = textureSample(gbuffer_normals, samp, in.uv);

    let albedo = albedo_rgba.rgb;
    let roughness = clamp(albedo_rgba.a, 0.02, 1.0);
    let metalness = clamp(normal_rgba.a, 0.0, 1.0);
    let n = normalize(normal_rgba.rgb * 2.0 - vec3<f32>(1.0));

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

    let spec_exp = mix(8.0, 128.0, 1.0 - roughness);
    let spec_color = mix(vec3<f32>(0.04), albedo, metalness);
    let half_key = normalize(key_l + view_dir);
    let spec = pow(max(dot(n, half_key), 0.0), spec_exp) * spec_color * key_ndotl;

    return vec4<f32>((diffuse + spec) * occlusion, 1.0);
}
"#;

// Concatenated shaders for pipeline creation
use std::sync::LazyLock;

pub static SSAO_SHADER: LazyLock<String> = LazyLock::new(|| {
    format!("{}{}", FULLSCREEN_VERTEX, SSAO_FRAGMENT)
});

pub static SSAO_BLUR_SHADER: LazyLock<String> = LazyLock::new(|| {
    format!("{}{}", FULLSCREEN_VERTEX, SSAO_BLUR_FRAGMENT)
});

pub static LIGHTING_SHADER: LazyLock<String> = LazyLock::new(|| {
    format!("{}{}", FULLSCREEN_VERTEX, LIGHTING_FRAGMENT)
});
