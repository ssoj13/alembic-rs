//! wgpu renderer with Standard Surface shader

use glam::{Mat4, Vec3};
use std::sync::Arc;
use wgpu::util::DeviceExt;

use standard_surface::{
    BindGroupLayouts, CameraUniform, LightUniform, ModelUniform, PipelineConfig,
    StandardSurfaceParams, Vertex,
};

/// Main renderer state
pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    
    // Pipelines
    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    layouts: BindGroupLayouts,
    
    // Uniforms
    camera_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    camera_light_bind_group: wgpu::BindGroup,
    
    // Depth buffer
    depth_texture: Option<DepthTexture>,
    
    // Grid
    grid_mesh: Option<Mesh>,
    grid_material: wgpu::BindGroup,
    grid_model: wgpu::BindGroup,
    grid_model_buffer: wgpu::Buffer,
    
    // Scene meshes
    pub meshes: Vec<SceneMesh>,
    
    // Settings
    pub show_wireframe: bool,
    pub show_grid: bool,
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
            ..config
        };
        let wireframe_pipeline = standard_surface::create_pipeline(&device, &layouts, &wireframe_config);

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

        // Light uniform
        let light_uniform = LightUniform::default();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light_buffer"),
            contents: bytemuck::bytes_of(&light_uniform),
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

        Self {
            device,
            queue,
            pipeline,
            wireframe_pipeline,
            layouts,
            camera_buffer,
            light_buffer,
            camera_light_bind_group,
            depth_texture: None,
            grid_mesh: None,
            grid_material,
            grid_model,
            grid_model_buffer,
            meshes: Vec::new(),
            show_wireframe: false,
            show_grid: true,
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

            let pipeline = if self.show_wireframe {
                &self.wireframe_pipeline
            } else {
                &self.pipeline
            };

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.camera_light_bind_group, &[]);

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
