//! Scene rendering pipelines (opaque, wireframe, gbuffer, shadow).

use standard_surface::{BindGroupLayouts, PipelineConfig};

pub struct Pipelines {
    pub depth_prepass_pipeline: wgpu::RenderPipeline,
    pub depth_prepass_pipeline_double_sided: wgpu::RenderPipeline,
    pub wireframe_pipeline: wgpu::RenderPipeline,
    pub wireframe_pipeline_double_sided: wgpu::RenderPipeline,
    pub gbuffer_pipeline: wgpu::RenderPipeline,
    pub gbuffer_pipeline_double_sided: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub point_pipeline: wgpu::RenderPipeline,
    pub shadow_pipeline: wgpu::RenderPipeline,
}

pub fn create_pipelines(
    device: &wgpu::Device,
    layouts: &BindGroupLayouts,
    format: wgpu::TextureFormat,
) -> Pipelines {
    let config = PipelineConfig {
        label: Some("opaque_pipeline"),
        format,
        depth_format: Some(wgpu::TextureFormat::Depth32Float),
        blend: false,  // Opaque pass should not blend
        depth_equal: true, // Color pass runs after GBuffer, allow equal depth
        cull_mode: Some(wgpu::Face::Back),
        wireframe: false,
        ..Default::default()
    };
    let wireframe_config = PipelineConfig {
        label: Some("wireframe_pipeline"),
        wireframe: true,
        ..config.clone()
    };
    let wireframe_pipeline = standard_surface::create_pipeline(device, layouts, &wireframe_config);

    let double_sided_config = PipelineConfig {
        label: Some("opaque_pipeline_double_sided"),
        cull_mode: None,
        ..config.clone()
    };
    let depth_prepass_config = PipelineConfig {
        label: Some("depth_prepass_pipeline"),
        depth_only: true,
        depth_write: true,
        ..config.clone()
    };
    let depth_prepass_pipeline = standard_surface::create_pipeline(device, layouts, &depth_prepass_config);

    let depth_prepass_double_sided_config = PipelineConfig {
        label: Some("depth_prepass_pipeline_double_sided"),
        depth_only: true,
        depth_write: true,
        cull_mode: None,
        ..config.clone()
    };
    let depth_prepass_pipeline_double_sided =
        standard_surface::create_pipeline(device, layouts, &depth_prepass_double_sided_config);

    let wireframe_double_sided_config = PipelineConfig {
        label: Some("wireframe_pipeline_double_sided"),
        wireframe: true,
        cull_mode: None,
        ..double_sided_config.clone()
    };
    let wireframe_pipeline_double_sided = standard_surface::create_pipeline(device, layouts, &wireframe_double_sided_config);

    let gbuffer_formats = vec![
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::R8Unorm,
    ];
    let gbuffer_config = PipelineConfig {
        label: Some("gbuffer_pipeline"),
        blend: false,
        fragment_entry: Some("fs_gbuffer"),
        color_formats: Some(gbuffer_formats.clone()),
        ..config.clone()
    };
    let gbuffer_pipeline = standard_surface::create_pipeline(device, layouts, &gbuffer_config);

    let gbuffer_double_sided_config = PipelineConfig {
        label: Some("gbuffer_pipeline_double_sided"),
        blend: false,
        fragment_entry: Some("fs_gbuffer"),
        color_formats: Some(gbuffer_formats),
        cull_mode: None,
        ..config.clone()
    };
    let gbuffer_pipeline_double_sided =
        standard_surface::create_pipeline(device, layouts, &gbuffer_double_sided_config);

    let line_config = PipelineConfig {
        label: Some("line_pipeline"),
        topology: wgpu::PrimitiveTopology::LineList,
        cull_mode: None,
        ..double_sided_config.clone()
    };
    let line_pipeline = standard_surface::create_pipeline(device, layouts, &line_config);

    let point_config = PipelineConfig {
        label: Some("point_pipeline"),
        topology: wgpu::PrimitiveTopology::PointList,
        cull_mode: None,
        ..double_sided_config
    };
    let point_pipeline = standard_surface::create_pipeline(device, layouts, &point_config);

    let shadow_pipeline = standard_surface::create_shadow_pipeline(device, layouts);

    Pipelines {
        depth_prepass_pipeline,
        depth_prepass_pipeline_double_sided,
        wireframe_pipeline,
        wireframe_pipeline_double_sided,
        gbuffer_pipeline,
        gbuffer_pipeline_double_sided,
        line_pipeline,
        point_pipeline,
        shadow_pipeline,
    }
}
