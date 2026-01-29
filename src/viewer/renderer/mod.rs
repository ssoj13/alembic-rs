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

use resources::{DepthTexture, GBuffer, LightingParams, ObjectIdTexture, SsaoBlurParams, SsaoParams, SsaoTargets};
use postfx::{create_postfx_pipelines, PostFxPipelines};
use pipelines::{create_pipelines, create_hover_pipeline, HoverParams, HoverPipeline, Pipelines};

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
    ssao_blur_h_bind_group: Option<wgpu::BindGroup>,
    ssao_blur_v_bind_group: Option<wgpu::BindGroup>,
    lighting_bind_group: Option<wgpu::BindGroup>,
    ssao_params_buffer: wgpu::Buffer,
    ssao_blur_h_params_buffer: wgpu::Buffer,
    ssao_blur_v_params_buffer: wgpu::Buffer,
    lighting_params_buffer: wgpu::Buffer,
    /// Tracks whether cached post-fx bind groups need rebuild (set on texture resize/env change)
    postfx_bind_groups_dirty: bool,
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
    pub floor_mesh: Option<SceneMesh>,
    
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

    // Path tracer (compute-based, optional)
    pub path_tracer: Option<super::pathtracer::PathTraceCompute>,
    /// When true, render using path tracer instead of rasterizer.
    pub use_path_tracing: bool,
    pub pt_max_samples: u32,
    pub pt_max_bounces: u32,
    /// Samples per frame update (batch multiple samples before display refresh)
    pub pt_samples_per_update: u32,
    pub pt_target_fps: f32,
    pub pt_auto_spp: bool,
    pub pt_camera_snap: bool,  // Snap camera at target FPS intervals
    last_render_time_ms: f32,  // Actual render time (not including wait)
    pt_batch_rendering: bool,  // True during multi-sample batch (skip camera updates)
    pt_camera_snap_time: std::time::Instant,  // When PT camera was last snapped
    pt_snap_ready: bool,  // True when it's time to dispatch new samples (snap interval reached)
    pt_last_dispatch_time: std::time::Instant,  // When PT last dispatched samples (for FPS limiting)
    /// Samples dispatched in last second (for samples/sec display)
    pub pt_samples_last_sec: u32,
    pt_samples_counter: u32,  // Running counter for current second
    pt_samples_sec_start: std::time::Instant,  // Start of current second
    // Snapped camera state (used for PT when snap is enabled)
    pt_snapped_view_proj: Option<glam::Mat4>,
    pt_snapped_view: Option<glam::Mat4>,
    pt_snapped_position: Option<glam::Vec3>,
    /// Max transmission/glass bounces (separate from diffuse/specular bounces)
    pub pt_max_transmission_depth: u32,
    /// Depth of field enabled
    pub pt_dof_enabled: bool,
    /// Aperture radius for DoF
    pub pt_aperture: f32,
    /// Focus distance for DoF (recomputed from focus_world_point each frame)
    pub pt_focus_distance: f32,
    /// World-space point to keep in focus (set by F-key or Ctrl+LMB pick)
    pub pt_focus_point: Option<glam::Vec3>,
    /// Surface format needed for path tracer blit pipeline creation.
    #[allow(dead_code)]
    surface_format: wgpu::TextureFormat,
    
    // Hover highlighting (object ID buffer approach)
    object_id_texture: Option<ObjectIdTexture>,
    object_id_pick_buffer: wgpu::Buffer,
    hover_pipeline: HoverPipeline,
    hover_bind_group: Option<wgpu::BindGroup>,
    pub hover_mode: super::settings::HoverMode,
    pub hover_outline_thickness: f32,
    pub hover_outline_alpha: f32,
    pub hovered_mesh_path: Option<String>,
    pub hovered_object_id: u32,              // Current hovered object ID (0 = none)
    pending_hover_pick: Option<(u32, u32)>,  // Pixel to read for hover detection
    mesh_id_map: HashMap<u32, String>,       // Map object ID -> mesh path
    next_object_id: u32,                     // Counter for assigning object IDs
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
    pub object_id: u32,  // Unique ID for hover/picking (0 = none/background)
    /// Per-object visibility flag. Controls rendering in both rasterizer and
    /// path tracer. When `false`, the mesh is skipped during opaque/transparent
    /// draw calls and marked invisible in the PT visibility buffer.
    /// Toggled via UI (e.g. floor checkbox) or programmatically.
    pub visible: bool,
    // For dynamic smooth normal recalculation
    pub smooth_data: Option<SmoothNormalData>,
    pub base_vertices: Option<Vec<Vertex>>,  // vertices with flat normals
    pub base_indices: Option<Vec<u32>>,      // face indices for path tracer
    pub material_params: StandardSurfaceParams, // CPU-side material for path tracer
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

