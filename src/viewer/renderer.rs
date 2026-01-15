//! wgpu renderer with Standard Surface shader

use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

use standard_surface::{
    BindGroupLayouts, CameraUniform, LightRig, ModelUniform, PipelineConfig,
    ShadowUniform, StandardSurfaceParams, Vertex,
};

use super::environment::{self, EnvironmentMap, EnvUniform};
use super::smooth_normals::SmoothNormalData;

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
    line_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    skybox_pipeline: wgpu::RenderPipeline,
    layouts: BindGroupLayouts,
    
    // Skybox
    #[allow(dead_code)]
    skybox_camera_layout: wgpu::BindGroupLayout,
    skybox_camera_bind_group: wgpu::BindGroup,
    skybox_vertex_buffer: wgpu::Buffer,
    skybox_index_buffer: wgpu::Buffer,
    skybox_index_count: u32,
    
    // Uniforms
    camera_buffer: wgpu::Buffer,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    grid_model_buffer: wgpu::Buffer,
    grid_step: f32, // current grid step for adaptive sizing
    
    // Environment map
    env_map: EnvironmentMap,
    #[allow(dead_code)]
    env_uniform_buffer: wgpu::Buffer,
    
    // Floor mesh (rendered before scene, managed separately)
    floor_mesh: Option<SceneMesh>,
    
    // Scene meshes (name -> mesh for efficient updates)
    pub meshes: HashMap<String, SceneMesh>,
    
    // Scene curves (rendered as lines)
    pub curves: HashMap<String, SceneCurves>,

    // Scene points (rendered as point sprites)
    pub points: HashMap<String, ScenePoints>,

    // Settings
    pub show_wireframe: bool,
    pub flat_shading: bool,
    pub show_grid: bool,
    pub show_shadows: bool,
    pub hdr_visible: bool,
    pub xray_alpha: f32,
    pub double_sided: bool,
    pub auto_normals: bool,
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
    pub vertex_hash: u64,  // Quick hash for change detection
    pub bounds: (Vec3, Vec3),  // (min, max) in world space
    #[allow(dead_code)]
    pub name: String,
    // For dynamic smooth normal recalculation
    pub smooth_data: Option<SmoothNormalData>,
    pub base_vertices: Option<Vec<Vertex>>,  // vertices with flat normals
}

