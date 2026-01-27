// Highlight shader for hover effects
// Supports both outline (vertex offset) and tint (additive color) modes

struct CameraUniform {
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

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

struct HighlightParams {
    color: vec4<f32>,        // highlight color (RGBA)
    outline_width: f32,      // vertex offset for outline mode (0 = tint mode)
    _pad: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> highlight: HighlightParams;
@group(2) @binding(0) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Offset position along normal for outline mode
    var pos = in.position;
    if highlight.outline_width > 0.0 {
        pos = pos + in.normal * highlight.outline_width;
    }
    
    let world_pos = model.model * vec4<f32>(pos, 1.0);
    out.world_pos = world_pos.xyz;
    out.position = camera.view_proj * world_pos;
    out.world_normal = normalize((model.normal_matrix * vec4<f32>(in.normal, 0.0)).xyz);
    return out;
}

@fragment
fn fs_tint(in: VertexOutput) -> @location(0) vec4<f32> {
    // Tint mode: fresnel-enhanced highlight
    let view_dir = normalize(camera.position - in.world_pos);
    let n_dot_v = abs(dot(normalize(in.world_normal), view_dir));
    
    // Boost edges (fresnel-like effect)
    let fresnel = pow(1.0 - n_dot_v, 3.0);
    let intensity = fresnel * 0.6 + 0.15;
    
    return vec4<f32>(highlight.color.rgb * intensity, highlight.color.a * intensity);
}

@fragment
fn fs_outline(in: VertexOutput) -> @location(0) vec4<f32> {
    // Outline mode: solid color
    return highlight.color;
}
