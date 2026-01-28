//! Scene rendering pipelines (opaque, wireframe, gbuffer, shadow, object ID, hover).

use standard_surface::{BindGroupLayouts, PipelineConfig};

pub struct Pipelines {
    pub wireframe_pipeline: wgpu::RenderPipeline,
    pub wireframe_pipeline_double_sided: wgpu::RenderPipeline,
    pub gbuffer_pipeline: wgpu::RenderPipeline,
    pub gbuffer_pipeline_double_sided: wgpu::RenderPipeline,
    pub transparent_pipeline: wgpu::RenderPipeline,
    pub transparent_pipeline_double_sided: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub point_pipeline: wgpu::RenderPipeline,
    pub shadow_pipeline: wgpu::RenderPipeline,
    pub object_id_pipeline: wgpu::RenderPipeline,
    pub object_id_pipeline_double_sided: wgpu::RenderPipeline,
}

/// Resources for hover highlight post-process
pub struct HoverPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub params_buffer: wgpu::Buffer,
}

/// Hover post-process parameters (must match WGSL struct)
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HoverParams {
    pub hovered_id: u32,
    pub mode: u32,           // 0=none, 1=outline, 2=tint, 3=both
    pub outline_width: f32,
    pub _pad0: f32,
    pub outline_color: [f32; 4],
    pub tint_color: [f32; 4],
    pub viewport_size: [f32; 2],
    pub _pad1: [f32; 2],
}

impl Default for HoverParams {
    fn default() -> Self {
        Self {
            hovered_id: 0,
            mode: 1,  // outline by default
            outline_width: 2.0,
            _pad0: 0.0,
            outline_color: [1.0, 0.5, 0.0, 1.0],  // Orange
            tint_color: [1.0, 0.5, 0.0, 0.15],    // Semi-transparent orange
            viewport_size: [1920.0, 1080.0],
            _pad1: [0.0; 2],
        }
    }
}

pub fn create_pipelines(
    device: &wgpu::Device,
    layouts: &BindGroupLayouts,
    format: wgpu::TextureFormat,
) -> Pipelines {
    let config = PipelineConfig {
        label: Some("opaque_pipeline"),
        format,
        depth_format: Some(wgpu::TextureFormat::Depth32Float),
        blend: false,  // Opaque pass should not blend
        depth_equal: false, // Use standard Less depth compare for G-Buffer + depth writes
        cull_mode: Some(wgpu::Face::Back),
        wireframe: false,
        ..Default::default()
    };
    let wireframe_config = PipelineConfig {
        label: Some("wireframe_pipeline"),
        wireframe: true,
        ..config.clone()
    };
    let wireframe_pipeline = standard_surface::create_pipeline(device, layouts, &wireframe_config);

    let double_sided_config = PipelineConfig {
        label: Some("opaque_pipeline_double_sided"),
        cull_mode: None,
        ..config.clone()
    };
    let wireframe_double_sided_config = PipelineConfig {
        label: Some("wireframe_pipeline_double_sided"),
        wireframe: true,
        cull_mode: None,
        ..double_sided_config.clone()
    };
    let wireframe_pipeline_double_sided = standard_surface::create_pipeline(device, layouts, &wireframe_double_sided_config);

    let gbuffer_formats = vec![
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::R8Unorm,
    ];
    let gbuffer_config = PipelineConfig {
        label: Some("gbuffer_pipeline"),
        blend: false,
        fragment_entry: Some("fs_gbuffer"),
        color_formats: Some(gbuffer_formats.clone()),
        ..config.clone()
    };
    let gbuffer_pipeline = standard_surface::create_pipeline(device, layouts, &gbuffer_config);

    let gbuffer_double_sided_config = PipelineConfig {
        label: Some("gbuffer_pipeline_double_sided"),
        blend: false,
        fragment_entry: Some("fs_gbuffer"),
        color_formats: Some(gbuffer_formats),
        cull_mode: None,
        ..config.clone()
    };
    let gbuffer_pipeline_double_sided =
        standard_surface::create_pipeline(device, layouts, &gbuffer_double_sided_config);

    let transparent_config = PipelineConfig {
        label: Some("transparent_pipeline"),
        blend: true,
        depth_write: false,
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        ..config.clone()
    };
    let transparent_pipeline = standard_surface::create_pipeline(device, layouts, &transparent_config);

    let transparent_double_sided_config = PipelineConfig {
        label: Some("transparent_pipeline_double_sided"),
        blend: true,
        depth_write: false,
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        cull_mode: None,
        ..config.clone()
    };
    let transparent_pipeline_double_sided =
        standard_surface::create_pipeline(device, layouts, &transparent_double_sided_config);

    let line_config = PipelineConfig {
        label: Some("line_pipeline"),
        topology: wgpu::PrimitiveTopology::LineList,
        cull_mode: None,
        ..double_sided_config.clone()
    };
    let line_pipeline = standard_surface::create_pipeline(device, layouts, &line_config);

    let point_config = PipelineConfig {
        label: Some("point_pipeline"),
        topology: wgpu::PrimitiveTopology::PointList,
        cull_mode: None,
        ..double_sided_config
    };
    let point_pipeline = standard_surface::create_pipeline(device, layouts, &point_config);

    let shadow_pipeline = standard_surface::create_shadow_pipeline(device, layouts);

    // Object ID pipeline - outputs u32 mesh ID
    let object_id_pipeline = create_object_id_pipeline(device, layouts, false);
    let object_id_pipeline_double_sided = create_object_id_pipeline(device, layouts, true);

    Pipelines {
        wireframe_pipeline,
        wireframe_pipeline_double_sided,
        gbuffer_pipeline,
        gbuffer_pipeline_double_sided,
        transparent_pipeline,
        transparent_pipeline_double_sided,
        line_pipeline,
        point_pipeline,
        shadow_pipeline,
        object_id_pipeline,
        object_id_pipeline_double_sided,
    }
}

/// Create the object ID render pipeline
fn create_object_id_pipeline(
    device: &wgpu::Device,
    layouts: &BindGroupLayouts,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("shaders/object_id.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("object_id_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // The model layout needs VERTEX | FRAGMENT visibility for object_id access
    // But we can reuse the existing model bind groups by passing object_id through vertex output
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("object_id_pipeline_layout"),
        bind_group_layouts: &[
            &layouts.camera_light,  // group 0: camera
            &layouts.model,         // group 1: model (reuse existing, pass id via vertex)
        ],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if double_sided { "object_id_pipeline_double_sided" } else { "object_id_pipeline" }),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[standard_surface::vertex_buffer_layout()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R32Uint,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: if double_sided { None } else { Some(wgpu::Face::Back) },
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,  // Write depth for proper occlusion in PT mode
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// Create the hover highlight post-process pipeline
pub fn create_hover_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> HoverPipeline {
    let shader_source = include_str!("shaders/outline.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hover_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("hover_bind_group_layout"),
        entries: &[
            // Object ID texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Params uniform
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hover_pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hover_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],  // Fullscreen triangle, no vertex buffer
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,  // No depth for post-process
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hover_params_buffer"),
        size: std::mem::size_of::<HoverParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    HoverPipeline {
        pipeline,
        bind_group_layout,
        params_buffer,
    }
}
