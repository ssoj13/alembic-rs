//! Autodesk Standard Surface shader for wgpu
//!
//! WGSL port of MaterialX implementation.
//!
//! ## References
//! - [Autodesk Standard Surface](https://autodesk.github.io/standard-surface/)
//! - [MaterialX](https://github.com/AcademySoftwareFoundation/MaterialX)
//!
//! ## Usage
//!
//! ```ignore
//! use standard_surface::{StandardSurfaceParams, create_pipeline};
//!
//! // Create material
//! let material = StandardSurfaceParams::metal(
//!     Vec3::new(1.0, 0.8, 0.3), // gold color
//!     0.3,                       // roughness
//! );
//!
//! // Create render pipeline
//! let pipeline = create_pipeline(&device, surface_format);
//! ```

mod params;

pub use params::{CameraUniform, Light, LightRig, LightUniform, ModelUniform, ShadowUniform, StandardSurfaceParams};

/// Embedded shader source
pub const SHADER_SOURCE: &str = include_str!("shaders/standard_surface.wgsl");

/// Skybox shader source (sky sphere with equirectangular mapping)
pub const SKYBOX_SHADER_SOURCE: &str = r#"
// Skybox - inverted sphere with equirectangular HDR texture

const PI: f32 = 3.141592653589793;

struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    position: vec3<f32>,
    _pad: f32,
}

struct EnvParams {
    intensity: f32,
    rotation: f32,
    enabled: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var env_map: texture_2d<f32>;
@group(1) @binding(1) var env_sampler: sampler;
@group(1) @binding(2) var<uniform> env: EnvParams;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_dir: vec3<f32>,
}

@vertex
fn vs_skybox(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Center sphere at camera position (sky follows camera)
    let world_pos = in.position + camera.position;
    out.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    // Use vertex position as direction (sphere centered at origin)
    out.world_dir = in.position;
    return out;
}

fn dir_to_equirect_uv(dir: vec3<f32>, rotation: f32) -> vec2<f32> {
    let d = normalize(dir);
    let phi = atan2(d.z, d.x) + rotation;
    let theta = acos(clamp(d.y, -1.0, 1.0));
    let u = (phi + PI) / (2.0 * PI);
    let v = theta / PI;
    return vec2<f32>(u, v);
}

@fragment
fn fs_skybox(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_dir);
    let uv = dir_to_equirect_uv(dir, env.rotation);
    let color = textureSample(env_map, env_sampler, uv).rgb * env.intensity;
    return vec4<f32>(color, 1.0);
}
"#;

/// Shadow depth pass shader source
pub const SHADOW_SHADER_SOURCE: &str = r#"
// Shadow depth pass - vertex shader only

struct ShadowUniform {
    light_view_proj: mat4x4<f32>,
}

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> shadow: ShadowUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_shadow(in: VertexInput) -> @builtin(position) vec4<f32> {
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    return shadow.light_view_proj * world_pos;
}
"#;

/// Shader library modules (for advanced users who want to compose shaders)
pub mod shader_lib {
    pub const COMMON: &str = include_str!("shaders/lib/common.wgsl");
    pub const FRESNEL: &str = include_str!("shaders/lib/fresnel.wgsl");
    pub const MICROFACET: &str = include_str!("shaders/lib/microfacet.wgsl");
    pub const DIFFUSE: &str = include_str!("shaders/lib/diffuse.wgsl");
}

/// Vertex buffer layout for standard mesh
pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            // position
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            // normal
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            // uv
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
        ],
    }
}

