//! wgpu renderer with Standard Surface shader

use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

mod resources;
mod shaders;
mod postfx;
mod passes;
mod pipelines;

use resources::{DepthTexture, GBuffer, LightingParams, SsaoBlurParams, SsaoParams, SsaoTargets};
use postfx::{create_postfx_pipelines, PostFxPipelines};
use pipelines::{create_pipelines, Pipelines};

use standard_surface::{
    BindGroupLayouts, CameraUniform, LightRig, ModelUniform,
    ShadowUniform, StandardSurfaceParams, Vertex,
};

use super::environment::{self, EnvironmentMap, EnvUniform};
use super::smooth_normals::SmoothNormalData;

/// Shadow map resolution
const SHADOW_MAP_SIZE: u32 = 2048;

const DEFAULT_BACKGROUND_COLOR: [f32; 4] = [0.1, 0.1, 0.12, 1.0];

/// Main renderer state
pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    // Pipelines
    pipelines: Pipelines,
    postfx: PostFxPipelines,
    ssao_bind_group: Option<wgpu::BindGroup>,
    ssao_blur_bind_group: Option<wgpu::BindGroup>,
    lighting_bind_group: Option<wgpu::BindGroup>,
    ssao_params_buffer: wgpu::Buffer,
    ssao_blur_params_buffer: wgpu::Buffer,
    lighting_params_buffer: wgpu::Buffer,
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
    gbuffer: Option<GBuffer>,
    ssao_targets: Option<SsaoTargets>,
    
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
    pub use_ssao: bool,
    pub ssao_strength: f32,
    pub ssao_radius: f32,
    pub hdr_visible: bool,
    pub xray_alpha: f32,
    pub double_sided: bool,
    pub auto_normals: bool,
    pub background_color: [f32; 4],

    // Scene bounds for shadow calculations
    scene_center: Vec3,
    scene_radius: f32,
    camera_position: Vec3,
}

/// GPU mesh data
pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    vertex_buffer_size: usize,
    index_buffer_size: usize,
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
    pub opacity: f32,
    #[allow(dead_code)]
    pub name: String,
    // For dynamic smooth normal recalculation
    pub smooth_data: Option<SmoothNormalData>,
    pub base_vertices: Option<Vec<Vertex>>,  // vertices with flat normals
    pub smooth_dirty: bool,
}

/// Compute a content hash for mesh data (vertices + indices).
pub fn compute_mesh_hash(vertices: &[standard_surface::Vertex], indices: &[u32]) -> u64 {
    let vert_bytes = bytemuck::cast_slice(vertices);
    let index_bytes = bytemuck::cast_slice(indices);
    let (h1, h2) = spooky_hash::SpookyHash::hash128(vert_bytes, 0, 0);
    let (h3, h4) = spooky_hash::SpookyHash::hash128(index_bytes, h1, h2);
    h3 ^ h4
}

/// Compute a content hash for curves (vertices + indices).
pub fn compute_curves_hash(vertices: &[standard_surface::Vertex], indices: &[u32]) -> u64 {
    compute_mesh_hash(vertices, indices)
}

/// Compute a content hash for points (positions + widths).
pub fn compute_points_hash(positions: &[[f32; 3]], widths: &[f32]) -> u64 {
    let pos_bytes = bytemuck::cast_slice(positions);
    let width_bytes = bytemuck::cast_slice(widths);
    let (h1, h2) = spooky_hash::SpookyHash::hash128(pos_bytes, 0, 0);
    let (h3, h4) = spooky_hash::SpookyHash::hash128(width_bytes, h1, h2);
    h3 ^ h4
}

/// Compute average opacity from material params.
fn params_opacity(params: &StandardSurfaceParams) -> f32 {
    (params.opacity.x + params.opacity.y + params.opacity.z) / 3.0
}

/// Compute center point from bounds.
fn bounds_center(bounds: (Vec3, Vec3)) -> Vec3 {
    (bounds.0 + bounds.1) * 0.5
}

/// Compute a back-to-front sort distance for a bounds.
fn bounds_sort_distance(bounds: (Vec3, Vec3), camera_position: Vec3) -> f32 {
    let center = bounds_center(bounds);
    let radius = (bounds.1 - bounds.0).length() * 0.5;
    (center - camera_position).length() + radius
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
    pub data_hash: u64,
    #[allow(dead_code)]
    pub name: String,
}

