//! HDR/EXR environment map loading and GPU resources

use std::path::Path;
use half::f16;
use wgpu::util::DeviceExt;

/// Environment map data with importance sampling support
#[allow(dead_code)] // GPU resources held alive
pub struct EnvironmentMap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub intensity: f32,
    
    // Importance sampling data (for path tracer)
    /// Marginal CDF (1D, height entries) - probability of selecting each row
    pub marginal_cdf: wgpu::Buffer,
    /// Conditional CDFs (2D, width x height) - probability within each row
    pub conditional_cdf: wgpu::Buffer,
    /// Raw CDF data for creating new buffers
    pub marginal_cdf_data: Vec<f32>,
    pub conditional_cdf_data: Vec<f32>,
    /// Total luminance (for PDF normalization)
    pub total_luminance: f32,
    /// Dimensions for importance sampling
    pub width: u32,
    pub height: u32,
}

/// Create bind group layout for environment map (group 4)
#[allow(dead_code)] // Available for future use
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

/// Load HDR/EXR file using image crate, convert to f16
fn load_image_file(path: &Path) -> anyhow::Result<(u32, u32, Vec<f16>, Vec<f32>)> {
    use image::{GenericImageView, ImageReader};
    
    let img = ImageReader::open(path)?.decode()?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba32f();
    let raw = rgba.as_raw();
    
    // Convert f32 to f16 for filterable texture format
    let data: Vec<f16> = raw.iter().map(|&v| f16::from_f32(v)).collect();
    
    // Also keep f32 luminance for CDF building
    let luminance: Vec<f32> = raw.chunks(4)
        .map(|px| {
            // Luminance with sin(theta) weight for equirectangular projection
            let lum = 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2];
            lum
        })
        .collect();
    
    Ok((width, height, data, luminance))
}

/// Build importance sampling CDFs from luminance data.
/// Returns (conditional_cdf, marginal_cdf, total_luminance)
/// 
/// For equirectangular maps, we weight by sin(theta) to account for solid angle.
fn build_env_cdfs(width: u32, height: u32, luminance: &[f32]) -> (Vec<f32>, Vec<f32>, f32) {
    let w = width as usize;
    let h = height as usize;
    
    // Conditional CDFs: for each row, cumulative probability of each column
    let mut conditional_cdf = vec![0.0f32; w * h];
    // Row integrals (will become marginal PDF)
    let mut row_integrals = vec![0.0f32; h];
    
    for y in 0..h {
        // Sin(theta) weight for equirectangular projection
        let theta = std::f32::consts::PI * (y as f32 + 0.5) / h as f32;
        let sin_theta = theta.sin();
        
        let row_start = y * w;
        let mut row_sum = 0.0f32;
        
        // Build conditional CDF for this row
        for x in 0..w {
            let lum = luminance[row_start + x] * sin_theta;
            row_sum += lum;
            conditional_cdf[row_start + x] = row_sum;
        }
        
        // Normalize to [0, 1]
        if row_sum > 0.0 {
            for x in 0..w {
                conditional_cdf[row_start + x] /= row_sum;
            }
        }
        
        row_integrals[y] = row_sum;
    }
    
    // Marginal CDF: cumulative probability of each row
    let mut marginal_cdf = vec![0.0f32; h];
    let mut total = 0.0f32;
    for y in 0..h {
        total += row_integrals[y];
        marginal_cdf[y] = total;
    }
    
    // Normalize marginal CDF
    if total > 0.0 {
        for y in 0..h {
            marginal_cdf[y] /= total;
        }
    }
    
    (conditional_cdf, marginal_cdf, total)
}

/// Load HDR/EXR image from file and create GPU texture with importance sampling CDFs
pub fn load_env_map(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    path: &Path,
) -> anyhow::Result<EnvironmentMap> {
    // Load image (image crate handles HDR and EXR)
    let (width, height, data, luminance) = load_image_file(path)?;
    
    let bytes: &[u8] = bytemuck::cast_slice(&data);
    
    // Create texture with Rgba16Float (filterable HDR format)
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
            format: wgpu::TextureFormat::Rgba16Float,
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
    
    // Build importance sampling CDFs
    let (conditional_cdf_data, marginal_cdf_data, total_luminance) = 
        build_env_cdfs(width, height, &luminance);
    
    // Create GPU buffers for CDFs (used by path tracer)
    let conditional_cdf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("env_conditional_cdf"),
        contents: bytemuck::cast_slice(&conditional_cdf_data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    
    let marginal_cdf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("env_marginal_cdf"),
        contents: bytemuck::cast_slice(&marginal_cdf_data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    
    Ok(EnvironmentMap {
        texture,
        view,
        sampler,
        bind_group,
        uniform_buffer,
        intensity: 1.0,
        marginal_cdf,
        conditional_cdf,
        marginal_cdf_data,
        conditional_cdf_data,
        total_luminance,
        width,
        height,
    })
}

/// Create a default (dummy) environment - 1x1 black texture
pub fn create_default_env(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> EnvironmentMap {
    // Use f16 for Rgba16Float format
    let data: [f16; 4] = [f16::ZERO, f16::ZERO, f16::ZERO, f16::ONE];
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
            format: wgpu::TextureFormat::Rgba16Float,
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
    
    // Dummy CDF buffers (1x1 uniform distribution)
    let marginal_cdf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("default_marginal_cdf"),
        contents: bytemuck::cast_slice(&[1.0f32]),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let conditional_cdf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("default_conditional_cdf"),
        contents: bytemuck::cast_slice(&[1.0f32]),
        usage: wgpu::BufferUsages::STORAGE,
    });
    
    EnvironmentMap {
        texture,
        view,
        sampler,
        bind_group,
        uniform_buffer,
        intensity: 0.0,
        marginal_cdf,
        conditional_cdf,
        marginal_cdf_data: vec![1.0f32],
        conditional_cdf_data: vec![1.0f32],
        total_luminance: 0.0,
        width: 1,
        height: 1,
    }
}
