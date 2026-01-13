//! wgpu renderer with Standard Surface shader

use glam::{Mat4, Vec3};
use std::sync::Arc;
use wgpu::util::DeviceExt;

use standard_surface::{
    BindGroupLayouts, CameraUniform, LightRig, ModelUniform, PipelineConfig,
    ShadowUniform, StandardSurfaceParams, Vertex, SkyboxVertex,
};

use crate::environment::{self, EnvironmentMap, EnvUniform};

/// Shadow map resolution
const SHADOW_MAP_SIZE: u32 = 2048;

/// Main renderer state
pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    
    // Pipelines
    pipeline: wgpu::RenderPipeline,
    pipeline_double_sided: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline_double_sided: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    skybox_pipeline: wgpu::RenderPipeline,
    layouts: BindGroupLayouts,
    
    // Skybox
    skybox_camera_layout: wgpu::BindGroupLayout,
    skybox_camera_bind_group: wgpu::BindGroup,
    skybox_vertex_buffer: wgpu::Buffer,
    skybox_index_buffer: wgpu::Buffer,
    skybox_index_count: u32,
    
    // Uniforms
    camera_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    camera_light_bind_group: wgpu::BindGroup,
    
    // Depth buffer
    depth_texture: Option<DepthTexture>,
    
    // Shadow mapping
    #[allow(dead_code)]
    shadow_texture: wgpu::Texture,
    #[allow(dead_code)]
    shadow_view: wgpu::TextureView,
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    shadow_uniform_buffer: wgpu::Buffer,
    shadow_bind_group: wgpu::BindGroup,
    shadow_pass_bind_group: wgpu::BindGroup,
    
    // Grid
    grid_mesh: Option<Mesh>,
    grid_material: wgpu::BindGroup,
    grid_model: wgpu::BindGroup,
    grid_model_buffer: wgpu::Buffer,
    
    // Environment map
    env_map: EnvironmentMap,
    env_uniform_buffer: wgpu::Buffer,
    
    // Scene meshes
    pub meshes: Vec<SceneMesh>,
    
    // Settings
    pub show_wireframe: bool,
    pub show_grid: bool,
    pub double_sided: bool,
    pub flip_normals: bool,
    pub background_color: [f32; 4],
}

struct DepthTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

/// GPU mesh data
pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// Scene mesh with transform and material
pub struct SceneMesh {
    pub mesh: Mesh,
    pub material_bind_group: wgpu::BindGroup,
    pub model_bind_group: wgpu::BindGroup,
    pub model_buffer: wgpu::Buffer,
    pub transform: Mat4,
    pub name: String,
}

