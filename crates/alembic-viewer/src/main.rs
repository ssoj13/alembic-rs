//! Alembic Viewer - 3D viewer for .abc files

mod app;
mod camera;
mod mesh_converter;
mod renderer;
mod viewport;

use std::path::PathBuf;
use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    
    // Get file path from command line if provided
    let initial_file: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Alembic Viewer"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Alembic Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(app::ViewerApp::new(cc, initial_file.clone())))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run: {}", e))
}
