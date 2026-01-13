//! HDR/EXR environment map loading and GPU resources

use std::path::Path;
use wgpu::util::DeviceExt;

/// Environment map data
pub struct EnvironmentMap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub intensity: f32,
}

/// Create bind group layout for environment map (group 4)
pub fn create_env_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("env_map_bind_group_layout"),
        entries: &[
            // Environment texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Environment params (intensity, rotation, etc)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
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

/// Environment uniform parameters
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EnvUniform {
    /// Environment intensity multiplier
    pub intensity: f32,
    /// Rotation offset in radians
    pub rotation: f32,
    /// Whether environment is enabled (1.0 = yes, 0.0 = no)
    pub enabled: f32,
    pub _pad: f32,
}

impl Default for EnvUniform {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            rotation: 0.0,
            enabled: 0.0, // Disabled by default
            _pad: 0.0,
        }
    }
}

/// Load HDR/EXR file using image crate
fn load_image_file(path: &Path) -> anyhow::Result<(u32, u32, Vec<f32>)> {
    use image::{GenericImageView, ImageReader};
    
    let img = ImageReader::open(path)?.decode()?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba32f();
    let data: Vec<f32> = rgba.as_raw().to_vec();
    
    Ok((width, height, data))
}

/// Load HDR/EXR image from file and create GPU texture
pub fn load_env_map(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    path: &Path,
) -> anyhow::Result<EnvironmentMap> {
    // Load image (image crate handles HDR and EXR)
    let (width, height, data) = load_image_file(path)?;
    
    let bytes: &[u8] = bytemuck::cast_slice(&data);
    
    // Create texture
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("hdr_env_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        bytes,
    );
    
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("hdr_env_sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    
    // Uniform buffer
    let uniform = EnvUniform {
        intensity: 1.0,
        rotation: 0.0,
        enabled: 1.0,
        _pad: 0.0,
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("env_uniform_buffer"),
        contents: bytemuck::bytes_of(&uniform),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("env_map_bind_group"),
        layout,
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
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });
    
    Ok(EnvironmentMap {
        texture,
        view,
        sampler,
        bind_group,
        uniform_buffer,
        intensity: 1.0,
    })
}

/// Create a default (dummy) environment - 1x1 black texture
pub fn create_default_env(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> EnvironmentMap {
    let data: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    let bytes: &[u8] = bytemuck::cast_slice(&data);
    
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("default_env_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        bytes,
    );
    
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("default_env_sampler"),
        ..Default::default()
    });
    
    let uniform = EnvUniform::default();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("env_uniform_buffer"),
        contents: bytemuck::bytes_of(&uniform),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("default_env_bind_group"),
        layout,
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
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });
    
    EnvironmentMap {
        texture,
        view,
        sampler,
        bind_group,
        uniform_buffer,
        intensity: 0.0,
    }
}
