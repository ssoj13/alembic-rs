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
    /// Whether to draw the HDR skybox as background (1.0 = yes, 0.0 = no).
    pub hdr_visible: f32,
    /// Padding to 16-byte alignment after hdr_visible.
    pub _pad0: [f32; 3],
    /// Pad to match WGSL uniform layout (vec3 occupies 16 bytes).
    pub _pad1: [f32; 4],
}

/// Parameters for separable SSAO blur.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoBlurParams {
    pub direction: [f32; 2],
    pub _pad: [f32; 2],
}