impl Renderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        format: wgpu::TextureFormat,
    ) -> Self {
        // Create bind group layouts
        let layouts = standard_surface::create_bind_group_layouts(&device);

        // Create pipelines
        let config = PipelineConfig {
            format,
            depth_format: Some(wgpu::TextureFormat::Depth32Float),
            blend: false,
            cull_mode: Some(wgpu::Face::Back),
            wireframe: false,
        };
        let pipeline = standard_surface::create_pipeline(&device, &layouts, &config);

        let wireframe_config = PipelineConfig {
            wireframe: true,
            ..config.clone()
        };
        let wireframe_pipeline = standard_surface::create_pipeline(&device, &layouts, &wireframe_config);
        
        // Double-sided pipelines (no backface culling)
        let double_sided_config = PipelineConfig {
            cull_mode: None,
            ..config
        };
        let pipeline_double_sided = standard_surface::create_pipeline(&device, &layouts, &double_sided_config);
        
        let wireframe_double_sided_config = PipelineConfig {
            wireframe: true,
            cull_mode: None,
            ..double_sided_config
        };
        let wireframe_pipeline_double_sided = standard_surface::create_pipeline(&device, &layouts, &wireframe_double_sided_config);
        
        // Shadow depth pipeline
        let shadow_pipeline = standard_surface::create_shadow_pipeline(&device, &layouts);
        
        // Skybox pipeline and resources
        let skybox_camera_layout = standard_surface::create_skybox_camera_layout(&device);
        let skybox_pipeline = standard_surface::create_skybox_pipeline(
            &device,
            &skybox_camera_layout,
            &layouts.environment,
            format,
        );
        
        // Generate sky sphere mesh
        let (sky_verts, sky_indices) = standard_surface::generate_sky_sphere(100.0, 32, 16);
        let skybox_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("skybox_vertex_buffer"),
            contents: bytemuck::cast_slice(&sky_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let skybox_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("skybox_index_buffer"),
            contents: bytemuck::cast_slice(&sky_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let skybox_index_count = sky_indices.len() as u32;

        // Camera uniform
        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            view: Mat4::IDENTITY.to_cols_array_2d(),
            position: Vec3::new(0.0, 0.0, 5.0),
            _pad: 0.0,
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Skybox camera bind group (uses same camera buffer)
        let skybox_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skybox_camera_bind_group"),
            layout: &skybox_camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Light rig (3-point lighting)
        let light_rig = LightRig::three_point();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light_rig_buffer"),
            contents: bytemuck::bytes_of(&light_rig),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Camera + Light bind group
        let camera_light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_light_bind_group"),
            layout: &layouts.camera_light,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        // Shadow map resources
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_map_texture"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        
        // Shadow uniform (light view-proj matrix)
        let shadow_uniform = ShadowUniform::default();
        let shadow_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_uniform_buffer"),
            contents: bytemuck::bytes_of(&shadow_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Shadow bind group (for main pass - samples shadow map)
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_bind_group"),
            layout: &layouts.shadow,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: shadow_uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Shadow pass bind group (for shadow depth pass - group 0)
        let shadow_pass_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_pass_bind_group"),
            layout: &layouts.shadow_pass,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_uniform_buffer.as_entire_binding(),
            }],
        });

        // Grid material (gray diffuse)
        let mut grid_params = StandardSurfaceParams::default();
        grid_params.set_base_color(Vec3::splat(0.3));
        grid_params.set_specular(0.1);
        let grid_material_buffer = standard_surface::create_material_buffer(&device, &grid_params);
        let grid_material = standard_surface::create_material_bind_group(
            &device,
            &layouts.material,
            &grid_material_buffer,
        );

        // Grid model transform
        let grid_model_uniform = ModelUniform::default();
        let grid_model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_model_buffer"),
            contents: bytemuck::bytes_of(&grid_model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let grid_model = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid_model_bind_group"),
            layout: &layouts.model,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: grid_model_buffer.as_entire_binding(),
            }],
        });

        // Default environment map (disabled)
        let env_map = environment::create_default_env(&device, &queue, &layouts.environment);
        let env_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("env_uniform_buffer"),
            contents: bytemuck::bytes_of(&EnvUniform::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            device,
            queue,
            pipeline,
            pipeline_double_sided,
            wireframe_pipeline,
            wireframe_pipeline_double_sided,
            shadow_pipeline,
            skybox_pipeline,
            layouts,
            skybox_camera_layout,
            skybox_camera_bind_group,
            skybox_vertex_buffer,
            skybox_index_buffer,
            skybox_index_count,
            camera_buffer,
            light_buffer,
            camera_light_bind_group,
            depth_texture: None,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            shadow_uniform_buffer,
            shadow_bind_group,
            shadow_pass_bind_group,
            grid_mesh: None,
            grid_material,
            grid_model,
            grid_model_buffer,
            env_map,
            env_uniform_buffer,
            meshes: Vec::new(),
            show_wireframe: false,
            show_grid: true,
            double_sided: false,
            flip_normals: false,
            background_color: [0.1, 0.1, 0.12, 1.0],
        }
    }

    /// Update camera uniform
    pub fn update_camera(&self, view_proj: Mat4, view: Mat4, position: Vec3) {
        let uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            position,
            _pad: 0.0,
        };
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Update shadow map for key light
    /// Creates orthographic projection from light's perspective
    pub fn update_shadow(&self, light_dir: Vec3, scene_center: Vec3, scene_radius: f32) {
        // Light position far from scene, looking at center
        let light_pos = scene_center - light_dir.normalize() * scene_radius * 3.0;
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        
        let light_view = Mat4::look_at_rh(light_pos, scene_center, up);
        // Orthographic projection covering scene
        let half_size = scene_radius * 1.5;
        let light_proj = Mat4::orthographic_rh(
            -half_size, half_size,
            -half_size, half_size,
            0.1, scene_radius * 6.0,
        );
        
        let light_view_proj = light_proj * light_view;
        
        let uniform = ShadowUniform {
            light_view_proj: light_view_proj.to_cols_array_2d(),
        };
        self.queue.write_buffer(&self.shadow_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Ensure depth buffer matches viewport size
    fn ensure_depth_texture(&mut self, width: u32, height: u32) {
        let needs_recreate = match &self.depth_texture {
            Some(dt) => dt.size != (width, height),
            None => true,
        };

        if needs_recreate && width > 0 && height > 0 {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_texture = Some(DepthTexture {
                texture,
                view,
                size: (width, height),
            });
        }
    }

    /// Create grid mesh
    fn ensure_grid(&mut self) {
        if self.grid_mesh.is_some() {
            return;
        }

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let size = 10.0;
        let divisions = 20;
        let step = size * 2.0 / divisions as f32;

        // Create grid lines as thin quads
        let line_width = 0.01;

        for i in 0..=divisions {
            let pos = -size + i as f32 * step;

            // X-axis lines
            let idx = vertices.len() as u32;
            vertices.extend_from_slice(&[
                Vertex { position: [-size, 0.0, pos - line_width], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
                Vertex { position: [size, 0.0, pos - line_width], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
                Vertex { position: [size, 0.0, pos + line_width], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
                Vertex { position: [-size, 0.0, pos + line_width], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
            ]);
            indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

            // Z-axis lines
            let idx = vertices.len() as u32;
            vertices.extend_from_slice(&[
                Vertex { position: [pos - line_width, 0.0, -size], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
                Vertex { position: [pos + line_width, 0.0, -size], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
                Vertex { position: [pos + line_width, 0.0, size], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
                Vertex { position: [pos - line_width, 0.0, size], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
            ]);
            indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
        }

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.grid_mesh = Some(Mesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        });
    }

    /// Create a mesh from vertices and indices
    pub fn create_mesh(&self, vertices: &[Vertex], indices: &[u32]) -> Mesh {
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    /// Add a mesh to the scene
    pub fn add_mesh(
        &mut self,
        name: String,
        vertices: &[Vertex],
        indices: &[u32],
        transform: Mat4,
        params: &StandardSurfaceParams,
    ) {
        let mesh = self.create_mesh(vertices, indices);

        // Material
        let material_buffer = standard_surface::create_material_buffer(&self.device, params);
        let material_bind_group = standard_surface::create_material_bind_group(
            &self.device,
            &self.layouts.material,
            &material_buffer,
        );

        // Model transform
        let normal_matrix = transform.inverse().transpose();
        let model_uniform = ModelUniform {
            model: transform.to_cols_array_2d(),
            normal_matrix: normal_matrix.to_cols_array_2d(),
        };
        let model_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model_buffer"),
            contents: bytemuck::bytes_of(&model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_bind_group"),
            layout: &self.layouts.model,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });

        self.meshes.push(SceneMesh {
            mesh,
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform,
            name,
        });
    }

    /// Clear all scene meshes
    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
    }

    /// Load HDR/EXR environment map
    pub fn load_environment(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        self.env_map = environment::load_env_map(
            &self.device,
            &self.queue,
            &self.layouts.environment,
            path,
        )?;
        Ok(())
    }

    /// Clear environment map (use default flat ambient)
    pub fn clear_environment(&mut self) {
        self.env_map = environment::create_default_env(
            &self.device,
            &self.queue,
            &self.layouts.environment,
        );
    }

    /// Check if environment map is loaded
    pub fn has_environment(&self) -> bool {
        self.env_map.intensity > 0.0
    }
    
    /// Set environment intensity (exposure)
    pub fn set_env_intensity(&mut self, intensity: f32) {
        self.env_map.intensity = intensity;
        let uniform = EnvUniform {
            intensity,
            rotation: 0.0,
            enabled: if intensity > 0.0 { 1.0 } else { 0.0 },
            _pad: 0.0,
        };
        self.queue.write_buffer(&self.env_map.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Update all mesh normal matrices (call when flip_normals changes)
    pub fn update_normals(&self) {
        let flip_scale = if self.flip_normals { -1.0 } else { 1.0 };
        
        for mesh in &self.meshes {
            let normal_matrix = mesh.transform.inverse().transpose() 
                * Mat4::from_scale(Vec3::splat(flip_scale));
            let model_uniform = ModelUniform {
                model: mesh.transform.to_cols_array_2d(),
                normal_matrix: normal_matrix.to_cols_array_2d(),
            };
            self.queue.write_buffer(&mesh.model_buffer, 0, bytemuck::bytes_of(&model_uniform));
        }
    }

    /// Render the scene
    pub fn render(&mut self, view: &wgpu::TextureView, width: u32, height: u32) {
        self.ensure_depth_texture(width, height);
        self.ensure_grid();

        let depth_view = match &self.depth_texture {
            Some(dt) => &dt.view,
            None => return,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Shadow depth pass - render scene from light's perspective
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_depth_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.shadow_pass_bind_group, &[]);

            // Render scene meshes to shadow map
            for mesh in &self.meshes {
                shadow_pass.set_bind_group(1, &mesh.model_bind_group, &[]);
                shadow_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
            }
        }

        // Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.background_color[0] as f64,
                            g: self.background_color[1] as f64,
                            b: self.background_color[2] as f64,
                            a: self.background_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw skybox if environment is loaded
            if self.has_environment() {
                render_pass.set_pipeline(&self.skybox_pipeline);
                render_pass.set_bind_group(0, &self.skybox_camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.env_map.bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.skybox_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.skybox_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.skybox_index_count, 0, 0..1);
            }

            let pipeline = match (self.show_wireframe, self.double_sided) {
                (false, false) => &self.pipeline,
                (false, true) => &self.pipeline_double_sided,
                (true, false) => &self.wireframe_pipeline,
                (true, true) => &self.wireframe_pipeline_double_sided,
            };

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.camera_light_bind_group, &[]);
            render_pass.set_bind_group(3, &self.shadow_bind_group, &[]);
            render_pass.set_bind_group(4, &self.env_map.bind_group, &[]);

            // Draw grid
            if self.show_grid {
                if let Some(grid) = &self.grid_mesh {
                    render_pass.set_bind_group(1, &self.grid_material, &[]);
                    render_pass.set_bind_group(2, &self.grid_model, &[]);
                    render_pass.set_vertex_buffer(0, grid.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(grid.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..grid.index_count, 0, 0..1);
                }
            }

            // Draw scene meshes
            for mesh in &self.meshes {
                render_pass.set_bind_group(1, &mesh.material_bind_group, &[]);
                render_pass.set_bind_group(2, &mesh.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
