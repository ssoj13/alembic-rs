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
pub mod pathtracer;

pub use settings::Settings;

use std::path::PathBuf;
use anyhow::Result;
use tracing_subscriber::prelude::*;

/// Run the viewer with optional initial file.
/// `verbosity`: 0=warn, 1=info, 2=debug, 3=trace.
/// `log_file`: optional path to redirect log output.
pub fn run(initial_file: Option<PathBuf>, verbosity: u8, log_file: Option<PathBuf>) -> Result<()> {
    let trace_guard = init_tracing(verbosity, log_file.as_deref());

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
                    // Request optional features when adapter supports them
                    let supported = adapter.features();
                    let mut features = wgpu::Features::POLYGON_MODE_LINE;
                    // Needed for 8x MSAA on some formats
                    if supported.contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES) {
                        features |= wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
                    }
                    // Needed for filtering Rgba32Float textures (path tracer blit)
                    if supported.contains(wgpu::Features::FLOAT32_FILTERABLE) {
                        features |= wgpu::Features::FLOAT32_FILTERABLE;
                    }
                    wgpu::DeviceDescriptor {
                        label: Some("alembic-viewer device"),
                        required_features: features,
                        required_limits: wgpu::Limits {
                            max_texture_dimension_2d: 8192,
                            max_bind_groups: 8,
                            max_storage_buffer_binding_size: 512 * 1024 * 1024, // 512MB for large PT scenes
                            max_buffer_size: 512 * 1024 * 1024,
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

/// Initialize tracing subscriber with console/file output and optional chrome profiler.
/// Returns chrome flush guard if ALEMBIC_TRACE=1 is set.
fn init_tracing(verbosity: u8, log_file: Option<&std::path::Path>) -> Option<tracing_chrome::FlushGuard> {
    use tracing_subscriber::{fmt, EnvFilter};

    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    // Chrome profiler layer (optional, via ALEMBIC_TRACE=1)
    let chrome_guard = if std::env::var("ALEMBIC_TRACE").ok().as_deref() == Some("1") {
        let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
            .file("trace.json")
            .build();
        // Build with chrome layer + fmt
        if let Some(path) = log_file {
            let file = std::fs::File::create(path).expect("Failed to create log file");
            let file_layer = fmt::layer()
                .with_writer(file)
                .with_ansi(false);
            let subscriber = tracing_subscriber::registry()
                .with(filter)
                .with(chrome_layer)
                .with(file_layer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        } else {
            let stderr_layer = fmt::layer()
                .with_writer(std::io::stderr);
            let subscriber = tracing_subscriber::registry()
                .with(filter)
                .with(chrome_layer)
                .with(stderr_layer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        Some(guard)
    } else {
        // No chrome profiler, just fmt output
        if let Some(path) = log_file {
            let file = std::fs::File::create(path).expect("Failed to create log file");
            let file_layer = fmt::layer()
                .with_writer(file)
                .with_ansi(false);
            let subscriber = tracing_subscriber::registry()
                .with(filter)
                .with(file_layer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        } else {
            let stderr_layer = fmt::layer()
                .with_writer(std::io::stderr);
            let subscriber = tracing_subscriber::registry()
                .with(filter)
                .with(stderr_layer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        None
    };

    chrome_guard
}