/// Compute a quick hash for vertex change detection
/// Uses vertex count + first/last position for speed
pub fn compute_vertex_hash(vertices: &[standard_surface::Vertex]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    vertices.len().hash(&mut hasher);
    if let Some(first) = vertices.first() {
        // Hash first vertex position as bits
        first.position[0].to_bits().hash(&mut hasher);
        first.position[1].to_bits().hash(&mut hasher);
        first.position[2].to_bits().hash(&mut hasher);
    }
    if let Some(last) = vertices.last() {
        last.position[0].to_bits().hash(&mut hasher);
        last.position[1].to_bits().hash(&mut hasher);
        last.position[2].to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Compute bounding box from vertices in world space
fn compute_mesh_bounds(vertices: &[standard_surface::Vertex], transform: Mat4) -> (Vec3, Vec3) {
    if vertices.is_empty() {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    
    for v in vertices {
        let p = transform.transform_point3(Vec3::from(v.position));
        min = min.min(p);
        max = max.max(p);
    }
    
    (min, max)
}

/// Compute bounding box from point positions
fn compute_points_bounds(positions: &[[f32; 3]], transform: Mat4) -> (Vec3, Vec3) {
    if positions.is_empty() {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    
    for pos in positions {
        let p = transform.transform_point3(Vec3::from(*pos));
        min = min.min(p);
        max = max.max(p);
    }
    
    (min, max)
}

/// Scene curves (lines) with transform and material
pub struct SceneCurves {
    pub mesh: Mesh, // vertex + index buffers for LINE_LIST
    pub material_bind_group: wgpu::BindGroup,
    pub model_bind_group: wgpu::BindGroup,
    #[allow(dead_code)] // kept for potential animation updates
    pub model_buffer: wgpu::Buffer,
    #[allow(dead_code)] // kept for potential animation updates
    pub transform: Mat4,
    pub bounds: (Vec3, Vec3),
    #[allow(dead_code)]
    pub name: String,
}

/// Scene points with transform and material
pub struct ScenePoints {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub material_bind_group: wgpu::BindGroup,
    pub model_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    pub model_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    pub transform: Mat4,
    pub bounds: (Vec3, Vec3),
    #[allow(dead_code)]
    pub name: String,
    /// Per-point widths (radius) for point sprites (not yet used in rendering)
    #[allow(dead_code)]
    pub widths: Vec<f32>,
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
            blend: true,  // Enable alpha blending for X-Ray mode
            cull_mode: Some(wgpu::Face::Back),
            wireframe: false,
            ..Default::default()
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
            ..config.clone()
        };
        let pipeline_double_sided = standard_surface::create_pipeline(&device, &layouts, &double_sided_config);
        
        let wireframe_double_sided_config = PipelineConfig {
            wireframe: true,
            cull_mode: None,
            ..double_sided_config.clone()
        };
        let wireframe_pipeline_double_sided = standard_surface::create_pipeline(&device, &layouts, &wireframe_double_sided_config);
        
        // Line pipeline for curves, hair, grid
        let line_config = PipelineConfig {
            topology: wgpu::PrimitiveTopology::LineList,
            cull_mode: None,
            ..double_sided_config
        };
        let line_pipeline = standard_surface::create_pipeline(&device, &layouts, &line_config);

        // Point pipeline for point clouds
        let point_config = PipelineConfig {
            topology: wgpu::PrimitiveTopology::PointList,
            cull_mode: None,
            ..double_sided_config
        };
        let point_pipeline = standard_surface::create_pipeline(&device, &layouts, &point_config);

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
            xray_alpha: 1.0,
            flat_shading: 0.0,
            auto_normals: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
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
            line_pipeline,
            point_pipeline,
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
            grid_step: 0.0, // will be set on first render
            env_map,
            env_uniform_buffer,
            floor_mesh: None,
            meshes: HashMap::new(),
            curves: HashMap::new(),
            points: HashMap::new(),
            show_wireframe: false,
            flat_shading: false,
            show_grid: true,
            show_shadows: true,
            hdr_visible: true,
            xray_alpha: 1.0,
            double_sided: false,
            auto_normals: false,
            background_color: [0.1, 0.1, 0.12, 1.0],
        }
    }

    /// Update camera uniform
    pub fn update_camera(&self, view_proj: Mat4, view: Mat4, position: Vec3) {
        let uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            position,
            xray_alpha: self.xray_alpha,
            flat_shading: if self.flat_shading { 1.0 } else { 0.0 },
            auto_normals: if self.auto_normals { 1.0 } else { 0.0 },
            _pad2: 0.0,
            _pad3: 0.0,
        };
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }
    
    /// Reset to default 3-point lighting
    pub fn set_default_lights(&self) {
        let rig = LightRig::three_point();
        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&rig));
    }
    
    /// Update lights from scene lights
    /// Takes up to 3 scene lights and maps them to key/fill/rim
    pub fn set_scene_lights(&self, scene_lights: &[super::mesh_converter::SceneLight]) {
        use standard_surface::Light;
        
        let make_light = |sl: &super::mesh_converter::SceneLight| -> Light {
            // SceneLight direction points where light goes, shader expects direction toward source
            Light::new(-sl.direction, sl.color, sl.intensity)
        };
        
        let rig = match scene_lights.len() {
            0 => LightRig::three_point(), // fallback to default
            1 => {
                // Single light as key, dim fill/rim
                let key = make_light(&scene_lights[0]);
                LightRig {
                    key,
                    fill: Light::new(Vec3::new(0.6, -0.3, -0.6), Vec3::ONE, 0.2),
                    rim: Light::new(Vec3::new(0.0, -0.5, 0.8), Vec3::ONE, 0.3),
                    ambient: Vec3::splat(0.05),
                    _pad: 0.0,
                }
            }
            2 => {
                // Two lights: key + fill
                LightRig {
                    key: make_light(&scene_lights[0]),
                    fill: make_light(&scene_lights[1]),
                    rim: Light::new(Vec3::new(0.0, -0.5, 0.8), Vec3::ONE, 0.3),
                    ambient: Vec3::splat(0.05),
                    _pad: 0.0,
                }
            }
            _ => {
                // Three or more: key + fill + rim
                LightRig {
                    key: make_light(&scene_lights[0]),
                    fill: make_light(&scene_lights[1]),
                    rim: make_light(&scene_lights[2]),
                    ambient: Vec3::splat(0.03),
                    _pad: 0.0,
                }
            }
        };
        
        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&rig));
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

    /// Update grid mesh based on camera distance (Lightwave-style adaptive grid)
    pub fn update_grid(&mut self, camera_distance: f32) {
        // Calculate grid step as power of 10 based on camera distance
        // e.g. distance 5 -> step 1, distance 50 -> step 10, distance 0.5 -> step 0.1
        let log_dist = camera_distance.max(0.01).log10();
        let step = 10.0_f32.powf(log_dist.floor());
        
        // Only rebuild if step changed
        if (step - self.grid_step).abs() < 0.001 && self.grid_mesh.is_some() {
            return;
        }
        self.grid_step = step;
        
        // Grid covers area proportional to camera distance
        let half_size = step * 10.0; // 10 major divisions visible
        let divisions = 20; // 20 lines each direction
        
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Create grid as LineList - two vertices per line
        for i in 0..=divisions {
            let t = i as f32 / divisions as f32;
            let pos = -half_size + t * half_size * 2.0;
            let idx = vertices.len() as u32;
            
            // X-axis line (parallel to X)
            vertices.push(Vertex { position: [-half_size, 0.0, pos], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] });
            vertices.push(Vertex { position: [half_size, 0.0, pos], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] });
            indices.extend_from_slice(&[idx, idx + 1]);
            
            // Z-axis line (parallel to Z)
            let idx = vertices.len() as u32;
            vertices.push(Vertex { position: [pos, 0.0, -half_size], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] });
            vertices.push(Vertex { position: [pos, 0.0, half_size], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] });
            indices.extend_from_slice(&[idx, idx + 1]);
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
        smooth_data: Option<SmoothNormalData>,
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

        // Calculate bounds from vertices (in world space)
        let bounds = compute_mesh_bounds(vertices, transform);
        
        self.meshes.insert(name.clone(), SceneMesh {
            mesh,
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform,
            vertex_hash: compute_vertex_hash(vertices),
            bounds,
            name,
            smooth_data,
            base_vertices: Some(vertices.to_vec()),
        });
    }
    
    /// Add curves (lines) to the scene
    pub fn add_curves(
        &mut self,
        name: String,
        vertices: &[Vertex],
        indices: &[u32],
        transform: Mat4,
        params: &StandardSurfaceParams,
    ) {
        let mesh = self.create_mesh(vertices, indices);
        
        // Material (using same system as meshes)
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
            label: Some("curves_model_buffer"),
            contents: bytemuck::bytes_of(&model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("curves_model_bind_group"),
            layout: &self.layouts.model,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });
        
        // Calculate bounds
        let bounds = compute_mesh_bounds(vertices, transform);
        
        self.curves.insert(name.clone(), SceneCurves {
            mesh,
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform,
            bounds,
            name,
        });
    }

    /// Add points to scene
    pub fn add_points(
        &mut self,
        name: String,
        positions: &[[f32; 3]],
        widths: &[f32],
        transform: Mat4,
        params: &StandardSurfaceParams,
    ) {
        if positions.is_empty() {
            return;
        }

        // Create vertices from positions (using dummy normals/uvs for point rendering)
        let vertices: Vec<Vertex> = positions.iter().map(|pos| Vertex {
            position: *pos,
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        }).collect();

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("points_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

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
            label: Some("points_model_buffer"),
            contents: bytemuck::bytes_of(&model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("points_model_bind_group"),
            layout: &self.layouts.model,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });

        // Calculate bounds
        let bounds = compute_points_bounds(positions, transform);
        
        self.points.insert(name.clone(), ScenePoints {
            vertex_buffer,
            vertex_count: positions.len() as u32,
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform,
            bounds,
            name,
            widths: widths.to_vec(),
        });
    }

    /// Check if points exist
    #[allow(dead_code)]
    pub fn has_points(&self, name: &str) -> bool {
        self.points.contains_key(name)
    }

    /// Compute combined bounds of all meshes, curves, and points
    pub fn compute_scene_bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut has_any = false;
        
        for mesh in self.meshes.values() {
            min = min.min(mesh.bounds.0);
            max = max.max(mesh.bounds.1);
            has_any = true;
        }
        
        for curves in self.curves.values() {
            min = min.min(curves.bounds.0);
            max = max.max(curves.bounds.1);
            has_any = true;
        }
        
        for points in self.points.values() {
            min = min.min(points.bounds.0);
            max = max.max(points.bounds.1);
            has_any = true;
        }
        
        // Validate bounds: has geometry, properly ordered, and finite
        if has_any && min.x <= max.x && min.is_finite() && max.is_finite() {
            Some((min, max))
        } else {
            None
        }
    }
    
    /// Clear all scene meshes
    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
        self.curves.clear();
        self.points.clear();
    }
    
    /// Update only transform for existing mesh (cheap operation)
    /// Returns true if mesh was found and updated
    pub fn update_mesh_transform(&mut self, name: &str, transform: Mat4) -> bool {
        if let Some(scene_mesh) = self.meshes.get_mut(name) {
            scene_mesh.transform = transform;
            let normal_matrix = transform.inverse().transpose();
            let model_uniform = ModelUniform {
                model: transform.to_cols_array_2d(),
                normal_matrix: normal_matrix.to_cols_array_2d(),
            };
            self.queue.write_buffer(
                &scene_mesh.model_buffer,
                0,
                bytemuck::bytes_of(&model_uniform),
            );
            return true;
        }
        false
    }
    
    /// Update vertex data for existing mesh (for deforming animation)
    /// Returns true if mesh was found and updated
    pub fn update_mesh_vertices(&mut self, name: &str, vertices: &[Vertex], indices: &[u32]) -> bool {
        if !self.meshes.contains_key(name) {
            return false;
        }
        // Create new mesh first to avoid borrow issues
        let new_mesh = self.create_mesh(vertices, indices);
        let new_hash = compute_vertex_hash(vertices);
        if let Some(scene_mesh) = self.meshes.get_mut(name) {
            scene_mesh.mesh = new_mesh;
            scene_mesh.vertex_hash = new_hash;
        }
        true
    }
    
    /// Check if mesh exists
    pub fn has_mesh(&self, name: &str) -> bool {
        self.meshes.contains_key(name)
    }
    
    /// Get vertex hash for change detection
    pub fn get_vertex_hash(&self, name: &str) -> Option<u64> {
        self.meshes.get(name).map(|m| m.vertex_hash)
    }
    
    /// Recalculate smooth normals for all meshes with given angle
    pub fn recalculate_smooth_normals(&mut self, angle_deg: f32, enabled: bool) {
        for scene_mesh in self.meshes.values_mut() {
            if let (Some(smooth_data), Some(base_verts)) = (&scene_mesh.smooth_data, &scene_mesh.base_vertices) {
                let mut new_vertices = base_verts.clone();
                
                if enabled {
                    // Calculate smooth normals
                    let smooth_normals = smooth_data.calculate(angle_deg);
                    for (vert, normal) in new_vertices.iter_mut().zip(smooth_normals.iter()) {
                        vert.normal = (*normal).into();
                    }
                }
                // else: use base_vertices which have flat normals
                
                // Recreate vertex buffer
                let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh_vertex_buffer"),
                    contents: bytemuck::cast_slice(&new_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                scene_mesh.mesh.vertex_buffer = vertex_buffer;
            }
        }
    }
    
    /// Update curves transform
    #[allow(dead_code)]
    pub fn update_curves_transform(&mut self, name: &str, transform: Mat4) -> bool {
        if let Some(curves) = self.curves.get_mut(name) {
            let normal_matrix = transform.inverse().transpose();
            let model_uniform = ModelUniform {
                model: transform.to_cols_array_2d(),
                normal_matrix: normal_matrix.to_cols_array_2d(),
            };
            self.queue.write_buffer(
                &curves.model_buffer,
                0,
                bytemuck::bytes_of(&model_uniform),
            );
            return true;
        }
        false
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
    
    /// Set floor plane based on scene bounds (call when checkbox enabled)
    pub fn set_floor(&mut self, bounds: &Option<super::mesh_converter::Bounds>) {
        // Get scene size, default to reasonable size if no bounds
        let (center, size) = if let Some(b) = bounds {
            let c = b.center();
            let r = b.radius().max(1.0);
            (c, r * 4.0)  // 4x scene radius
        } else {
            (Vec3::ZERO, 10.0)
        };
        
        // Create floor quad at Y=0
        let half = size;
        let y = 0.0;
        let vertices = vec![
            Vertex { position: [center.x - half, y, center.z - half], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
            Vertex { position: [center.x + half, y, center.z - half], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
            Vertex { position: [center.x + half, y, center.z + half], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
            Vertex { position: [center.x - half, y, center.z + half], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
        ];
        // Counter-clockwise winding for front-face (viewed from above)
        let indices: Vec<u32> = vec![0, 2, 1, 0, 3, 2];
        
        // Create GPU buffers
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("floor_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("floor_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Dark matte material
        let material = StandardSurfaceParams::plastic(Vec3::new(0.08, 0.08, 0.1), 0.6);
        let material_buffer = standard_surface::create_material_buffer(&self.device, &material);
        let material_bind_group = standard_surface::create_material_bind_group(
            &self.device,
            &self.layouts.material,
            &material_buffer,
        );
        
        // Model transform (identity)
        let model_uniform = ModelUniform {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            normal_matrix: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let model_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("floor_model_buffer"),
            contents: bytemuck::bytes_of(&model_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("floor_model_bind_group"),
            layout: &self.layouts.model,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });
        
        self.floor_mesh = Some(SceneMesh {
            mesh: Mesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
            },
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform: Mat4::IDENTITY,
            vertex_hash: 0,
            bounds: (Vec3::ZERO, Vec3::ZERO),
            name: "_FLOOR_".into(),
            smooth_data: None,
            base_vertices: None,
        });
    }
    
    /// Clear floor plane (call when checkbox disabled)
    pub fn clear_floor(&mut self) {
        self.floor_mesh = None;
    }

    /// Render the scene
    pub fn render(&mut self, view: &wgpu::TextureView, width: u32, height: u32, camera_distance: f32) {
        self.ensure_depth_texture(width, height);
        self.update_grid(camera_distance);

        let depth_view = match &self.depth_texture {
            Some(dt) => &dt.view,
            None => return,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        // Shadow depth pass - render scene from light's perspective
        if self.show_shadows {
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
            for mesh in self.meshes.values() {
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
                    depth_slice: None,
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

            // Draw skybox if environment is loaded and visible
            if self.has_environment() && self.hdr_visible {
                render_pass.set_pipeline(&self.skybox_pipeline);
                render_pass.set_bind_group(0, &self.skybox_camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.env_map.bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.skybox_vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.skybox_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.skybox_index_count, 0, 0..1);
            }

            // Select pipeline based on display mode
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

            // Draw grid using line pipeline
            if self.show_grid {
                if let Some(grid) = &self.grid_mesh {
                    render_pass.set_pipeline(&self.line_pipeline);
                    render_pass.set_bind_group(1, &self.grid_material, &[]);
                    render_pass.set_bind_group(2, &self.grid_model, &[]);
                    render_pass.set_vertex_buffer(0, grid.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(grid.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..grid.index_count, 0, 0..1);
                    // Restore mesh pipeline
                    render_pass.set_pipeline(pipeline);
                }
            }
            
            // Draw floor (before scene meshes so it's behind)
            if let Some(floor) = &self.floor_mesh {
                render_pass.set_bind_group(1, &floor.material_bind_group, &[]);
                render_pass.set_bind_group(2, &floor.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, floor.mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(floor.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..floor.mesh.index_count, 0, 0..1);
            }

            // Draw scene meshes
            for mesh in self.meshes.values() {
                render_pass.set_bind_group(1, &mesh.material_bind_group, &[]);
                render_pass.set_bind_group(2, &mesh.model_bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
            }
            
            // Draw curves using line pipeline
            if !self.curves.is_empty() {
                render_pass.set_pipeline(&self.line_pipeline);
                for curve in self.curves.values() {
                    render_pass.set_bind_group(1, &curve.material_bind_group, &[]);
                    render_pass.set_bind_group(2, &curve.model_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, curve.mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(curve.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..curve.mesh.index_count, 0, 0..1);
                }
            }

            // Draw points using point pipeline
            if !self.points.is_empty() {
                render_pass.set_pipeline(&self.point_pipeline);
                for pts in self.points.values() {
                    render_pass.set_bind_group(1, &pts.material_bind_group, &[]);
                    render_pass.set_bind_group(2, &pts.model_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, pts.vertex_buffer.slice(..));
                    render_pass.draw(0..pts.vertex_count, 0..1);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
