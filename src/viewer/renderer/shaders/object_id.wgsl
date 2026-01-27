// Object ID shader - outputs mesh ID for hover picking
// Renders to R32Uint texture, each pixel contains the object ID
// Note: object_id is passed through vertex output since model uniform is VERTEX-only

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
    object_id: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) object_id: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    out.position = camera.view_proj * world_pos;
    out.object_id = model.object_id;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) u32 {
    return in.object_id;
}
