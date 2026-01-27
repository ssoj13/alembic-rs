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
/// WGSL alignment: vec3<f32> aligns to 16 bytes, so position needs padding.
/// Total size must be 176 bytes to match WGSL struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PtCameraUniform {
    /// Inverse view matrix (world from view). Offset 0, 64 bytes.
    pub inv_view: [[f32; 4]; 4],
    /// Inverse projection matrix (view from clip). Offset 64, 64 bytes.
    pub inv_proj: [[f32; 4]; 4],
    /// Camera world position. Offset 128, 12 bytes.
    pub position: [f32; 3],
    /// Padding after position (vec3 -> vec4 alignment). Offset 140, 4 bytes.
    pub _pad0: u32,
    /// Frame count for progressive accumulation. Offset 144, 4 bytes.
    pub frame_count: u32,
    /// Maximum bounces (1-8). Offset 148, 4 bytes.
    pub max_bounces: u32,
    /// Maximum transmission/glass depth. Offset 152, 4 bytes.
    pub max_transmission_depth: u32,
    /// Padding. Offset 156, 4 bytes.
    pub _pad1: u32,
    /// Final padding (vec3<u32> in WGSL = 16 bytes). Offset 160, 16 bytes.
    pub _pad2: [u32; 4],
    // Total: 176 bytes
}

/// Workgroup size (must match @workgroup_size in WGSL).
const WG_SIZE: u32 = 8;

/// Environment uniform for path tracer.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PtEnvUniform {
    pub intensity: f32,
    pub rotation: f32,
    pub enabled: f32,
    pub _pad: f32,
}

impl Default for PtEnvUniform {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            rotation: 0.0,
            enabled: 0.0,
            _pad: 0.0,
        }
    }
}

/// Path trace compute pipeline state.
pub struct PathTraceCompute {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,

    // Storage buffers (uploaded from GpuSceneData)
    nodes_buffer: Option<wgpu::Buffer>,
    triangles_buffer: Option<wgpu::Buffer>,
    materials_buffer: Option<wgpu::Buffer>,

    // Camera uniform
    camera_buffer: wgpu::Buffer,

    // Output texture (rgba32float storage)
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,

    // Accumulation buffer (vec4<f32> per pixel, read_write storage)
    accum_buffer: wgpu::Buffer,

    // Environment map (shared from renderer)
    #[allow(dead_code)] // Texture kept alive for env_view
    env_texture: wgpu::Texture,
    env_view: wgpu::TextureView,
    env_sampler: wgpu::Sampler,
    env_uniform_buffer: wgpu::Buffer,
    env_dirty: bool,

    // Dimensions
    width: u32,
    height: u32,

    // Progressive frame counter
    pub frame_count: u32,
    /// Stop accumulating after this many samples
    pub max_samples: u32,

    // Camera change detection (skip reset if camera didn't move)
    pub last_camera_pos: Option<[f32; 3]>,
    pub last_view_proj: Option<[[f32; 4]; 4]>,

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
                // @binding(4) Accumulation buffer (read_write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // @binding(5) Materials storage
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // @binding(6) Environment map texture
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(7) Environment sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // @binding(8) Environment params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
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

        // Output storage texture + accumulation buffer
        let (output_texture, output_view) = Self::create_output(device, width, height);
        let accum_buffer = Self::create_accum_buffer(device, width, height);

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

        // Create default 1x1 black environment texture
        let (env_texture, env_view) = Self::create_default_env_texture(device);
        let env_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pt_env_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let env_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pt_env_uniform"),
            contents: bytemuck::bytes_of(&PtEnvUniform::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group: None,
            nodes_buffer: None,
            triangles_buffer: None,
            materials_buffer: None,
            max_samples: 512,
            last_camera_pos: None,
            last_view_proj: None,
            camera_buffer,
            output_texture,
            output_view,
            accum_buffer,
            env_texture,
            env_view,
            env_sampler,
            env_uniform_buffer,
            env_dirty: false,
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

    /// Create accumulation buffer (vec4<f32> per pixel).
    fn create_accum_buffer(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
        let size = (width * height) as u64 * 16; // 4 x f32 per pixel
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pt_accum"),
            size: size.max(16), // min 16 bytes
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
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

    /// Create default 1x1 black environment texture.
    fn create_default_env_texture(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
        use half::f16;
        // 1x1 black pixel in Rgba16Float
        let data: [f16; 4] = [f16::ZERO, f16::ZERO, f16::ZERO, f16::ONE];
        let bytes: &[u8] = bytemuck::cast_slice(&data);

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pt_default_env"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Write data immediately
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pt_env_staging"),
            contents: bytes,
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        // For a 1x1 texture we can use queue.write_texture in set_environment
        // For now just create empty and let set_environment fill it

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
        self.accum_buffer = Self::create_accum_buffer(device, width, height);
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

        let mats_bytes = data.materials_bytes();
        // GpuMaterial is 144 bytes (9 x vec4<f32>)
        let mats_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pt_materials"),
            contents: if mats_bytes.is_empty() { &[0u8; 144] } else { mats_bytes },
            usage: wgpu::BufferUsages::STORAGE,
        });

        self.nodes_buffer = Some(nodes_buffer);
        self.triangles_buffer = Some(tris_buffer);
        self.materials_buffer = Some(mats_buffer);
        self.scene_ready = true;
        self.frame_count = 0;
        self.rebuild_bind_group(device);
    }

