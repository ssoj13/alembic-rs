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

pub use params::{CameraUniform, LightUniform, ModelUniform, StandardSurfaceParams};

/// Embedded shader source
pub const SHADER_SOURCE: &str = include_str!("shaders/standard_surface.wgsl");

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
                    min_binding_size: None,
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
                    min_binding_size: None,
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

    BindGroupLayouts {
        camera_light,
        material,
        model,
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
}

/// Pipeline configuration
#[derive(Clone, Debug)]
pub struct PipelineConfig {
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
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            depth_format: Some(wgpu::TextureFormat::Depth32Float),
            blend: false,
            cull_mode: Some(wgpu::Face::Back),
            wireframe: false,
        }
    }
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
        bind_group_layouts: &[&layouts.camera_light, &layouts.material, &layouts.model],
        push_constant_ranges: &[],
    });

    let fragment_entry = if config.wireframe {
        "fs_wireframe"
    } else {
        "fs_main"
    };

    let blend_state = if config.blend {
        Some(wgpu::BlendState::ALPHA_BLENDING)
    } else {
        Some(wgpu::BlendState::REPLACE)
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("standard_surface_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[vertex_buffer_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: config.cull_mode,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: config.depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: blend_state,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
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
        // 2 mat4 + vec3 + pad = 128 + 16 = 144 bytes
        assert_eq!(std::mem::size_of::<CameraUniform>(), 144);
    }

    #[test]
    fn test_light_uniform_size() {
        // vec3 + pad + vec3 + f32 = 32 bytes
        assert_eq!(std::mem::size_of::<LightUniform>(), 32);
    }

    #[test]
    fn test_model_uniform_size() {
        // 2 mat4 = 128 bytes
        assert_eq!(std::mem::size_of::<ModelUniform>(), 128);
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
        let l = LightUniform::default();
        assert_eq!(l.intensity, 1.0);
        assert_eq!(l.color, Vec3::ONE);
        // Direction should be normalized
        let len = l.direction.length();
        assert!((len - 1.0).abs() < 0.001);
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
