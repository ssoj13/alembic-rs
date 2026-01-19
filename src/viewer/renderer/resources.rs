//! GPU-side resources used by the renderer.

#[derive(Debug)]
pub struct DepthTexture {
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: (u32, u32),
}

#[derive(Debug)]
pub struct GBuffer {
    #[allow(dead_code)]
    pub albedo: wgpu::Texture,
    #[allow(dead_code)]
    pub normals: wgpu::Texture,
    #[allow(dead_code)]
    pub occlusion: wgpu::Texture,
    pub albedo_view: wgpu::TextureView,
    pub normals_view: wgpu::TextureView,
    pub occlusion_view: wgpu::TextureView,
    pub size: (u32, u32),
}

#[derive(Debug)]
pub struct SsaoTargets {
    #[allow(dead_code)]
    pub color: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub size: (u32, u32),
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoParams {
    pub strength: [f32; 4],
}

/// Lighting parameters for the fullscreen shading pass.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingParams {
    /// Background color used when no geometry is present in G-Buffer.
    pub background: [f32; 4],
}