/// Convert LINE_LIST curves into ribbon triangles for path tracer.
/// Each line segment becomes a thin quad (2 tris) oriented toward camera-up.
/// Width is read from vertex uv.y (set during curve conversion).
fn curves_to_ribbon_tris(
    verts: &[Vertex],
    indices: &[u32],
    transform: &Mat4,
    material_id: u32,
) -> Vec<super::pathtracer::bvh::Triangle> {
    use super::pathtracer::bvh::Triangle;
    let mut tris = Vec::with_capacity(indices.len()); // ~2 tris per segment

    for pair in indices.chunks_exact(2) {
        let (i0, i1) = (pair[0] as usize, pair[1] as usize);
        if i0 >= verts.len() || i1 >= verts.len() {
            continue;
        }

        let p0 = transform.transform_point3(Vec3::from(verts[i0].position));
        let p1 = transform.transform_point3(Vec3::from(verts[i1].position));

        // Width from uv.y (half-width for offset)
        let w0 = verts[i0].uv[1].max(0.001) * 0.5;
        let w1 = verts[i1].uv[1].max(0.001) * 0.5;

        // Tangent along segment
        let tangent = (p1 - p0).normalize_or_zero();
        if tangent == Vec3::ZERO {
            continue;
        }

        // Choose a side vector perpendicular to tangent
        let up = if tangent.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
        let side = tangent.cross(up).normalize();

        // Quad corners
        let a = p0 - side * w0;
        let b = p0 + side * w0;
        let c = p1 + side * w1;
        let d = p1 - side * w1;

        // Normal = side × tangent (face normal)
        let n = side.cross(tangent).normalize();
        let nn: [f32; 3] = n.into();

        tris.push(Triangle {
            v0: a.into(), v1: b.into(), v2: c.into(),
            n0: nn, n1: nn, n2: nn,
            material_id, object_id: 0,
        });
        tris.push(Triangle {
            v0: a.into(), v1: c.into(), v2: d.into(),
            n0: nn, n1: nn, n2: nn,
            material_id, object_id: 0,
        });
    }
    tris
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
    pub transform: Mat4,
    pub bounds: (Vec3, Vec3),
    pub data_hash: u64,
    #[allow(dead_code)]
    pub name: String,
    // CPU-side data for path tracer ribbon conversion
    pub base_vertices: Option<Vec<Vertex>>,
    pub base_indices: Option<Vec<u32>>,
    pub material_params: StandardSurfaceParams,
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
        let ssao_blur_h_params = SsaoBlurParams {
            direction: [1.0, 0.0],
            _pad: [0.0, 0.0],
        };
        let ssao_blur_h_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao_blur_h_params_buffer"),
            contents: bytemuck::bytes_of(&ssao_blur_h_params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let ssao_blur_v_params = SsaoBlurParams {
            direction: [0.0, 1.0],
            _pad: [0.0, 0.0],
        };
        let ssao_blur_v_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao_blur_v_params_buffer"),
            contents: bytemuck::bytes_of(&ssao_blur_v_params),
            usage: wgpu::BufferUsages::UNIFORM,
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
        
        // Buffer for reading back object ID - needs to hold entire row (aligned to 256)
        // For 4K (3840px) * 4 bytes = 15360 bytes, aligned = 15360 bytes
        // Use 64KB to handle any reasonable resolution
        let object_id_pick_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("object_id_pick_buffer"),
            size: 65536, // 64KB - enough for 4K+ row
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        let hover_pipeline = create_hover_pipeline(&device, format);
        
        Self {
            device,
            queue,
            pipelines,
            postfx,
            ssao_bind_group: None,
            ssao_blur_h_bind_group: None,
            ssao_blur_v_bind_group: None,
            lighting_bind_group: None,
            ssao_params_buffer,
            ssao_blur_h_params_buffer,
            ssao_blur_v_params_buffer,
            lighting_params_buffer,
            postfx_bind_groups_dirty: true,
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
            path_tracer: None,
            use_path_tracing: false,
            pt_max_samples: 512,
            pt_max_bounces: 4,
            pt_samples_per_update: 1,
            pt_target_fps: 30.0,
            pt_auto_spp: false,
            pt_camera_snap: true,
            last_render_time_ms: 10.0,  // Conservative initial estimate
            pt_batch_rendering: false,
            pt_camera_snap_time: std::time::Instant::now(),
            pt_snap_ready: true,
            pt_last_dispatch_time: std::time::Instant::now(),
            pt_samples_last_sec: 0,
            pt_samples_counter: 0,
            pt_samples_sec_start: std::time::Instant::now(),
            pt_snapped_view_proj: None,
            pt_snapped_view: None,
            pt_snapped_position: None,
            pt_max_transmission_depth: 8,
            pt_dof_enabled: false,
            pt_aperture: 0.1,
            pt_focus_distance: 10.0,
            pt_focus_point: None,
            surface_format: format,
            object_id_texture: None,
            object_id_pick_buffer,
            hover_pipeline,
            hover_bind_group: None,
            hover_mode: super::settings::HoverMode::None,
            hover_outline_thickness: 2.0,
            hover_outline_alpha: 1.0,
            hovered_mesh_path: None,
            hovered_object_id: 0,
            pending_hover_pick: None,
            mesh_id_map: HashMap::new(),
            next_object_id: 1,  // 0 is reserved for background
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

        // Recompute focus distance from world-space focus point
        if let Some(fp) = self.pt_focus_point {
            self.pt_focus_distance = (fp - position).length();
        }

        // Update path tracer camera — only reset accumulation if camera actually moved
        // Skip during batch rendering to avoid ghost samples
        if self.pt_batch_rendering {
            return;
        }

        // Camera snapping: Use a fixed camera position for PT to prevent ghost samples.
        // When snap is enabled, we store a snapped camera and only update it at target FPS intervals.
        // Between intervals, samples accumulate using the snapped camera even if user moves.

        // Determine which camera to use for PT
        let (pt_view_proj, pt_view, pt_position) = if self.pt_camera_snap {
            let snap_interval = 1.0 / self.pt_target_fps;
            let elapsed = self.pt_camera_snap_time.elapsed().as_secs_f32();

            // Check if we need to update the snapped camera
            let should_update_snap = self.pt_snapped_view_proj.is_none() || elapsed >= snap_interval;

            if should_update_snap {
                // Time to snap: check if camera actually moved since last snap
                let snapped_changed = self.pt_snapped_position.map_or(true, |prev| {
                    (prev - position).length() > 0.01
                });

                if snapped_changed {
                    tracing::info!("SNAP: camera changed, updating snap and resetting accumulation");
                    // Reset accumulation because camera position changed
                    if let Some(pt) = &mut self.path_tracer {
                        pt.reset_accumulation();
                    }
                }

                // Update snapped camera
                self.pt_snapped_view_proj = Some(view_proj);
                self.pt_snapped_view = Some(view);
                self.pt_snapped_position = Some(position);
                self.pt_camera_snap_time = std::time::Instant::now();
                self.pt_snap_ready = true;
            } else {
                // Not time for new snap yet - keep using old snapped camera
                // Still allow rendering (samples accumulate)
                self.pt_snap_ready = true;
            }

            // Use snapped camera for PT
            (
                self.pt_snapped_view_proj.unwrap_or(view_proj),
                self.pt_snapped_view.unwrap_or(view),
                self.pt_snapped_position.unwrap_or(position),
            )
        } else {
            // Snap disabled - use current camera
            self.pt_snap_ready = true;
            (view_proj, view, position)
        };

        if let Some(pt) = &mut self.path_tracer {
            let inv_view = pt_view.inverse();
            let proj = pt_view_proj * pt_view.inverse();
            let inv_proj = proj.inverse();

            let pos_arr = pt_position.to_array();
            // Compare with higher epsilon to avoid spurious resets
            // Position threshold: ~0.01 units of movement
            // ViewProj threshold: ~1% change in any matrix element
            let vp_arr = pt_view_proj.to_cols_array_2d();
            // Use very small epsilon to catch any camera movement
            const POS_EPS: f32 = 1e-5;
            const VP_EPS: f32 = 1e-6;
            let camera_changed = pt.last_camera_pos.map_or(true, |prev| {
                (prev[0] - pos_arr[0]).abs() > POS_EPS
                    || (prev[1] - pos_arr[1]).abs() > POS_EPS
                    || (prev[2] - pos_arr[2]).abs() > POS_EPS
            }) || pt.last_view_proj.map_or(true, |prev| {
                prev.iter().flatten().zip(vp_arr.iter().flatten())
                    .any(|(a, b)| (a - b).abs() > VP_EPS)
            });

            let cam = super::pathtracer::PtCameraUniform {
                inv_view: inv_view.to_cols_array_2d(),
                inv_proj: inv_proj.to_cols_array_2d(),
                position: pos_arr,
                _pad0: 0,
                frame_count: pt.frame_count,
                max_bounces: self.pt_max_bounces,
                max_transmission_depth: self.pt_max_transmission_depth,
                dof_enabled: if self.pt_dof_enabled { 1 } else { 0 },
                aperture: self.pt_aperture,
                focus_distance: self.pt_focus_distance,
                _pad1: [0; 2],
                _pad2: [0; 4],
            };
            pt.update_camera(&self.queue, &cam);

            if camera_changed {
                // Log max delta to diagnose spurious resets
                let pos_delta = pt.last_camera_pos.map_or(f32::MAX, |prev| {
                    (prev[0] - pos_arr[0]).abs()
                        .max((prev[1] - pos_arr[1]).abs())
                        .max((prev[2] - pos_arr[2]).abs())
                });
                let vp_delta = pt.last_view_proj.map_or(f32::MAX, |prev| {
                    prev.iter().flatten().zip(vp_arr.iter().flatten())
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0f32, f32::max)
                });
                tracing::warn!("PT: camera snapped, pos_delta={pos_delta:.2e}, vp_delta={vp_delta:.2e}, resetting");
                pt.reset_accumulation();
            }
            pt.last_camera_pos = Some(pos_arr);
            pt.last_view_proj = Some(vp_arr);
        }
    }

    /// Initialize the path tracer compute pipeline (lazy, on first toggle).
    /// Called from the UI when the user enables path tracing mode.
    #[allow(dead_code)]
    pub fn init_path_tracer(&mut self, width: u32, height: u32) {
        if self.path_tracer.is_some() {
            return;
        }
        self.path_tracer = Some(super::pathtracer::PathTraceCompute::new(
            &self.device, width, height, self.surface_format,
        ));
    }

    /// Build BVH from current scene meshes and upload to path tracer.
    /// Called when scene changes or path tracing is enabled.
    #[allow(dead_code)]
    #[tracing::instrument(skip_all)]
    pub fn upload_scene_to_path_tracer(&mut self) {
        self.upload_scene_to_path_tracer_impl(false, 0.0);
    }
    
    /// Upload scene to path tracer with optional smooth normals
    pub fn upload_scene_to_path_tracer_with_normals(&mut self, smooth_enabled: bool, smooth_angle: f32) {
        self.upload_scene_to_path_tracer_impl(smooth_enabled, smooth_angle);
    }
    
    fn upload_scene_to_path_tracer_impl(&mut self, smooth_enabled: bool, smooth_angle: f32) {
        use super::pathtracer::{build, gpu_data, scene_convert};

        let pt = match &mut self.path_tracer {
            Some(pt) => pt,
            None => return,
        };

        // Collect triangles and materials from all scene meshes + curves + floor
        let mut all_tris = Vec::new();
        let mut materials = Vec::new();
        // Track max object_id for visibility buffer sizing
        let mut max_object_id: u32 = 0;

        // Scene meshes
        for mesh in self.meshes.values() {
            if let (Some(base_verts), Some(indices)) = (&mesh.base_vertices, &mesh.base_indices) {
                let verts = if smooth_enabled {
                    if let Some(smooth_data) = &mesh.smooth_data {
                        let mut new_verts = base_verts.clone();
                        let smooth_normals = smooth_data.calculate(smooth_angle);
                        for (vert, normal) in new_verts.iter_mut().zip(smooth_normals.iter()) {
                            vert.normal = (*normal).into();
                        }
                        new_verts
                    } else {
                        base_verts.clone()
                    }
                } else {
                    base_verts.clone()
                };

                let mat_id = materials.len() as u32;
                materials.push(scene_convert::material_from_params(&mesh.material_params));
                let tris = scene_convert::extract_triangles(
                    &verts, indices, &mesh.transform, mat_id, mesh.object_id,
                );
                max_object_id = max_object_id.max(mesh.object_id);
                all_tris.extend(tris);
            }
        }

        // Curves (ribbon quads)
        for curve in self.curves.values() {
            if let (Some(verts), Some(indices)) = (&curve.base_vertices, &curve.base_indices) {
                let mat_id = materials.len() as u32;
                materials.push(scene_convert::material_from_params(&curve.material_params));
                // Curves use object_id 0 (no per-object visibility yet)
                let ribbon_tris = curves_to_ribbon_tris(verts, indices, &curve.transform, mat_id);
                all_tris.extend(ribbon_tris);
            }
        }

        // Floor mesh
        for mesh in self.floor_mesh.iter() {
            if let (Some(verts), Some(indices)) = (&mesh.base_vertices, &mesh.base_indices) {
                let mat_id = materials.len() as u32;
                materials.push(scene_convert::material_from_params(&mesh.material_params));
                let tris = scene_convert::extract_triangles(
                    verts, indices, &mesh.transform, mat_id, mesh.object_id,
                );
                max_object_id = max_object_id.max(mesh.object_id);
                all_tris.extend(tris);
            }
        }

        if all_tris.is_empty() {
            return;
        }
        if materials.is_empty() {
            materials.push(scene_convert::default_material());
        }

        // Build BVH
        let bvh = build::build_bvh(&all_tris);
        let gpu_data = gpu_data::build_gpu_data(&bvh, &all_tris, &materials);

        pt.upload_scene(&self.device, &self.queue, &gpu_data, max_object_id);
        
        // Also set the environment texture
        let has_env = self.env_map.intensity > 0.0;
        pt.set_environment_texture(
            &self.device,
            &self.queue,
            &self.env_map.texture,
            self.env_map.intensity,
            has_env,
        );
    }

    /// Set PT visibility for a specific object_id (no BVH rebuild).
    pub fn set_pt_object_visible(&mut self, object_id: u32, visible: bool) {
        if let Some(pt) = &mut self.path_tracer {
            pt.set_object_visible(&self.queue, object_id, visible);
        }
    }
    
    /// Update frame time for auto SPP calculation (deprecated - now measured internally)
    pub fn update_frame_time(&mut self, _frame_time_ms: f32) {
        // Now measured internally in render() for accuracy
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
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
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
    
    /// Ensure object ID texture exists and matches size
    fn ensure_object_id_texture(&mut self, width: u32, height: u32) {
        let needs_recreate = match &self.object_id_texture {
            Some(oit) => oit.size != (width, height),
            None => true,
        };

        if needs_recreate && width > 0 && height > 0 {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("object_id_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Uint,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                     | wgpu::TextureUsages::TEXTURE_BINDING 
                     | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.object_id_texture = Some(ObjectIdTexture {
                texture,
                view,
                size: (width, height),
            });
            // Invalidate hover bind group since texture changed
            self.hover_bind_group = None;
        }
    }
    
    /// Render object IDs to the object ID texture for hover detection
    /// If `clear_depth` is true, clears depth buffer (for PT mode). Otherwise loads existing depth.
    fn render_object_id_pass(&mut self, encoder: &mut wgpu::CommandEncoder, depth_view: &wgpu::TextureView, clear_depth: bool) {
        let id_texture = match &self.object_id_texture {
            Some(t) => t,
            None => return,
        };
        
        let pipeline = if self.double_sided {
            &self.pipelines.object_id_pipeline_double_sided
        } else {
            &self.pipelines.object_id_pipeline
        };
        
        let depth_load = if clear_depth {
            wgpu::LoadOp::Clear(1.0)
        } else {
            wgpu::LoadOp::Load
        };
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("object_id_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &id_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: depth_load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.camera_light_bind_group, &[]);
        
        // Render visible meshes only
        for mesh in self.meshes.values().filter(|m| m.visible) {
            render_pass.set_bind_group(1, &mesh.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, mesh.mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.mesh.index_count, 0, 0..1);
        }
        
        // Floor excluded from object ID pass: hovering floor would outline
        // the scene silhouette (floor border = mesh silhouette), not useful.
        // Floor is still pickable via CPU ray picking for selection.
    }
    
    /// Render hover highlight overlay (outline/tint)
    fn render_hover_pass(&mut self, encoder: &mut wgpu::CommandEncoder, color_view: &wgpu::TextureView, width: u32, height: u32) {
        use super::settings::HoverMode;
        
        // Skip if hover mode is disabled or no object is hovered
        if self.hover_mode == HoverMode::None || self.hovered_object_id == 0 {
            return;
        }
        
        let id_texture = match &self.object_id_texture {
            Some(t) => t,
            None => return,
        };
        
        // Create/update hover bind group if needed
        if self.hover_bind_group.is_none() {
            self.hover_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("hover_bind_group"),
                layout: &self.hover_pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&id_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.hover_pipeline.params_buffer.as_entire_binding(),
                    },
                ],
            }));
        }
        
        // Update hover params
        let mode = match self.hover_mode {
            HoverMode::None => 0,
            HoverMode::Outline => 1,
            HoverMode::Tint => 2,
            HoverMode::Both => 3,
        };
        let params = HoverParams {
            hovered_id: self.hovered_object_id,
            mode,
            outline_width: self.hover_outline_thickness,
            _pad0: 0.0,
            outline_color: [1.0, 0.5, 0.0, self.hover_outline_alpha],  // Orange with configurable alpha
            tint_color: [1.0, 0.5, 0.0, 0.12],    // Semi-transparent orange
            viewport_size: [width as f32, height as f32],
            _pad1: [0.0; 2],
        };
        self.queue.write_buffer(&self.hover_pipeline.params_buffer, 0, bytemuck::bytes_of(&params));
        
        let bind_group = self.hover_bind_group.as_ref().unwrap();
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hover_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,  // Blend over existing
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        render_pass.set_pipeline(&self.hover_pipeline.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);  // Fullscreen triangle
    }
    
    /// Request hover detection at the given pixel coordinates
    pub fn request_hover_pick(&mut self, x: u32, y: u32) {
        self.pending_hover_pick = Some((x, y));
    }
    
    /// Process pending hover pick - reads object ID at cursor position
    fn process_hover_pick(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (px, py) = match self.pending_hover_pick {
            Some(coords) => coords,
            None => return,
        };
        
        let id_texture = match &self.object_id_texture {
            Some(t) => t,
            None => return,
        };
        
        // Ensure pixel is within bounds
        if px >= id_texture.size.0 || py >= id_texture.size.1 {
            self.pending_hover_pick = None;
            return;
        }
        
        // bytes_per_row must be aligned to 256 (COPY_BYTES_PER_ROW_ALIGNMENT)
        // For R32Uint (4 bytes per pixel), we need to copy at least 64 pixels per row
        // Instead, copy the entire row and read the correct pixel
        let texture_width = id_texture.size.0;
        let bytes_per_pixel = 4u32;  // R32Uint
        let bytes_per_row = (texture_width * bytes_per_pixel + 255) & !255;  // Align to 256
        
        // Copy entire row containing our pixel
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &id_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: py, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.object_id_pick_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d { width: texture_width, height: 1, depth_or_array_layers: 1 },
        );
        // Store the x coordinate for reading the correct pixel later
        // (pending_hover_pick is not taken, it's still Some with the coords)
    }
    
    /// Read back the hovered object ID from the GPU (call after queue.submit)
    pub fn poll_hover_result(&mut self) {
        let (px, _py) = match self.pending_hover_pick.take() {
            Some(coords) => coords,
            None => return,
        };
        
        let buffer_slice = self.object_id_pick_buffer.slice(..);
        
        // Map the buffer for reading
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        
        {
            let data = buffer_slice.get_mapped_range();
            // Read the pixel at offset px * 4 (R32Uint = 4 bytes per pixel)
            let offset = (px as usize) * 4;
            if offset + 4 <= data.len() {
                let id: u32 = *bytemuck::from_bytes(&data[offset..offset + 4]);
                
                if id != self.hovered_object_id {
                    self.hovered_object_id = id;
                    self.hovered_mesh_path = self.mesh_id_map.get(&id).cloned();
                }
            }
        }
        self.object_id_pick_buffer.unmap();
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
            self.postfx_bind_groups_dirty = true;
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
            self.postfx_bind_groups_dirty = true;
        }
    }

    /// Rebuild cached post-fx bind groups (SSAO, blur, lighting).
    /// Called once per resize instead of every frame.
    fn rebuild_postfx_bind_groups(&mut self) {
        if !self.postfx_bind_groups_dirty {
            return;
        }
        let (gbuffer, ssao_targets) = match (&self.gbuffer, &self.ssao_targets) {
            (Some(gb), Some(st)) => (gb, st),
            _ => return,
        };
        let depth_view = match &self.depth_texture {
            Some(dt) => &dt.view,
            None => return,
        };

        // SSAO bind group
        self.ssao_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_bind_group"),
            layout: &self.postfx.ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.normals_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.postfx.ssao_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.ssao_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.camera_buffer.as_entire_binding(),
                },
            ],
        }));

        // SSAO blur H: input=gbuffer.occlusion, output=ssao_targets.color
        self.ssao_blur_h_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_blur_h_bind_group"),
            layout: &self.postfx.ssao_blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.postfx.ssao_blur_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.ssao_blur_h_params_buffer.as_entire_binding(),
                },
            ],
        }));

        // SSAO blur V: input=ssao_targets.color, output=gbuffer.occlusion
        self.ssao_blur_v_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_blur_v_bind_group"),
            layout: &self.postfx.ssao_blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ssao_targets.color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.postfx.ssao_blur_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.ssao_blur_v_params_buffer.as_entire_binding(),
                },
            ],
        }));

        // Lighting bind group
        self.lighting_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_bind_group"),
            layout: &self.postfx.lighting_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.normals_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.postfx.lighting_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.lighting_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.env_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&self.env_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: self.env_map.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
            ],
        }));

        self.postfx_bind_groups_dirty = false;
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
        let vertex_buffer_size = std::mem::size_of_val(vertices);
        let index_buffer_size = std::mem::size_of_val(indices);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
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

        // Assign unique object ID for hover/picking
        let object_id = self.next_object_id;
        self.next_object_id += 1;
        self.mesh_id_map.insert(object_id, name.clone());

        // Model transform with object ID
        let normal_matrix = transform.inverse().transpose();
        let model_uniform = ModelUniform {
            model: transform.to_cols_array_2d(),
            normal_matrix: normal_matrix.to_cols_array_2d(),
            object_id,
            _pad: [0; 3],
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
            object_id,
            smooth_data,
            base_vertices: Some(vertices.to_vec()),
            base_indices: Some(indices.to_vec()),
            material_params: params.clone(),
            smooth_dirty: true,
            visible: true,
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
        
        // Model transform (curves don't need object ID for hover)
        let normal_matrix = transform.inverse().transpose();
        let model_uniform = ModelUniform {
            model: transform.to_cols_array_2d(),
            normal_matrix: normal_matrix.to_cols_array_2d(),
            object_id: 0,
            _pad: [0; 3],
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
            base_vertices: Some(vertices.to_vec()),
            base_indices: Some(indices.to_vec()),
            material_params: params.clone(),
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Material
        let material_buffer = standard_surface::create_material_buffer(&self.device, params);
        let material_bind_group = standard_surface::create_material_bind_group(
            &self.device,
            &self.layouts.material,
            &material_buffer,
        );

        // Model transform (points don't need object ID for hover)
        let normal_matrix = transform.inverse().transpose();
        let model_uniform = ModelUniform {
            model: transform.to_cols_array_2d(),
            normal_matrix: normal_matrix.to_cols_array_2d(),
            object_id: 0,
            _pad: [0; 3],
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
        // Reset object ID tracking
        self.mesh_id_map.clear();
        self.next_object_id = 1;  // 0 is reserved for background
        self.hovered_object_id = 0;
        self.hovered_mesh_path = None;
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
                object_id: scene_mesh.object_id,
                _pad: [0; 3],
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
    /// Update mesh GPU buffers with pre-computed hash and bounds from worker thread.
    pub fn update_mesh_vertices(
        &mut self, name: &str, vertices: &[Vertex], indices: &[u32],
        data_hash: u64, bounds: (Vec3, Vec3),
    ) -> bool {
        let scene_mesh = match self.meshes.get_mut(name) {
            Some(m) => m,
            None => return false,
        };
        let vertex_bytes = bytemuck::cast_slice(vertices);
        let index_bytes = bytemuck::cast_slice(indices);
        if vertex_bytes.len() <= scene_mesh.mesh.vertex_buffer_size
            && index_bytes.len() <= scene_mesh.mesh.index_buffer_size
        {
            self.queue.write_buffer(&scene_mesh.mesh.vertex_buffer, 0, vertex_bytes);
            self.queue.write_buffer(&scene_mesh.mesh.index_buffer, 0, index_bytes);
            scene_mesh.mesh.index_count = indices.len() as u32;
        } else {
            let new_mesh = Self::create_mesh(&self.device, vertices, indices);
            scene_mesh.mesh = new_mesh;
        }
        scene_mesh.vertex_hash = data_hash;
        scene_mesh.bounds = bounds;
        // CPU copies for PT — only if path tracing is active
        if self.use_path_tracing {
            scene_mesh.base_vertices = Some(vertices.to_vec());
            scene_mesh.base_indices = Some(indices.to_vec());
        }
        scene_mesh.smooth_dirty = true;
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
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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
                object_id: 0,
                _pad: [0; 3],
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
    /// Update curves GPU buffers with pre-computed hash and bounds from worker thread.
    pub fn update_curves_vertices(
        &mut self, name: &str, vertices: &[Vertex], indices: &[u32],
        data_hash: u64, bounds: (Vec3, Vec3),
    ) -> bool {
        let curves = match self.curves.get_mut(name) {
            Some(c) => c,
            None => return false,
        };
        let vertex_bytes = bytemuck::cast_slice(vertices);
        let index_bytes = bytemuck::cast_slice(indices);
        if vertex_bytes.len() <= curves.mesh.vertex_buffer_size
            && index_bytes.len() <= curves.mesh.index_buffer_size
        {
            self.queue.write_buffer(&curves.mesh.vertex_buffer, 0, vertex_bytes);
            self.queue.write_buffer(&curves.mesh.index_buffer, 0, index_bytes);
            curves.mesh.index_count = indices.len() as u32;
        } else {
            let new_mesh = Self::create_mesh(&self.device, vertices, indices);
            curves.mesh = new_mesh;
        }
        curves.data_hash = data_hash;
        curves.bounds = bounds;
        curves.base_vertices = Some(vertices.to_vec());
        curves.base_indices = Some(indices.to_vec());
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
                object_id: 0,
                _pad: [0; 3],
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
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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
        self.postfx_bind_groups_dirty = true;
        
        // Update path tracer environment with texture and CDFs for importance sampling
        if let Some(pt) = &mut self.path_tracer {
            pt.set_environment_texture(
                &self.device,
                &self.queue,
                &self.env_map.texture,
                self.env_map.intensity,
                true,
            );
            // Upload CDF data for importance sampling
            pt.set_environment_cdfs(
                &self.device,
                &self.queue,
                &self.env_map.marginal_cdf_data,
                &self.env_map.conditional_cdf_data,
                self.env_map.width,
                self.env_map.height,
            );
        }
        Ok(())
    }

    /// Clear environment map (use default flat ambient)
    pub fn clear_environment(&mut self) {
        self.env_map = environment::create_default_env(
            &self.device,
            &self.queue,
            &self.layouts.environment,
        );
        self.postfx_bind_groups_dirty = true;
        
        // Update path tracer to disable environment
        if let Some(pt) = &mut self.path_tracer {
            pt.update_environment_params(&self.queue, 0.0, false);
            pt.reset_accumulation();
        }
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
        
        // Update path tracer environment intensity
        if let Some(pt) = &mut self.path_tracer {
            pt.update_environment_params(&self.queue, intensity, intensity > 0.0);
            pt.reset_accumulation();
        }
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
        
        // Assign object ID for floor
        let floor_object_id = self.next_object_id;
        self.next_object_id += 1;
        self.mesh_id_map.insert(floor_object_id, "_FLOOR_".into());
        
        // Model transform (identity) with object ID
        let model_uniform = ModelUniform {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            normal_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            object_id: floor_object_id,
            _pad: [0; 3],
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
            object_id: floor_object_id,
            smooth_data: None,
            base_vertices: Some(vertices),
            base_indices: Some(indices),
            material_params: material,
            smooth_dirty: false,
            visible: true,
        });
    }
    

    /// Render the scene
    pub fn render(&mut self, view: &wgpu::TextureView, width: u32, height: u32, camera_distance: f32, _near: f32, _far: f32) {
        let render_start = std::time::Instant::now();
        
        // Path tracing mode: dispatch compute shader and blit to screen
        if self.use_path_tracing {
            if let Some(pt) = &mut self.path_tracer {
                pt.resize(&self.device, width, height);
                pt.max_samples = self.pt_max_samples;
                
                let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("pt_encoder"),
                });
                
                // Dispatch when snap is ready (throttling is done at viewport level)
                if self.pt_snap_ready {
                    // Calculate samples per frame
                    let samples_this_frame = if self.pt_auto_spp {
                        // Auto mode: adaptive based on last render time
                        let target_frame_ms = 1000.0 / self.pt_target_fps;
                        let current_spp = self.pt_samples_per_update.max(1);
                        let time_per_sample = self.last_render_time_ms / current_spp as f32;

                        // Only adjust if we have reasonable timing data
                        if time_per_sample > 0.1 {
                            let estimated = (target_frame_ms / time_per_sample) as u32;
                            // Gradual adjustment: don't jump more than 2x
                            let max_increase = current_spp * 2;
                            let min_decrease = (current_spp / 2).max(1);
                            estimated.clamp(min_decrease, max_increase.min(128))
                        } else {
                            // Start conservative
                            current_spp.min(2)
                        }
                    } else {
                        self.pt_samples_per_update
                    };

                    // Lock camera during batch to prevent ghost samples
                    self.pt_batch_rendering = true;

                    // Measure actual render time
                    let render_start = std::time::Instant::now();

                    // Multiple samples per frame for target FPS control
                    // Each dispatch creates and submits its own encoder to ensure
                    // frame_count is synchronized (write_buffer is immediate, dispatch is deferred)
                    let mut samples_dispatched = 0u32;
                    for i in 0..samples_this_frame {
                        if !pt.dispatch(&self.device, &self.queue) {
                            tracing::debug!("PT dispatch loop: break at sample {}/{}", i, samples_this_frame);
                            break; // Scene not ready or converged
                        }
                        samples_dispatched += 1;
                    }

                    // Update samples/sec counter
                    self.pt_samples_counter += samples_dispatched;
                    if self.pt_samples_sec_start.elapsed().as_secs_f32() >= 1.0 {
                        self.pt_samples_last_sec = self.pt_samples_counter;
                        self.pt_samples_counter = 0;
                        self.pt_samples_sec_start = std::time::Instant::now();
                    }

                    // Update render time estimate (EMA)
                    let render_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;
                    self.last_render_time_ms = self.last_render_time_ms * 0.7 + render_time_ms * 0.3;

                    // Update actual samples per update for next frame estimation
                    if self.pt_auto_spp {
                        self.pt_samples_per_update = samples_this_frame;
                    }

                    // Unlock camera after batch
                    self.pt_batch_rendering = false;
                }
                // Always blit (shows accumulated result even between snap intervals)
                
                pt.blit(&mut encoder, view);
                
                // Add hover overlay in PT mode if enabled
                if self.hover_mode != super::settings::HoverMode::None {
                    self.ensure_depth_texture(width, height);
                    self.ensure_object_id_texture(width, height);
                    
                    if let Some(dt) = &self.depth_texture {
                        let depth_view = dt.view.clone();
                        // Render object IDs with depth clear (PT doesn't have depth buffer)
                        self.render_object_id_pass(&mut encoder, &depth_view, true);
                        self.process_hover_pick(&mut encoder);
                        self.render_hover_pass(&mut encoder, view, width, height);
                    }
                }
                
                self.queue.submit(std::iter::once(encoder.finish()));
            }
            return;
        }

        self.ensure_depth_texture(width, height);
        let use_gbuffer = true;
        if use_gbuffer {
            self.ensure_gbuffer(width, height);
        }
        // Ensure object ID texture for hover detection
        if self.hover_mode != super::settings::HoverMode::None {
            self.ensure_object_id_texture(width, height);
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
            let mut meshes: Vec<&SceneMesh> = self.meshes.values().filter(|m| m.visible).collect();
            if let Some(floor) = &self.floor_mesh {
                if floor.visible { meshes.push(floor); }
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
            if !mesh.visible { continue; }
            let effective_opacity = mesh.opacity * self.xray_alpha;
            if effective_opacity < opacity_threshold {
                let distance = bounds_sort_distance(mesh.bounds, self.camera_position);
                transparent_meshes.push((distance, name.clone()));
            } else {
                opaque_mesh_names.push(name.clone());
            }
        }
        if let Some(floor) = &self.floor_mesh {
            if floor.visible {
                let effective_opacity = floor.opacity * self.xray_alpha;
                if effective_opacity < opacity_threshold {
                    let distance = bounds_sort_distance(floor.bounds, self.camera_position);
                    floor_transparent_distance = Some(distance);
                } else {
                    floor_opaque = true;
                }
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
            self.rebuild_postfx_bind_groups();
            self.render_ssao_pass(&mut encoder, &depth_view, self.use_ssao);
            if self.use_ssao {
                let (ssao_temp_view, gbuffer_occlusion_view) = match (&self.ssao_targets, &self.gbuffer) {
                    (Some(targets), Some(gbuffer)) => (
                        targets.color_view.clone(),
                        gbuffer.occlusion_view.clone(),
                    ),
                    _ => return,
                };

                // H-blur: gbuffer.occlusion -> ssao_temp (cached bind group, no alloc)
                if let Some(h_bg) = &self.ssao_blur_h_bind_group {
                    self.render_ssao_blur_pass(&mut encoder, h_bg, &ssao_temp_view);
                }
                // V-blur: ssao_temp -> gbuffer.occlusion (cached bind group, no alloc)
                if let Some(v_bg) = &self.ssao_blur_v_bind_group {
                    self.render_ssao_blur_pass(&mut encoder, v_bg, &gbuffer_occlusion_view);
                }
            }

            // Update lighting params (cheap uniform write)
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
            self.render_lighting_pass(&mut encoder, color_target_view_ref);
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

        // Object ID pass for hover detection (reuse depth from main pass)
        if self.hover_mode != super::settings::HoverMode::None {
            self.render_object_id_pass(&mut encoder, &depth_view, false);
            self.process_hover_pick(&mut encoder);
            self.render_hover_pass(&mut encoder, color_target_view_ref, width, height);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Poll for hover pick result if we did a pick this frame
        if self.hover_mode != super::settings::HoverMode::None && self.pending_hover_pick.is_none() {
            // Only poll if we had a pending pick that was processed
            // (pending_hover_pick was cleared by process_hover_pick)
        }
        
        let render_ms = render_start.elapsed().as_secs_f64() * 1000.0;
        if render_ms > 16.0 {
            tracing::warn!("SLOW RENDER: {:.1}ms", render_ms);
        }
    }
}
