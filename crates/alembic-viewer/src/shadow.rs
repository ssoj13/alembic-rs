//! Shadow mapping implementation

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Shadow map configuration
pub const SHADOW_MAP_SIZE: u32 = 2048;
pub const SHADOW_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Shadow map resources
pub struct ShadowMap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    /// Light's view-projection matrix
    pub light_vp_buffer: wgpu::Buffer,
}

/// Light space matrix uniform
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniform {
    pub light_view_proj: [[f32; 4]; 4],
}

impl ShadowMap {
    pub fn new(
        device: &wgpu::Device,
        shadow_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Create shadow depth texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_map_texture"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Comparison sampler for shadow testing
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        // Light VP matrix buffer
        let light_vp = ShadowUniform {
            light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let light_vp_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_light_vp_buffer"),
            contents: bytemuck::bytes_of(&light_vp),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_bind_group"),
            layout: shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light_vp_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group,
            light_vp_buffer,
        }
    }

    /// Update light view-projection matrix
    pub fn update_light_matrix(
        &self,
        queue: &wgpu::Queue,
        light_dir: Vec3,
        scene_center: Vec3,
        scene_radius: f32,
    ) {
        // Create orthographic projection for directional light
        let light_pos = scene_center - light_dir.normalize() * scene_radius * 2.0;
        
        let view = Mat4::look_at_rh(light_pos, scene_center, Vec3::Y);
        
        // Orthographic projection covering the scene
        let proj = Mat4::orthographic_rh(
            -scene_radius, scene_radius,
            -scene_radius, scene_radius,
            0.1, scene_radius * 4.0,
        );
        
        let light_vp = proj * view;
        
        let uniform = ShadowUniform {
            light_view_proj: light_vp.to_cols_array_2d(),
        };
        queue.write_buffer(&self.light_vp_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}

/// Create bind group layout for shadow sampling
pub fn create_shadow_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow_bind_group_layout"),
        entries: &[
            // Shadow map texture (depth comparison)
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
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// WGSL code for shadow sampling
pub const SHADOW_WGSL: &str = r#"
// Shadow map sampling with PCF (percentage closer filtering)
fn sample_shadow(
    world_pos: vec3<f32>,
    light_vp: mat4x4<f32>,
    shadow_map: texture_depth_2d,
    shadow_sampler: sampler_comparison,
) -> f32 {
    // Transform to light space
    let light_space = light_vp * vec4<f32>(world_pos, 1.0);
    let proj_coords = light_space.xyz / light_space.w;
    
    // Transform to [0, 1] range for texture sampling
    let shadow_uv = proj_coords.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    let current_depth = proj_coords.z;
    
    // Check if outside shadow map
    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 {
        return 1.0; // No shadow outside map
    }
    
    // PCF - sample multiple points for soft shadows
    let texel_size = 1.0 / 2048.0; // SHADOW_MAP_SIZE
    var shadow = 0.0;
    
    // 3x3 PCF kernel
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                shadow_uv + offset,
                current_depth - 0.002 // Bias to reduce shadow acne
            );
        }
    }
    
    return shadow / 9.0;
}
"#;