/// Scene points with transform and material
pub struct ScenePoints {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub vertex_buffer_size: usize,
    pub material_bind_group: wgpu::BindGroup,
    pub model_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    pub model_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    pub transform: Mat4,
    pub bounds: (Vec3, Vec3),
    pub data_hash: u64,
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
        let pipelines = create_pipelines(&device, &layouts, format);
        let postfx = create_postfx_pipelines(&device, format);
        let ssao_params = SsaoParams {
            strength: [0.5, 0.0, 0.0, 0.0],
        };
        let ssao_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao_params_buffer"),
            contents: bytemuck::bytes_of(&ssao_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ssao_blur_params = SsaoBlurParams {
            direction: [1.0, 0.0],
            _pad: [0.0, 0.0],
        };
        let ssao_blur_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao_blur_params_buffer"),
            contents: bytemuck::bytes_of(&ssao_blur_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let lighting_params = LightingParams {
            background: self::DEFAULT_BACKGROUND_COLOR,
            hdr_visible: 1.0,
            _pad0: [0.0; 3],
            _pad1: [0.0; 4],
        };
        let lighting_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lighting_params_buffer"),
            contents: bytemuck::bytes_of(&lighting_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
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
            pipelines,
            postfx,
            ssao_bind_group: None,
            ssao_blur_bind_group: None,
            lighting_bind_group: None,
            ssao_params_buffer,
            ssao_blur_params_buffer,
            lighting_params_buffer,
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
            gbuffer: None,
            ssao_targets: None,
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
            use_ssao: false,
            ssao_strength: 0.5,
            ssao_radius: 0.015,
            hdr_visible: true,
            xray_alpha: 1.0,
            double_sided: true,
            auto_normals: true,
            background_color: DEFAULT_BACKGROUND_COLOR,
            scene_center: Vec3::ZERO,
            scene_radius: 10.0,
            camera_position: Vec3::ZERO,
        }
    }

    /// Update camera uniform
    pub fn update_camera(&mut self, view_proj: Mat4, view: Mat4, position: Vec3) {
        let inv_view_proj = view_proj.inverse();
        let uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            position,
            xray_alpha: self.xray_alpha,
            flat_shading: if self.flat_shading { 1.0 } else { 0.0 },
            auto_normals: if self.auto_normals { 1.0 } else { 0.0 },
            _pad2: 0.0,
            _pad3: 0.0,
        };
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
        self.camera_position = position;
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

    /// Set scene bounds for shadow calculations
    pub fn set_scene_bounds(&mut self, center: Vec3, radius: f32) {
        self.scene_center = center;
        self.scene_radius = radius.max(1.0);
    }

    /// Update shadow map for key light
    /// Creates orthographic projection from light's perspective
    pub fn update_shadow(&self, light_dir: Vec3) {
        // Light position far from scene, looking at center
        let light_pos = self.scene_center - light_dir.normalize() * self.scene_radius * 3.0;
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        
        let light_view = Mat4::look_at_rh(light_pos, self.scene_center, up);
        // Orthographic projection covering scene
        // Floor is 4x radius, so shadow frustum needs to cover it fully
        let half_size = self.scene_radius * 4.5;
        let light_proj = Mat4::orthographic_rh(
            -half_size, half_size,
            -half_size, half_size,
            0.1, self.scene_radius * 6.0,
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
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
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

    fn ensure_gbuffer(&mut self, width: u32, height: u32) {
        let needs_recreate = match &self.gbuffer {
            Some(gb) => gb.size != (width, height),
            None => true,
        };

        if needs_recreate && width > 0 && height > 0 {
            let albedo = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gbuffer_albedo"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let normals = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gbuffer_normals"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let occlusion = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gbuffer_occlusion"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let albedo_view = albedo.create_view(&wgpu::TextureViewDescriptor::default());
            let normals_view = normals.create_view(&wgpu::TextureViewDescriptor::default());
            let occlusion_view = occlusion.create_view(&wgpu::TextureViewDescriptor::default());

            self.gbuffer = Some(GBuffer {
                albedo,
                normals,
                occlusion,
                albedo_view,
                normals_view,
                occlusion_view,
                size: (width, height),
            });
        }
    }

    fn ensure_ssao_targets(&mut self, width: u32, height: u32) {
        let needs_recreate = match &self.ssao_targets {
            Some(t) => t.size != (width, height),
            None => true,
        };

        if needs_recreate && width > 0 && height > 0 {
            let color = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ssao_color"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());

            self.ssao_targets = Some(SsaoTargets {
                color,
                color_view,
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
            vertex_buffer_size: std::mem::size_of::<Vertex>() * vertices.len(),
            index_buffer_size: std::mem::size_of::<u32>() * indices.len(),
        });
    }

    /// Create a mesh from vertices and indices
    pub fn create_mesh(
        device: &wgpu::Device,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> Mesh {
        let vertex_buffer_size = std::mem::size_of::<Vertex>() * vertices.len();
        let index_buffer_size = std::mem::size_of::<u32>() * indices.len();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            vertex_buffer_size,
            index_buffer_size,
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
        let mesh = Self::create_mesh(&self.device, vertices, indices);

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
        let opacity = params_opacity(params);
        
        self.meshes.insert(name.clone(), SceneMesh {
            mesh,
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform,
            vertex_hash: compute_mesh_hash(vertices, indices),
            bounds,
            opacity,
            name,
            smooth_data,
            base_vertices: Some(vertices.to_vec()),
            smooth_dirty: true,
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
        let mesh = Self::create_mesh(&self.device, vertices, indices);
        let data_hash = compute_curves_hash(vertices, indices);
        
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
            data_hash,
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

        let vertex_buffer_size = std::mem::size_of::<Vertex>() * vertices.len();
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
        
        let data_hash = compute_points_hash(positions, widths);
        self.points.insert(name.clone(), ScenePoints {
            vertex_buffer,
            vertex_count: positions.len() as u32,
            vertex_buffer_size,
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform,
            bounds,
            data_hash,
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
            if let Some(base_vertices) = scene_mesh.base_vertices.as_ref() {
                scene_mesh.bounds = compute_mesh_bounds(base_vertices, transform);
            }
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
        let new_hash = compute_mesh_hash(vertices, indices);
        let vertex_bytes = bytemuck::cast_slice(vertices);
        let index_bytes = bytemuck::cast_slice(indices);
        let vertex_size = vertex_bytes.len();
        let index_size = index_bytes.len();
        if let Some(scene_mesh) = self.meshes.get_mut(name) {
            if vertex_size <= scene_mesh.mesh.vertex_buffer_size
                && index_size <= scene_mesh.mesh.index_buffer_size
            {
                self.queue.write_buffer(&scene_mesh.mesh.vertex_buffer, 0, vertex_bytes);
                self.queue.write_buffer(&scene_mesh.mesh.index_buffer, 0, index_bytes);
                scene_mesh.mesh.index_count = indices.len() as u32;
            } else {
                let new_mesh = Self::create_mesh(&self.device, vertices, indices);
                scene_mesh.mesh = new_mesh;
            }
            scene_mesh.vertex_hash = new_hash;
            scene_mesh.bounds = compute_mesh_bounds(vertices, scene_mesh.transform);
            scene_mesh.base_vertices = Some(vertices.to_vec());
            scene_mesh.smooth_dirty = true;
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
    
    /// Recalculate smooth normals for meshes with given angle.
    /// When force_all is true, all meshes are updated; otherwise only dirty meshes.
    pub fn recalculate_smooth_normals(&mut self, angle_deg: f32, enabled: bool, force_all: bool) {
        for scene_mesh in self.meshes.values_mut() {
            if !force_all && !scene_mesh.smooth_dirty {
                continue;
            }
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
                scene_mesh.smooth_dirty = false;
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

    /// Update curves vertices/indices for existing curves
    pub fn update_curves_vertices(&mut self, name: &str, vertices: &[Vertex], indices: &[u32]) -> bool {
        if !self.curves.contains_key(name) {
            return false;
        }
        let new_hash = compute_curves_hash(vertices, indices);
        let vertex_bytes = bytemuck::cast_slice(vertices);
        let index_bytes = bytemuck::cast_slice(indices);
        let vertex_size = vertex_bytes.len();
        let index_size = index_bytes.len();
        if let Some(curves) = self.curves.get_mut(name) {
            if vertex_size <= curves.mesh.vertex_buffer_size
                && index_size <= curves.mesh.index_buffer_size
            {
                self.queue.write_buffer(&curves.mesh.vertex_buffer, 0, vertex_bytes);
                self.queue.write_buffer(&curves.mesh.index_buffer, 0, index_bytes);
                curves.mesh.index_count = indices.len() as u32;
            } else {
                let new_mesh = Self::create_mesh(&self.device, vertices, indices);
                curves.mesh = new_mesh;
            }
            curves.data_hash = new_hash;
            curves.bounds = compute_mesh_bounds(vertices, curves.transform);
        }
        true
    }

    /// Check if curves exist
    pub fn has_curves(&self, name: &str) -> bool {
        self.curves.contains_key(name)
    }

    /// Get curves hash for change detection
    pub fn get_curves_hash(&self, name: &str) -> Option<u64> {
        self.curves.get(name).map(|c| c.data_hash)
    }

    /// Update points transform
    #[allow(dead_code)]
    pub fn update_points_transform(&mut self, name: &str, transform: Mat4) -> bool {
        if let Some(points) = self.points.get_mut(name) {
            let normal_matrix = transform.inverse().transpose();
            let model_uniform = ModelUniform {
                model: transform.to_cols_array_2d(),
                normal_matrix: normal_matrix.to_cols_array_2d(),
            };
            self.queue.write_buffer(
                &points.model_buffer,
                0,
                bytemuck::bytes_of(&model_uniform),
            );
            return true;
        }
        false
    }

    /// Update points vertices for existing points
    pub fn update_points_vertices(&mut self, name: &str, positions: &[[f32; 3]], widths: &[f32]) -> bool {
        if !self.points.contains_key(name) {
            return false;
        }
        let new_hash = compute_points_hash(positions, widths);
        let vertices: Vec<Vertex> = positions.iter().map(|pos| Vertex {
            position: *pos,
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        }).collect();
        let vertex_bytes = bytemuck::cast_slice(&vertices);
        let vertex_size = vertex_bytes.len();
        if let Some(points) = self.points.get_mut(name) {
            if vertex_size <= points.vertex_buffer_size {
                self.queue.write_buffer(&points.vertex_buffer, 0, vertex_bytes);
            } else {
                points.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("points_vertex_buffer"),
                    contents: vertex_bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                points.vertex_buffer_size = vertex_size;
            }
            points.vertex_count = positions.len() as u32;
            points.data_hash = new_hash;
            points.bounds = compute_points_bounds(positions, points.transform);
            points.widths = widths.to_vec();
        }
        true
    }

    /// Get points hash for change detection
    pub fn get_points_hash(&self, name: &str) -> Option<u64> {
        self.points.get(name).map(|p| p.data_hash)
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
        // Get scene size and floor Y position
        let (center, size, y) = if let Some(b) = bounds {
            let c = b.center();
            let r = b.radius().max(1.0);
            // Floor at 0.1 below scene's lower bound (avoid z-fighting)
            (c, r * 4.0, b.min.y - 0.1)
        } else {
            (Vec3::ZERO, 10.0, 0.0)
        };
        
        // Create floor quad
        let half = size;
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
        
        let opacity = params_opacity(&material);
        let bounds = compute_mesh_bounds(&vertices, Mat4::IDENTITY);
        self.floor_mesh = Some(SceneMesh {
            mesh: Mesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                vertex_buffer_size: std::mem::size_of::<Vertex>() * vertices.len(),
                index_buffer_size: std::mem::size_of::<u32>() * indices.len(),
            },
            material_bind_group,
            model_bind_group,
            model_buffer,
            transform: Mat4::IDENTITY,
            vertex_hash: 0,
            bounds,
            opacity,
            name: "_FLOOR_".into(),
            smooth_data: None,
            base_vertices: None,
            smooth_dirty: false,
        });
    }
    
    /// Clear floor plane (call when checkbox disabled)
    pub fn clear_floor(&mut self) {
        self.floor_mesh = None;
    }

    /// Render the scene
    pub fn render(&mut self, view: &wgpu::TextureView, width: u32, height: u32, camera_distance: f32) {
        self.ensure_depth_texture(width, height);
        let use_gbuffer = true;
        if use_gbuffer {
            self.ensure_gbuffer(width, height);
        }
        self.update_grid(camera_distance);

        let depth_view = match &self.depth_texture {
            Some(dt) => dt.view.clone(),
            None => return,
        };
        let color_target_view_ref = view;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        self.render_shadow_pass(&mut encoder);

        // Opaque render pass (Stage 1: G-Buffer + lighting)
        if self.show_wireframe {
            let mut meshes: Vec<&SceneMesh> = self.meshes.values().collect();
            if let Some(floor) = &self.floor_mesh {
                meshes.push(floor);
            }
            let opaque_depth_load = wgpu::LoadOp::Clear(1.0);
            let wire_pipeline = if self.double_sided {
                &self.pipelines.wireframe_pipeline_double_sided
            } else {
                &self.pipelines.wireframe_pipeline
            };
            self.render_opaque_pass(
                &mut encoder,
                &depth_view,
                color_target_view_ref,
                &meshes,
                wire_pipeline,
                None,
                opaque_depth_load,
            );
            self.queue.submit(std::iter::once(encoder.finish()));
            return;
        }

        let opacity_threshold = 0.999;
        let mut opaque_mesh_names: Vec<String> = Vec::new();
        let mut transparent_meshes: Vec<(f32, String)> = Vec::new();
        let mut floor_transparent_distance: Option<f32> = None;
        let mut floor_opaque = false;

        for (name, mesh) in &self.meshes {
            let effective_opacity = mesh.opacity * self.xray_alpha;
            if effective_opacity < opacity_threshold {
                let distance = bounds_sort_distance(mesh.bounds, self.camera_position);
                transparent_meshes.push((distance, name.clone()));
            } else {
                opaque_mesh_names.push(name.clone());
            }
        }
        if let Some(floor) = &self.floor_mesh {
            let effective_opacity = floor.opacity * self.xray_alpha;
            if effective_opacity < opacity_threshold {
                let distance = bounds_sort_distance(floor.bounds, self.camera_position);
                floor_transparent_distance = Some(distance);
            } else {
                floor_opaque = true;
            }
        }

        {
            let mut opaque_meshes: Vec<&SceneMesh> = Vec::new();
            for name in &opaque_mesh_names {
                if let Some(mesh) = self.meshes.get(name) {
                    opaque_meshes.push(mesh);
                }
            }
            if floor_opaque {
                if let Some(floor) = &self.floor_mesh {
                    opaque_meshes.push(floor);
                }
            }

            if use_gbuffer {
                self.render_gbuffer_pass(&mut encoder, &depth_view, &opaque_meshes);
            }
        }

        if use_gbuffer {
            self.ensure_ssao_targets(width, height);
            self.render_ssao_pass(&mut encoder, &depth_view, self.use_ssao);
            if self.use_ssao {
                let (gbuffer_occlusion_view, ssao_temp_view) = match (&self.gbuffer, &self.ssao_targets) {
                    (Some(gbuffer), Some(targets)) => (
                        gbuffer.occlusion_view.clone(),
                        targets.color_view.clone(),
                    ),
                    _ => return,
                };

                let blur_params = SsaoBlurParams {
                    direction: [1.0, 0.0],
                    _pad: [0.0, 0.0],
                };
                self.queue.write_buffer(
                    &self.ssao_blur_params_buffer,
                    0,
                    bytemuck::bytes_of(&blur_params),
                );
                self.render_ssao_blur_pass(&mut encoder, &gbuffer_occlusion_view, &ssao_temp_view);

                let blur_params = SsaoBlurParams {
                    direction: [0.0, 1.0],
                    _pad: [0.0, 0.0],
                };
                self.queue.write_buffer(
                    &self.ssao_blur_params_buffer,
                    0,
                    bytemuck::bytes_of(&blur_params),
                );
                self.render_ssao_blur_pass(&mut encoder, &ssao_temp_view, &gbuffer_occlusion_view);
            }

            let lighting_params = LightingParams {
                background: self.background_color,
                hdr_visible: if self.hdr_visible { 1.0 } else { 0.0 },
                _pad0: [0.0; 3],
                _pad1: [0.0; 4],
            };
            self.queue.write_buffer(
                &self.lighting_params_buffer,
                0,
                bytemuck::bytes_of(&lighting_params),
            );
            let occlusion_view = match &self.gbuffer {
                Some(gbuffer) => gbuffer.occlusion_view.clone(),
                None => return,
            };
            self.render_lighting_pass(&mut encoder, color_target_view_ref, &occlusion_view, &depth_view);
        }

        if !transparent_meshes.is_empty()
            || floor_transparent_distance.is_some()
            || !self.curves.is_empty()
            || !self.points.is_empty()
        {
            transparent_meshes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut sorted_meshes: Vec<(f32, &SceneMesh)> = Vec::new();
            for (distance, name) in transparent_meshes {
                if let Some(mesh) = self.meshes.get(&name) {
                    sorted_meshes.push((distance, mesh));
                }
            }
            if let (Some(distance), Some(floor)) = (floor_transparent_distance, &self.floor_mesh) {
                sorted_meshes.push((distance, floor));
            }
            sorted_meshes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let sorted_mesh_refs: Vec<&SceneMesh> = sorted_meshes
                .into_iter()
                .map(|(_, mesh)| mesh)
                .collect();
            let transparent_pipeline = if self.double_sided {
                &self.pipelines.transparent_pipeline_double_sided
            } else {
                &self.pipelines.transparent_pipeline
            };
            self.render_transparent_pass(
                &mut encoder,
                &depth_view,
                color_target_view_ref,
                &sorted_mesh_refs,
                transparent_pipeline,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
