//! Alembic Viewer - 3D viewer for .abc files

mod app;
mod camera;
mod environment;
mod mesh_converter;
mod renderer;
mod settings;
mod viewport;
mod worker;

pub use settings::Settings;
pub use app::set_log_level;

use std::path::PathBuf;
use anyhow::Result;

/// Log levels for viewer
pub const LOG_NONE: u8 = 0;
pub const LOG_INFO: u8 = 1;
pub const LOG_DEBUG: u8 = 2;
pub const LOG_TRACE: u8 = 3;

/// Run the viewer with optional initial file
pub fn run(initial_file: Option<PathBuf>) -> Result<()> {
    env_logger::init();
    

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
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([settings.window_width, settings.window_height])
            .with_title("Alembic Viewer"),
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
                        required_features: wgpu::Features::POLYGON_MODE_LINE,
                        required_limits: wgpu::Limits {
                            max_texture_dimension_2d: 8192,
                            max_bind_groups: 8,
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
        Box::new(move |cc| Ok(Box::new(app::ViewerApp::new(cc, initial_file.clone())))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run: {}", e))
}