    /// Rebuild bind groups after buffer/texture change.
    fn rebuild_bind_group(&mut self, device: &wgpu::Device) {
        let (Some(nodes), Some(tris), Some(mats)) =
            (&self.nodes_buffer, &self.triangles_buffer, &self.materials_buffer) else {
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
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.accum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: mats.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.env_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&self.env_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: self.env_uniform_buffer.as_entire_binding(),
                },
            ],
        }));
        
        self.env_dirty = false;

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

    /// Set environment map from renderer's EnvironmentMap.
    /// Call this when HDR is loaded or changed.
    pub fn set_environment(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        env_view: &wgpu::TextureView,
        env_sampler: &wgpu::Sampler,
        intensity: f32,
        enabled: bool,
    ) {
        // We need to recreate our own view/sampler references or copy the texture
        // For now, we'll create a new bind group referencing the renderer's resources
        // This requires storing references, which is complex. Instead, update uniform.
        
        let uniform = PtEnvUniform {
            intensity,
            rotation: 0.0,
            enabled: if enabled { 1.0 } else { 0.0 },
            _pad: 0.0,
        };
        queue.write_buffer(&self.env_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        
        // Mark that we need to rebuild bind group with new env texture
        // Store the view reference - but we can't store borrowed references
        // So we need a different approach: pass the texture view at bind group rebuild time
        self.env_dirty = true;
        
        // For proper integration, we'd store Arc<TextureView> or rebuild here
        // For now, we'll need the renderer to call a special rebuild method
        let _ = (device, env_view, env_sampler); // suppress warnings
    }

    /// Set environment from renderer's EnvironmentMap (creates new bind group).
    pub fn set_environment_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        intensity: f32,
        enabled: bool,
    ) {
        // Create our own view of the texture
        self.env_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let uniform = PtEnvUniform {
            intensity,
            rotation: 0.0,
            enabled: if enabled { 1.0 } else { 0.0 },
            _pad: 0.0,
        };
        queue.write_buffer(&self.env_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        
        // Rebuild bind group with new env texture
        self.rebuild_bind_group(device);
        self.reset_accumulation();
    }

    /// Update just the environment intensity/enabled state.
    pub fn update_environment_params(&mut self, queue: &wgpu::Queue, intensity: f32, enabled: bool) {
        let uniform = PtEnvUniform {
            intensity,
            rotation: 0.0,
            enabled: if enabled { 1.0 } else { 0.0 },
            _pad: 0.0,
        };
        queue.write_buffer(&self.env_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Reset progressive accumulation (call on camera move / scene change).
    pub fn reset_accumulation(&mut self) {
        tracing::warn!("PT reset_accumulation called! was at frame {}", self.frame_count);
        // Capture backtrace for debugging spurious resets
        #[cfg(debug_assertions)]
        {
            let bt = std::backtrace::Backtrace::force_capture();
            tracing::debug!("PT reset backtrace:\n{}", bt);
        }
        self.frame_count = 0;
    }

    /// Dispatch the compute shader. Returns false if scene not ready.
    ///
    /// Increments frame counter and writes it to camera uniform buffer
    /// so the shader can do progressive accumulation (blend = 1/frame_count).
    #[tracing::instrument(skip_all, fields(frame = self.frame_count))]
    pub fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) -> bool {
        let Some(bg) = &self.bind_group else { return false; };
        if !self.scene_ready { return false; }
        if self.frame_count >= self.max_samples { return true; } // converged

        self.frame_count += 1;

        // Update frame_count in camera uniform (offset of frame_count field)
        // inv_view (64) + inv_proj (64) + position (12) + _pad0 (4) = 144
        let fc_offset = std::mem::size_of::<[[f32; 4]; 4]>() * 2 // inv_view + inv_proj = 128
                      + std::mem::size_of::<[f32; 3]>()          // position = 12
                      + std::mem::size_of::<u32>();              // _pad0 = 4 -> total 144
        queue.write_buffer(
            &self.camera_buffer,
            fc_offset as u64,
            bytemuck::bytes_of(&self.frame_count),
        );

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
