//! Compute pipeline for BVH-based path tracing.
//!
//! Creates and manages the wgpu compute pipeline, storage buffers for BVH/triangles,
//! accumulation texture, and dispatches the path tracing kernel.
//!
//! ## Usage
//! ```ignore
//! let pt = PathTraceCompute::new(&device, width, height);
//! pt.upload_scene(&device, &queue, &gpu_data);
//! pt.update_camera(&queue, &camera_uniform);
//! pt.dispatch(&device, &queue); // writes to accumulation texture
//! // blit pt.output_view() to screen
//! ```

use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

use super::gpu_data::GpuSceneData;

/// WGSL source embedded at compile time.
const BVH_TRAVERSE_WGSL: &str = include_str!("bvh_traverse.wgsl");
const BLIT_WGSL: &str = include_str!("blit.wgsl");

/// Camera uniform matching the WGSL Camera struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PtCameraUniform {
    /// Inverse view matrix (world from view).
    pub inv_view: [[f32; 4]; 4],
    /// Inverse projection matrix (view from clip).
    pub inv_proj: [[f32; 4]; 4],
    /// Camera world position.
    pub position: [f32; 3],
    /// Frame count for progressive accumulation.
    pub frame_count: u32,
}

/// Workgroup size (must match @workgroup_size in WGSL).
const WG_SIZE: u32 = 8;

/// Path trace compute pipeline state.
pub struct PathTraceCompute {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,

    // Storage buffers (uploaded from GpuSceneData)
    nodes_buffer: Option<wgpu::Buffer>,
    triangles_buffer: Option<wgpu::Buffer>,

    // Camera uniform
    camera_buffer: wgpu::Buffer,

    // Output texture (rgba32float storage)
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,

    // Dimensions
    width: u32,
    height: u32,

    // Progressive frame counter
    pub frame_count: u32,

    // Whether scene data has been uploaded
    scene_ready: bool,

    // Blit pipeline (renders PT output to screen with tone mapping)
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_bind_group: Option<wgpu::BindGroup>,
    blit_sampler: wgpu::Sampler,
}

impl PathTraceCompute {
    /// Create a new path trace compute pipeline.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bvh_traverse_shader"),
            source: wgpu::ShaderSource::Wgsl(BVH_TRAVERSE_WGSL.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pt_bind_group_layout"),
            entries: &[
                // @binding(0) BVH nodes storage
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // @binding(1) Triangles storage
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // @binding(2) Camera uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // @binding(3) Output storage texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pt_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pt_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Camera uniform buffer
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pt_camera_buffer"),
            size: std::mem::size_of::<PtCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Output storage texture
        let (output_texture, output_view) = Self::create_output(device, width, height);

        // Blit pipeline (tone map PT output → screen)
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pt_blit_shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });

        let blit_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pt_blit_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pt_blit_pl"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pt_blit_pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pt_blit_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Pre-build blit bind group
        let blit_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pt_blit_bg"),
            layout: &blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        }));

        Self {
            pipeline,
            bind_group_layout,
            bind_group: None,
            nodes_buffer: None,
            triangles_buffer: None,
            camera_buffer,
            output_texture,
            output_view,
            width,
            height,
            frame_count: 0,
            scene_ready: false,
            blit_pipeline,
            blit_bind_group_layout,
            blit_bind_group,
            blit_sampler,
        }
    }

    /// Create output storage texture.
    fn create_output(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pt_output"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    /// Resize output texture if dimensions changed.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        let (tex, view) = Self::create_output(device, width, height);
        self.output_texture = tex;
        self.output_view = view;
        self.frame_count = 0; // reset accumulation
        self.rebuild_bind_group(device);
    }

    /// Upload scene data (BVH nodes + triangles) to GPU.
    pub fn upload_scene(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, data: &GpuSceneData) {
        let nodes_bytes = data.nodes_bytes();
        let tris_bytes = data.triangles_bytes();

        // Ensure non-empty buffers (wgpu requires >0 size)
        let nodes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pt_nodes"),
            contents: if nodes_bytes.is_empty() { &[0u8; 32] } else { nodes_bytes },
            usage: wgpu::BufferUsages::STORAGE,
        });

        let tris_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pt_triangles"),
            contents: if tris_bytes.is_empty() { &[0u8; 96] } else { tris_bytes },
            usage: wgpu::BufferUsages::STORAGE,
        });

        self.nodes_buffer = Some(nodes_buffer);
        self.triangles_buffer = Some(tris_buffer);
        self.scene_ready = true;
        self.frame_count = 0;
        self.rebuild_bind_group(device);
    }

    /// Rebuild bind groups after buffer/texture change.
    fn rebuild_bind_group(&mut self, device: &wgpu::Device) {
        let (Some(nodes), Some(tris)) = (&self.nodes_buffer, &self.triangles_buffer) else {
            self.bind_group = None;
            return;
        };

        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pt_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: nodes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: tris.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.output_view),
                },
            ],
        }));

        // Rebuild blit bind group (references output_view)
        self.blit_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pt_blit_bg"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        }));
    }

    /// Update camera uniform.
    pub fn update_camera(&mut self, queue: &wgpu::Queue, uniform: &PtCameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(uniform));
    }

    /// Reset progressive accumulation (call on camera move / scene change).
    pub fn reset_accumulation(&mut self) {
        self.frame_count = 0;
    }

    /// Dispatch the compute shader. Returns false if scene not ready.
    pub fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder) -> bool {
        let Some(bg) = &self.bind_group else { return false; };
        if !self.scene_ready { return false; }

        self.frame_count += 1;

        let wg_x = (self.width + WG_SIZE - 1) / WG_SIZE;
        let wg_y = (self.height + WG_SIZE - 1) / WG_SIZE;

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("pt_compute_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);

        true
    }

    /// Get the output texture view (for blitting to screen).
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.output_view
    }

    /// Whether scene data is uploaded and ready for tracing.
    pub fn is_ready(&self) -> bool {
        self.scene_ready && self.bind_group.is_some()
    }

    /// Current output dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Blit the path tracer output to a render target with tone mapping.
    /// Call after dispatch() to display the result.
    pub fn blit(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let Some(bg) = &self.blit_bind_group else { return; };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pt_blit_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.blit_pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.draw(0..3, 0..1); // fullscreen triangle
    }
}