/// Standard vertex format
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Create bind group layouts for the standard surface pipeline
pub fn create_bind_group_layouts(device: &wgpu::Device) -> BindGroupLayouts {
    let camera_uniform_size = std::num::NonZeroU64::new(std::mem::size_of::<CameraUniform>() as u64);
    let light_uniform_size = std::num::NonZeroU64::new(std::mem::size_of::<LightRig>() as u64);

    // Group 0: Camera + Light
    let camera_light = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("standard_surface_camera_light"),
        entries: &[
            // Camera
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: camera_uniform_size,
                },
                count: None,
            },
            // Light
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: light_uniform_size,
                },
                count: None,
            },
        ],
    });

    // Group 1: Material
    let material = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("standard_surface_material"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // Group 2: Model transform
    let model = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("standard_surface_model"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // Group 3: Shadow map
    let shadow = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("standard_surface_shadow"),
        entries: &[
            // Shadow depth texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Comparison sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
            // Light view-projection matrix
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Shadow pass: just the light view-proj uniform
    let shadow_pass = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow_pass_uniform"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // Group 4: Environment map
    let environment = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("standard_surface_environment"),
        entries: &[
            // Environment texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Environment params
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    BindGroupLayouts {
        camera_light,
        material,
        model,
        shadow,
        shadow_pass,
        environment,
    }
}

/// Bind group layouts for Standard Surface
pub struct BindGroupLayouts {
    /// Group 0: Camera + Light uniforms
    pub camera_light: wgpu::BindGroupLayout,
    /// Group 1: Material parameters
    pub material: wgpu::BindGroupLayout,
    /// Group 2: Model transform
    pub model: wgpu::BindGroupLayout,
    /// Group 3: Shadow map (for main pass)
    pub shadow: wgpu::BindGroupLayout,
    /// Shadow pass: light view-proj uniform only
    pub shadow_pass: wgpu::BindGroupLayout,
    /// Group 4: Environment map
    pub environment: wgpu::BindGroupLayout,
}

/// Pipeline configuration
#[derive(Clone, Debug)]
pub struct PipelineConfig {
    /// Debug label for the pipeline (defaults to "standard_surface_pipeline")
    pub label: Option<&'static str>,
    /// Surface texture format
    pub format: wgpu::TextureFormat,
    /// Depth texture format (None to disable depth)
    pub depth_format: Option<wgpu::TextureFormat>,
    /// Enable alpha blending
    pub blend: bool,
    /// Cull mode
    pub cull_mode: Option<wgpu::Face>,
    /// Use wireframe entry point
    pub wireframe: bool,
    /// Custom fragment entry point (overrides wireframe/default)
    pub fragment_entry: Option<&'static str>,
    /// Primitive topology (TriangleList, LineList, etc)
    pub topology: wgpu::PrimitiveTopology,
    /// Write to depth buffer (disable for transparency)
    pub depth_write: bool,
    /// Depth-only pass (no color output, for depth prepass)
    pub depth_only: bool,
    /// Use LessEqual depth compare (needed after depth prepass)
    pub depth_equal: bool,
    /// Override depth compare function (None = use depth_equal/default)
    pub depth_compare: Option<wgpu::CompareFunction>,
    /// Override color target formats (None = use single config.format)
    pub color_formats: Option<Vec<wgpu::TextureFormat>>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            label: None,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            depth_format: Some(wgpu::TextureFormat::Depth32Float),
            blend: false,
            cull_mode: Some(wgpu::Face::Back),
            wireframe: false,
            fragment_entry: None,
            topology: wgpu::PrimitiveTopology::TriangleList,
            depth_write: true,
            depth_only: false,
            depth_equal: false,
            depth_compare: None,
            color_formats: None,
        }
    }
}

/// Simple vertex for skybox (position only)
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxVertex {
    pub position: [f32; 3],
}

/// Generate inverted sphere mesh for skybox
pub fn generate_sky_sphere(radius: f32, segments: u32, rings: u32) -> (Vec<SkyboxVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Generate vertices
    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32;
        let y = radius * phi.cos();
        let r = radius * phi.sin();
        
        for seg in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let x = r * theta.cos();
            let z = r * theta.sin();
            vertices.push(SkyboxVertex { position: [x, y, z] });
        }
    }
    
    // Generate indices (inverted winding for inside-out rendering)
    for ring in 0..rings {
        for seg in 0..segments {
            let curr = ring * (segments + 1) + seg;
            let next = curr + segments + 1;
            // Inverted winding order (CW instead of CCW)
            indices.push(curr);
            indices.push(curr + 1);
            indices.push(next);
            indices.push(next);
            indices.push(curr + 1);
            indices.push(next + 1);
        }
    }
    
    (vertices, indices)
}

