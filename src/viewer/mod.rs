//! Alembic Viewer - 3D viewer for .abc files

mod app;
mod camera;
mod environment;
pub mod export;
mod mesh_converter;
mod renderer;
mod settings;
mod smooth_normals;
mod viewport;
mod worker;

pub use settings::Settings;

use std::path::PathBuf;
use anyhow::Result;
use tracing_subscriber::prelude::*;

/// Run the viewer with optional initial file
pub fn run(initial_file: Option<PathBuf>) -> Result<()> {
    env_logger::init();
    
    let trace_guard = init_tracing();

    // Friendly panic handler for GPU errors
    std::panic::set_hook(Box::new(|info| {
        let msg = info.payload()
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("Unknown error");
        
        if msg.contains("wgpu") || msg.contains("Buffer") || msg.contains("shader") {
            eprintln!("\n[GPU Error] {}", msg);
            eprintln!("\nThis is likely a shader/buffer mismatch. Try updating or rebuilding.");
        } else {
            eprintln!("\n[Error] {}", msg);
            if let Some(loc) = info.location() {
                eprintln!("  at {}:{}:{}", loc.file(), loc.line(), loc.column());
            }
        }
    }));
    
    let settings = Settings::load();
    
    let options = eframe::NativeOptions {
        viewport: {
            let mut vp = egui::ViewportBuilder::default()
                .with_inner_size([settings.window_width, settings.window_height])
                .with_title("Alembic Viewer");
            if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
                vp = vp.with_position([x, y]);
            }
            vp
        },
        multisampling: settings.antialiasing as u16, // MSAA (0, 2, 4, 8)
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
                device_descriptor: std::sync::Arc::new(|adapter| {
                    let base_limits = if adapter.get_info().backend == wgpu::Backend::Gl {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    };
                    wgpu::DeviceDescriptor {
                        label: Some("alembic-viewer device"),
                        required_features: wgpu::Features::POLYGON_MODE_LINE
                            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                            | wgpu::Features::FLOAT32_FILTERABLE
                            | wgpu::Features::DEPTH32FLOAT_STENCIL8
                            | wgpu::Features::TEXTURE_COMPRESSION_BC
                            | wgpu::Features::PUSH_CONSTANTS
                            | wgpu::Features::MULTI_DRAW_INDIRECT_COUNT,
                        required_limits: wgpu::Limits {
                            max_texture_dimension_2d: 8192,
                            max_bind_groups: 8,
                            max_push_constant_size: 128,  // For PUSH_CONSTANTS feature
                            ..base_limits
                        },
                        ..Default::default()
                    }
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "Alembic Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(app::ViewerApp::new(cc, initial_file.clone(), trace_guard)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run: {}", e))
}

fn init_tracing() -> Option<tracing_chrome::FlushGuard> {
    if std::env::var("ALEMBIC_TRACE").ok().as_deref() != Some("1") {
        return None;
    }

    let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
        .file("trace.json")
        .build();

    let subscriber = tracing_subscriber::registry().with(chrome_layer);
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        return None;
    }

    Some(guard)
}
