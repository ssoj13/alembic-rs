// Autodesk Standard Surface Shader
// WGSL port of MaterialX implementation
//
// References:
// - https://autodesk.github.io/standard-surface/
// - https://github.com/AcademySoftwareFoundation/MaterialX

// ============================================================================
// Constants
// ============================================================================

const PI: f32 = 3.141592653589793;
const PI_INV: f32 = 0.3183098861837907;
const EPSILON: f32 = 1e-6;

// ============================================================================
// Uniforms
// ============================================================================

struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    position: vec3<f32>,
    _pad: f32,
}

struct Light {
    direction: vec3<f32>,
    _pad1: f32,
    color: vec3<f32>,
    intensity: f32,
}

// 3-point lighting rig (key, fill, rim)
struct LightRig {
    key: Light,
    fill: Light,
    rim: Light,
    ambient: vec3<f32>,
    _pad: f32,
}

// Using vec4 for colors to ensure proper alignment (16-byte)
struct StandardSurfaceParams {
    // Base (vec4: rgb=color, a=weight)
    base_color_weight: vec4<f32>,
    // Specular (vec4: rgb=color, a=weight)
    specular_color_weight: vec4<f32>,
    // Transmission (vec4: rgb=color, a=weight)
    transmission_color_weight: vec4<f32>,
    // Subsurface (vec4: rgb=color, a=weight)
    subsurface_color_weight: vec4<f32>,
    // Coat (vec4: rgb=color, a=weight)
    coat_color_weight: vec4<f32>,
    // Emission (vec4: rgb=color, a=weight)
    emission_color_weight: vec4<f32>,
    // Opacity (vec4: rgb=opacity, a=unused)
    opacity: vec4<f32>,
    // Packed scalars
    // x=diffuse_roughness, y=metalness, z=specular_roughness, w=specular_IOR
    params1: vec4<f32>,
    // x=specular_anisotropy, y=coat_roughness, z=coat_IOR, w=unused
    params2: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> lights: LightRig;
@group(1) @binding(0) var<uniform> material: StandardSurfaceParams;

// ============================================================================
// Vertex Shader
// ============================================================================

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

@group(2) @binding(0) var<uniform> model: ModelUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    out.world_position = world_pos.xyz;
    out.clip_position = camera.view_proj * world_pos;
    out.world_normal = normalize((model.normal_matrix * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv = in.uv;

    return out;
}

// ============================================================================
// Helper Functions
// ============================================================================

fn square(x: f32) -> f32 {
    return x * x;
}

fn pow5(x: f32) -> f32 {
    let x2 = x * x;
    return x2 * x2 * x;
}

// Schlick Fresnel
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (vec3<f32>(1.0) - F0) * pow5(1.0 - cos_theta);
}

// IOR to F0
fn ior_to_f0(ior: f32) -> f32 {
    let r = (ior - 1.0) / (ior + 1.0);
    return r * r;
}

// GGX Normal Distribution
fn distribution_ggx(NdotH: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = NdotH * NdotH * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom);
}

// Smith GGX Geometry (height-correlated)
fn geometry_smith(NdotV: f32, NdotL: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;

    let lambda_v = sqrt(a2 + (1.0 - a2) * NdotV * NdotV);
    let lambda_l = sqrt(a2 + (1.0 - a2) * NdotL * NdotL);

    return 2.0 * NdotV * NdotL / (lambda_v * NdotL + lambda_l * NdotV + EPSILON);
}

// Oren-Nayar Diffuse
fn oren_nayar(NdotV: f32, NdotL: f32, LdotV: f32, roughness: f32) -> f32 {
    let sigma2 = roughness * roughness;
    let A = 1.0 - 0.5 * sigma2 / (sigma2 + 0.33);
    let B = 0.45 * sigma2 / (sigma2 + 0.09);

    let s = LdotV - NdotL * NdotV;
    var t: f32;
    if s > 0.0 {
        t = max(NdotL, NdotV);
    } else {
        t = 1.0;
    }

    return A + B * s / t;
}

// ============================================================================
// Fragment Shader
// ============================================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.world_normal);
    let V = normalize(camera.position - in.world_position);
    let L = normalize(-light.direction);
    let H = normalize(V + L);

    let NdotV = max(dot(N, V), EPSILON);
    let NdotL = max(dot(N, L), 0.0);
    let NdotH = max(dot(N, H), 0.0);
    let VdotH = max(dot(V, H), 0.0);
    let LdotV = max(dot(L, V), 0.0);

    // Unpack material parameters
    let base_color = material.base_color_weight.rgb;
    let base = material.base_color_weight.a;
    let specular_color = material.specular_color_weight.rgb;
    let specular = material.specular_color_weight.a;
    let coat_color = material.coat_color_weight.rgb;
    let coat = material.coat_color_weight.a;
    let emission_color = material.emission_color_weight.rgb;
    let emission = material.emission_color_weight.a;
    
    let diffuse_roughness = material.params1.x;
    let metalness = material.params1.y;
    let specular_roughness = max(material.params1.z, 0.04);
    let specular_IOR = material.params1.w;
    let coat_roughness = max(material.params2.y, 0.04);
    let coat_IOR = material.params2.z;

    // Effective base color
    let effective_base = base_color * base;

    // F0 for dielectrics from IOR, for metals from base_color
    let dielectric_F0 = vec3<f32>(ior_to_f0(specular_IOR));
    let F0 = mix(dielectric_F0 * specular_color, effective_base, metalness);

    // ========================================
    // Diffuse Layer (Oren-Nayar)
    // ========================================
    let diffuse_factor = oren_nayar(NdotV, NdotL, LdotV, diffuse_roughness);
    let diffuse = effective_base * diffuse_factor * PI_INV;

    // ========================================
    // Specular Layer (GGX)
    // ========================================
    let D = distribution_ggx(NdotH, specular_roughness);
    let G = geometry_smith(NdotV, NdotL, specular_roughness);
    let F = fresnel_schlick(VdotH, F0);

    let specular_brdf = (D * G * F) / (4.0 * NdotV * NdotL + EPSILON);

    // ========================================
    // Energy Conservation
    // ========================================
    let kS = F;
    let kD = (vec3<f32>(1.0) - kS) * (1.0 - metalness);

    // ========================================
    // Coat Layer (simplified)
    // ========================================
    var coat_contribution = vec3<f32>(0.0);
    if coat > EPSILON {
        let coat_F0 = vec3<f32>(ior_to_f0(coat_IOR));

        let coat_D = distribution_ggx(NdotH, coat_roughness);
        let coat_G = geometry_smith(NdotV, NdotL, coat_roughness);
        let coat_F = fresnel_schlick(VdotH, coat_F0);

        coat_contribution = coat * coat_color *
                           (coat_D * coat_G * coat_F) / (4.0 * NdotV * NdotL + EPSILON);
    }

    // ========================================
    // Emission
    // ========================================
    let emissive = emission * emission_color;

    // ========================================
    // Combine
    // ========================================
    let radiance = light.color * light.intensity;
    var color = (kD * diffuse + specular_brdf * specular) * radiance * NdotL;
    color += coat_contribution * radiance * NdotL;
    color += emissive;

    // Simple ambient
    let ambient = vec3<f32>(0.03) * effective_base;
    color += ambient;

    // Opacity
    let alpha = (material.opacity.r + material.opacity.g + material.opacity.b) / 3.0;

    return vec4<f32>(color, alpha);
}

// ============================================================================
// Wireframe variant (for debugging)
// ============================================================================

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