/// Create skybox pipeline (renders HDR environment as background)
pub fn create_skybox_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    env_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("skybox_shader"),
        source: wgpu::ShaderSource::Wgsl(SKYBOX_SHADER_SOURCE.into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("skybox_pipeline_layout"),
        bind_group_layouts: &[camera_layout, env_layout],
        push_constant_ranges: &[],
    });
    
    // Skybox vertex buffer layout (position only)
    let skybox_vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkyboxVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        }],
    };
    
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("skybox_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_skybox"),
            compilation_options: Default::default(),
            buffers: &[skybox_vertex_layout],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // No culling - we're inside the sphere
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false, // Don't write to depth
            depth_compare: wgpu::CompareFunction::LessEqual, // Draw at far plane
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_skybox"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

/// Create camera-only bind group layout for skybox
pub fn create_skybox_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("skybox_camera_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Create shadow depth pipeline (renders from light's perspective)
pub fn create_shadow_pipeline(
    device: &wgpu::Device,
    layouts: &BindGroupLayouts,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shadow_depth_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER_SOURCE.into()),
    });

    // Shadow pass needs: shadow uniform (group 0) + model transform (group 1)
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shadow_pipeline_layout"),
        bind_group_layouts: &[&layouts.shadow_pass, &layouts.model],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_shadow"),
            compilation_options: Default::default(),
            buffers: &[vertex_buffer_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2, // Bias to reduce shadow acne
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None, // No fragment shader - depth only
        multiview: None,
        cache: None,
    })
}

/// Create the standard surface render pipeline
pub fn create_pipeline(
    device: &wgpu::Device,
    layouts: &BindGroupLayouts,
    config: &PipelineConfig,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("standard_surface_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("standard_surface_pipeline_layout"),
        bind_group_layouts: &[
            &layouts.camera_light,
            &layouts.material,
            &layouts.model,
            &layouts.shadow,
            &layouts.environment,
        ],
        push_constant_ranges: &[],
    });

    let fragment_entry = if let Some(entry) = config.fragment_entry {
        entry
    } else if config.wireframe {
        "fs_wireframe"
    } else {
        "fs_main"
    };

    let blend_state = if config.blend {
        Some(wgpu::BlendState::ALPHA_BLENDING)
    } else {
        Some(wgpu::BlendState::REPLACE)
    };

    // Color targets for fragment shader (must outlive the pipeline descriptor)
    let default_targets = [Some(wgpu::ColorTargetState {
        format: config.format,
        blend: blend_state,
        write_mask: wgpu::ColorWrites::ALL,
    })];

    let mut custom_targets: Vec<Option<wgpu::ColorTargetState>> = Vec::new();
    if let Some(formats) = &config.color_formats {
        custom_targets = formats
            .iter()
            .map(|format| {
                Some(wgpu::ColorTargetState {
                    format: *format,
                    blend: blend_state,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .collect();
    }

    let color_targets = if custom_targets.is_empty() {
        &default_targets[..]
    } else {
        &custom_targets[..]
    };

    let fragment_state = if config.depth_only {
        None  // Depth-only pass - no fragment shader
    } else {
        Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: color_targets,
        })
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(config.label.unwrap_or("standard_surface_pipeline")),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[vertex_buffer_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: config.topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: if config.topology == wgpu::PrimitiveTopology::LineList { None } else { config.cull_mode },
            polygon_mode: if config.wireframe { wgpu::PolygonMode::Line } else { wgpu::PolygonMode::Fill },
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: config.depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: config.depth_write,
            depth_compare: config.depth_compare.unwrap_or({
                if config.depth_equal {
                    wgpu::CompareFunction::LessEqual
                } else {
                    wgpu::CompareFunction::Less
                }
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: fragment_state,
        multiview: None,
        cache: None,
    })
}

/// Create a material uniform buffer
pub fn create_material_buffer(
    device: &wgpu::Device,
    params: &StandardSurfaceParams,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("material_buffer"),
        contents: bytemuck::bytes_of(params),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

/// Create a material bind group
pub fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("material_bind_group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    // === Size and alignment tests ===

    #[test]
    fn test_params_size() {
        // 9 Vec4s = 9 * 16 = 144 bytes (must match WGSL struct)
        assert_eq!(std::mem::size_of::<StandardSurfaceParams>(), 144);
    }

    #[test]
    fn test_params_alignment() {
        // Must be 16-byte aligned for GPU
        assert_eq!(std::mem::align_of::<StandardSurfaceParams>(), 16);
    }

    #[test]
    fn test_camera_uniform_size() {
        // 3 mat4 + vec4(position+xray) + vec4(flat+auto+pad) = 192 + 16 + 16 = 224 bytes
        assert_eq!(std::mem::size_of::<CameraUniform>(), 224);
    }

    #[test]
    fn test_light_size() {
        // vec3 + pad + vec3 + f32 = 32 bytes
        assert_eq!(std::mem::size_of::<Light>(), 32);
    }
    
    #[test]
    fn test_light_rig_size() {
        // 3 lights (3 * 32) + ambient vec3 + pad = 96 + 16 = 112 bytes
        assert_eq!(std::mem::size_of::<LightRig>(), 112);
    }

    #[test]
    fn test_model_uniform_size() {
        // 2 mat4 = 128 bytes
        assert_eq!(std::mem::size_of::<ModelUniform>(), 128);
    }

    #[test]
    fn test_shadow_uniform_size() {
        // 1 mat4 = 64 bytes
        assert_eq!(std::mem::size_of::<ShadowUniform>(), 64);
    }

    #[test]
    fn test_vertex_size() {
        // position(12) + normal(12) + uv(8) = 32 bytes
        assert_eq!(std::mem::size_of::<Vertex>(), 32);
    }

    #[test]
    fn test_params_pod() {
        let params = StandardSurfaceParams::default();
        let bytes = bytemuck::bytes_of(&params);
        assert_eq!(bytes.len(), 144);
    }

    // === Default values tests ===

    #[test]
    fn test_default_params() {
        let p = StandardSurfaceParams::default();
        // Base: gray 0.8, weight 1.0
        assert_eq!(p.base_color_weight.w, 1.0);
        assert!((p.base_color_weight.x - 0.8).abs() < 0.001);
        // Specular: white, weight 1.0
        assert_eq!(p.specular_color_weight.w, 1.0);
        // Metalness 0, roughness 0.2, IOR 1.5
        assert_eq!(p.params1.y, 0.0); // metalness
        assert_eq!(p.params1.z, 0.2); // roughness
        assert_eq!(p.params1.w, 1.5); // IOR
        // Opacity fully opaque
        assert_eq!(p.opacity.x, 1.0);
    }

    #[test]
    fn test_default_light() {
        let l = Light::default();
        assert_eq!(l.intensity, 1.0);
        assert_eq!(l.color, Vec3::ONE);
        // Direction should be normalized
        let len = l.direction.length();
        assert!((len - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_light_rig_three_point() {
        let rig = LightRig::three_point();
        // Key light should be brightest
        assert!(rig.key.intensity > rig.fill.intensity);
        // Fill should be non-zero
        assert!(rig.fill.intensity > 0.0);
        // Rim should be non-zero
        assert!(rig.rim.intensity > 0.0);
        // Ambient should be subtle
        assert!(rig.ambient.x < 0.2);
    }
    
    #[test]
    fn test_light_off() {
        let l = Light::off();
        assert_eq!(l.intensity, 0.0);
        assert_eq!(l.color, Vec3::ZERO);
    }

    // === Preset tests ===

    #[test]
    fn test_diffuse_preset() {
        let d = StandardSurfaceParams::diffuse(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(d.base_color_weight.x, 1.0); // red
        assert_eq!(d.base_color_weight.w, 1.0); // base weight
        assert_eq!(d.specular_color_weight.w, 0.0); // no specular
    }

    #[test]
    fn test_metal_preset() {
        let gold = StandardSurfaceParams::metal(Vec3::new(1.0, 0.8, 0.3), 0.3);
        assert_eq!(gold.params1.y, 1.0); // metalness
        assert_eq!(gold.params1.z, 0.3); // roughness
        assert_eq!(gold.base_color_weight.x, 1.0); // gold color
    }

    #[test]
    fn test_plastic_preset() {
        let p = StandardSurfaceParams::plastic(Vec3::new(0.2, 0.5, 0.8), 0.4);
        assert_eq!(p.params1.y, 0.0); // not metal
        assert_eq!(p.params1.z, 0.4); // roughness
    }

    #[test]
    fn test_glass_preset() {
        let g = StandardSurfaceParams::glass(Vec3::ONE, 1.5);
        assert_eq!(g.base_color_weight.w, 0.0); // no base
        assert_eq!(g.transmission_color_weight.w, 1.0); // full transmission
        assert_eq!(g.params1.w, 1.5); // IOR
        assert_eq!(g.params1.z, 0.0); // smooth
    }

    #[test]
    fn test_emissive_preset() {
        let e = StandardSurfaceParams::emissive(Vec3::new(1.0, 0.5, 0.0), 10.0);
        assert_eq!(e.emission_color_weight.w, 10.0); // intensity
        assert_eq!(e.base_color_weight.w, 0.0); // no base
        assert_eq!(e.specular_color_weight.w, 0.0); // no specular
    }

    // === Builder pattern tests ===

    #[test]
    fn test_with_coat() {
        let p = StandardSurfaceParams::default().with_coat(0.5, 0.1);
        assert_eq!(p.coat_color_weight.w, 0.5);
        assert_eq!(p.params2.y, 0.1); // coat roughness
    }

    #[test]
    fn test_with_opacity() {
        let p = StandardSurfaceParams::default().with_opacity(0.5);
        assert_eq!(p.opacity.x, 0.5);
        assert_eq!(p.opacity.y, 0.5);
        assert_eq!(p.opacity.z, 0.5);
    }

    // === Setter tests ===

    #[test]
    fn test_setters() {
        let mut p = StandardSurfaceParams::default();
        p.set_base_color(Vec3::new(1.0, 0.0, 0.0));
        p.set_base(0.8);
        p.set_metalness(0.5);
        p.set_roughness(0.3);
        p.set_specular(0.7);

        assert_eq!(p.base_color_weight.x, 1.0);
        assert_eq!(p.base_color_weight.w, 0.8);
        assert_eq!(p.params1.y, 0.5); // metalness
        assert_eq!(p.params1.z, 0.3); // roughness
        assert_eq!(p.specular_color_weight.w, 0.7);
    }

    // === Edge cases ===

    #[test]
    fn test_zero_values() {
        let mut p = StandardSurfaceParams::default();
        p.set_base(0.0);
        p.set_metalness(0.0);
        p.set_roughness(0.0);
        p.set_specular(0.0);
        // Should not panic, values should be exactly 0
        assert_eq!(p.base_color_weight.w, 0.0);
    }

    #[test]
    fn test_max_values() {
        let mut p = StandardSurfaceParams::default();
        p.set_metalness(1.0);
        p.set_roughness(1.0);
        assert_eq!(p.params1.y, 1.0);
        assert_eq!(p.params1.z, 1.0);
    }

    #[test]
    fn test_negative_color() {
        // Some HDR workflows use negative values
        let p = StandardSurfaceParams::diffuse(Vec3::new(-0.5, 0.0, 0.0));
        assert_eq!(p.base_color_weight.x, -0.5);
    }

    #[test]
    fn test_hdr_color() {
        // HDR values > 1.0
        let e = StandardSurfaceParams::emissive(Vec3::splat(100.0), 1.0);
        assert_eq!(e.emission_color_weight.x, 100.0);
    }

    // === Shader source tests ===

    #[test]
    fn test_shader_entry_points() {
        assert!(SHADER_SOURCE.contains("@vertex"));
        assert!(SHADER_SOURCE.contains("fn vs_main"));
        assert!(SHADER_SOURCE.contains("@fragment"));
        assert!(SHADER_SOURCE.contains("fn fs_main"));
        assert!(SHADER_SOURCE.contains("fn fs_wireframe"));
    }

    #[test]
    fn test_shader_bindings() {
        // Verify binding groups exist
        assert!(SHADER_SOURCE.contains("@group(0) @binding(0)")); // camera
        assert!(SHADER_SOURCE.contains("@group(0) @binding(1)")); // light
        assert!(SHADER_SOURCE.contains("@group(1) @binding(0)")); // material
        assert!(SHADER_SOURCE.contains("@group(2) @binding(0)")); // model
    }

    #[test]
    fn test_shader_material_struct() {
        // Ensure material struct uses vec4 packing
        assert!(SHADER_SOURCE.contains("base_color_weight: vec4<f32>"));
        assert!(SHADER_SOURCE.contains("params1: vec4<f32>"));
        assert!(SHADER_SOURCE.contains("params2: vec4<f32>"));
    }
}
