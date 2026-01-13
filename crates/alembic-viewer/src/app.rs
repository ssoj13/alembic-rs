//! Main application state and UI

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use egui::{menu, Color32, RichText, TopBottomPanel, CentralPanel, SidePanel};
use glam::{Mat4, Vec3};

use standard_surface::{StandardSurfaceParams, Vertex};

use crate::mesh_converter;
use crate::settings::Settings;
use crate::viewport::Viewport;

/// Main viewer application
pub struct ViewerApp {
    viewport: Viewport,
    initialized: bool,
    settings: Settings,
    
    // File state
    current_file: Option<PathBuf>,
    pending_file: Option<PathBuf>,
    pending_hdr_file: Option<PathBuf>,
    archive: Option<Arc<alembic::abc::IArchive>>,
    
    // Animation state
    num_samples: usize,
    current_frame: usize,
    playing: bool,
    playback_fps: f32,
    last_frame_time: Instant,
    
    // UI state
    status_message: String,
    show_settings: bool,
    
    // Scene info
    mesh_count: usize,
    vertex_count: usize,
    face_count: usize,
}

impl ViewerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        let settings = Settings::load();
        
        // Use last file if no initial file provided
        let pending = initial_file.or_else(|| settings.last_file.clone());
        
        Self {
            viewport: Viewport::new(),
            initialized: false,
            settings,
            current_file: None,
            pending_file: pending,
            pending_hdr_file: None,
            archive: None,
            num_samples: 0,
            current_frame: 0,
            playing: false,
            playback_fps: 24.0,
            last_frame_time: Instant::now(),
            status_message: "Ready".into(),
            show_settings: false,
            mesh_count: 0,
            vertex_count: 0,
            face_count: 0,
        }
    }

    fn initialize(&mut self, ctx: &egui::Context) {
        if self.initialized {
            return;
        }

        // Get wgpu render state from egui
        let _render_state = ctx.input(|i| {
            i.viewport()
                .clone()
        });

        // We need to get the render state differently
        // For now, mark as initialized and we'll init the renderer later
        self.initialized = true;
        self.status_message = "Viewport ready".into();
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        // Collect recent files to avoid borrow issues
        let recent: Vec<PathBuf> = self.settings.recent_files().into_iter().cloned().collect();
        
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open...").clicked() {
                    self.open_file_dialog();
                    ui.close_menu();
                }
                
                // Recent files submenu
                if !recent.is_empty() {
                    ui.menu_button("Recent", |ui| {
                        for path in &recent {
                            let name = path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.display().to_string());
                            if ui.button(&name).clicked() {
                                self.pending_file = Some(path.clone());
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("Clear Recent").clicked() {
                            self.settings.recent_files.clear();
                            self.settings.save();
                            ui.close_menu();
                        }
                    });
                }
                
                ui.separator();
                if ui.button("Exit").clicked() {
                    std::process::exit(0);
                }
            });

            ui.menu_button("View", |ui| {
                if let Some(renderer) = &mut self.viewport.renderer {
                    if ui.checkbox(&mut renderer.show_grid, "Show Grid").changed() {
                        self.settings.show_grid = renderer.show_grid;
                        self.settings.save();
                    }
                    if ui.checkbox(&mut renderer.show_wireframe, "Wireframe").changed() {
                        self.settings.show_wireframe = renderer.show_wireframe;
                        self.settings.save();
                    }
                }
                ui.separator();
                if ui.button("Reset Camera").clicked() {
                    self.viewport.camera.reset();
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.status_message = "Alembic Viewer v0.1.0".into();
                    ui.close_menu();
                }
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scene");
        ui.separator();

        // File info
        if let Some(path) = &self.current_file {
            ui.label(format!("File: {}", path.file_name().unwrap_or_default().to_string_lossy()));
        } else {
            ui.label("No file loaded");
        }

        ui.separator();

        // Stats
        ui.label(RichText::new("Statistics").strong());
        ui.label(format!("Meshes: {}", self.mesh_count));
        ui.label(format!("Vertices: {}", self.vertex_count));
        ui.label(format!("Faces: {}", self.face_count));

        ui.separator();

        // Camera info
        ui.label(RichText::new("Camera").strong());
        let pos = self.viewport.camera.position();
        ui.label(format!("Position: ({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z));
        ui.label(format!("Distance: {:.2}", self.viewport.camera.distance()));

        ui.separator();

        // View settings
        ui.label(RichText::new("Display").strong());
        if let Some(renderer) = &mut self.viewport.renderer {
            let mut changed = false;
            
            if ui.checkbox(&mut self.settings.show_grid, "Grid").changed() {
                renderer.show_grid = self.settings.show_grid;
                changed = true;
            }
            if ui.checkbox(&mut self.settings.show_wireframe, "Wireframe").changed() {
                renderer.show_wireframe = self.settings.show_wireframe;
                changed = true;
            }
            if ui.checkbox(&mut self.settings.show_shadows, "Shadows").changed() {
                renderer.show_shadows = self.settings.show_shadows;
                changed = true;
            }
            if ui.checkbox(&mut self.settings.double_sided, "Double Sided").changed() {
                renderer.double_sided = self.settings.double_sided;
                changed = true;
            }
            if ui.checkbox(&mut self.settings.flip_normals, "Flip Normals").changed() {
                renderer.flip_normals = self.settings.flip_normals;
                renderer.update_normals();
                changed = true;
            }
            
            ui.horizontal(|ui| {
                ui.label("Background:");
                let mut color = Color32::from_rgba_unmultiplied(
                    (self.settings.background_color[0] * 255.0) as u8,
                    (self.settings.background_color[1] * 255.0) as u8,
                    (self.settings.background_color[2] * 255.0) as u8,
                    255,
                );
                if ui.color_edit_button_srgba(&mut color).changed() {
                    self.settings.background_color = [
                        color.r() as f32 / 255.0,
                        color.g() as f32 / 255.0,
                        color.b() as f32 / 255.0,
                        1.0,
                    ];
                    renderer.background_color = self.settings.background_color;
                    changed = true;
                }
            });
            
            if changed {
                self.settings.save();
            }
            
        }
        
        // Environment section (outside renderer borrow)
        ui.separator();
        ui.label(RichText::new("Environment").strong());
        
        let has_env = self.viewport.renderer.as_ref().map(|r| r.has_environment()).unwrap_or(false);
        
        // HDR enable checkbox + exposure slider
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.settings.hdr_enabled, "HDR").changed() {
                if self.settings.hdr_enabled {
                    // Try to load last HDR file if available
                    if let Some(path) = self.settings.last_hdr_file.clone() {
                        if path.exists() {
                            self.pending_hdr_file = Some(path);
                        } else {
                            self.load_environment_dialog();
                        }
                    } else {
                        self.load_environment_dialog();
                    }
                } else {
                    // Disable HDR
                    if let Some(renderer) = &mut self.viewport.renderer {
                        renderer.clear_environment();
                    }
                }
                self.settings.save();
            }
            
            // Exposure slider (only when HDR enabled)
            if self.settings.hdr_enabled {
                ui.label("Exp:");
                if ui.add(egui::Slider::new(&mut self.settings.hdr_exposure, 0.1..=10.0).logarithmic(true)).changed() {
                    // Update env intensity
                    if let Some(renderer) = &mut self.viewport.renderer {
                        renderer.set_env_intensity(self.settings.hdr_exposure);
                    }
                    self.settings.save();
                }
            }
        });
        
        if ui.button("Load HDR/EXR...").clicked() {
            self.load_environment_dialog();
        }
        
        if has_env {
            if ui.button("Clear Environment").clicked() {
                if let Some(renderer) = &mut self.viewport.renderer {
                    renderer.clear_environment();
                }
                self.settings.hdr_enabled = false;
                self.settings.save();
            }
        }

        ui.separator();

        // Actions
        if ui.button("Load Test Cube").clicked() {
            self.load_test_cube();
        }

        if ui.button("Clear Scene").clicked() {
            self.clear_scene();
        }
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status_message);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("FPS: {:.0}", ui.ctx().input(|i| 1.0 / i.stable_dt)));
            });
        });
    }
    
    fn timeline_panel(&mut self, ui: &mut egui::Ui) {
        // Only show if we have animation
        if self.num_samples <= 1 {
            return;
        }
        
        ui.horizontal(|ui| {
            // Play/Pause button
            let icon = if self.playing { "\u{23F8}" } else { "\u{25B6}" }; // ⏸ or ▶
            if ui.button(icon).clicked() {
                self.playing = !self.playing;
                self.last_frame_time = Instant::now();
            }
            
            // Stop/reset button
            if ui.button("\u{23F9}").clicked() { // ⏹
                self.playing = false;
                if self.current_frame != 0 {
                    self.load_frame(0);
                }
            }
            
            // Frame slider
            let mut frame = self.current_frame as f32;
            let max_frame = (self.num_samples - 1) as f32;
            
            if ui.add(
                egui::Slider::new(&mut frame, 0.0..=max_frame)
                    .step_by(1.0)
                    .show_value(false)
            ).changed() {
                let new_frame = frame as usize;
                if new_frame != self.current_frame {
                    self.load_frame(new_frame);
                }
            }
            
            // Frame counter
            ui.label(format!("{} / {}", self.current_frame + 1, self.num_samples));
            
            ui.separator();
            
            // FPS selector
            ui.label("FPS:");
            egui::ComboBox::from_id_salt("fps_select")
                .selected_text(format!("{:.0}", self.playback_fps))
                .width(50.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.playback_fps, 12.0, "12");
                    ui.selectable_value(&mut self.playback_fps, 24.0, "24");
                    ui.selectable_value(&mut self.playback_fps, 30.0, "30");
                    ui.selectable_value(&mut self.playback_fps, 60.0, "60");
                });
        });
    }
    
    fn update_animation(&mut self) {
        if !self.playing || self.num_samples <= 1 {
            return;
        }
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time).as_secs_f32();
        let frame_duration = 1.0 / self.playback_fps;
        
        if elapsed >= frame_duration {
            self.last_frame_time = now;
            
            let next_frame = (self.current_frame + 1) % self.num_samples;
            self.load_frame(next_frame);
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Alembic", &["abc"])
            .pick_file()
        {
            self.load_file(path);
        }
    }

    fn load_environment_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("HDR/EXR", &["hdr", "exr"])
            .pick_file()
        {
            self.load_environment(path);
        }
    }

    fn load_environment(&mut self, path: PathBuf) {
        let renderer = match &mut self.viewport.renderer {
            Some(r) => r,
            None => {
                self.status_message = "Renderer not initialized".into();
                return;
            }
        };
        
        match renderer.load_environment(&path) {
            Ok(()) => {
                self.status_message = format!("Loaded environment: {}", 
                    path.file_name().unwrap_or_default().to_string_lossy());
                // Save HDR file and enable
                self.settings.last_hdr_file = Some(path);
                self.settings.hdr_enabled = true;
                self.settings.save();
            }
            Err(e) => {
                self.status_message = format!("Failed to load environment: {}", e);
                self.settings.hdr_enabled = false;
                self.settings.save();
            }
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        self.status_message = format!("Loading: {}", path.display());
        
        if self.viewport.renderer.is_none() {
            self.status_message = "Renderer not initialized".into();
            return;
        }
        
        match alembic::abc::IArchive::open(&path) {
            Ok(archive) => {
                // Detect animation - find max samples across all meshes
                let num_samples = Self::detect_num_samples(&archive);
                
                // Store archive for animation playback
                self.archive = Some(Arc::new(archive));
                self.num_samples = num_samples;
                self.current_frame = 0;
                self.playing = false;
                
                // Load frame 0
                self.load_frame(0);
                
                self.current_file = Some(path.clone());
                
                // Add to recent files
                self.settings.add_recent(path.clone());
                self.settings.save();
                
                let frames_info = if num_samples > 1 {
                    format!(", {} frames", num_samples)
                } else {
                    String::new()
                };
                
                self.status_message = format!(
                    "Loaded: {} meshes, {} vertices, {} triangles{}",
                    self.mesh_count, self.vertex_count, self.face_count, frames_info
                );
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }
    
    /// Detect maximum number of samples in archive
    fn detect_num_samples(archive: &alembic::abc::IArchive) -> usize {
        let root = archive.root();
        Self::detect_num_samples_recursive(&root, 1)
    }
    
    fn detect_num_samples_recursive(obj: &alembic::abc::IObject, max: usize) -> usize {
        let mut current_max = max;
        
        // Check if PolyMesh
        if let Some(polymesh) = alembic::geom::IPolyMesh::new(obj) {
            current_max = current_max.max(polymesh.num_samples());
        }
        
        // Check if Xform
        if let Some(xform) = alembic::geom::IXform::new(obj) {
            current_max = current_max.max(xform.num_samples());
        }
        
        // Recurse children
        for child in obj.children() {
            current_max = Self::detect_num_samples_recursive(&child, current_max);
        }
        
        current_max
    }
    
    /// Load meshes for a specific frame
    fn load_frame(&mut self, frame: usize) {
        let archive = match &self.archive {
            Some(a) => a.clone(),
            None => return,
        };
        
        let renderer = match &mut self.viewport.renderer {
            Some(r) => r,
            None => return,
        };
        
        // Clear existing meshes
        renderer.clear_meshes();
        
        // Collect and convert all meshes at this frame
        let meshes = mesh_converter::collect_meshes(&archive, frame);
        let stats = mesh_converter::compute_stats(&meshes);
        
        // Add meshes to renderer
        for mesh in meshes {
            let material = StandardSurfaceParams::plastic(
                Vec3::new(0.7, 0.7, 0.75),
                0.4,
            );
            renderer.add_mesh(
                mesh.name,
                &mesh.vertices,
                &mesh.indices,
                mesh.transform,
                &material,
            );
        }
        
        // Update stats
        self.mesh_count = stats.mesh_count;
        self.vertex_count = stats.vertex_count;
        self.face_count = stats.triangle_count;
        self.current_frame = frame;
    }

    fn load_test_cube(&mut self) {
        let renderer = match &mut self.viewport.renderer {
            Some(r) => r,
            None => {
                self.status_message = "Renderer not initialized".into();
                return;
            }
        };

        // Simple cube vertices
        let vertices = [
            // Front face
            Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
            Vertex { position: [0.5, 0.5, 0.5], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
            Vertex { position: [-0.5, 0.5, 0.5], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
            // Back face
            Vertex { position: [0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
            Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
            Vertex { position: [0.5, 0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
            // Top face
            Vertex { position: [-0.5, 0.5, 0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
            Vertex { position: [0.5, 0.5, 0.5], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
            Vertex { position: [0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
            Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
            // Bottom face
            Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0] },
            Vertex { position: [0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], uv: [1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.5], normal: [0.0, -1.0, 0.0], uv: [1.0, 1.0] },
            Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, -1.0, 0.0], uv: [0.0, 1.0] },
            // Right face
            Vertex { position: [0.5, -0.5, 0.5], normal: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
            Vertex { position: [0.5, -0.5, -0.5], normal: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },
            Vertex { position: [0.5, 0.5, -0.5], normal: [1.0, 0.0, 0.0], uv: [1.0, 1.0] },
            Vertex { position: [0.5, 0.5, 0.5], normal: [1.0, 0.0, 0.0], uv: [0.0, 1.0] },
            // Left face
            Vertex { position: [-0.5, -0.5, -0.5], normal: [-1.0, 0.0, 0.0], uv: [0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.5], normal: [-1.0, 0.0, 0.0], uv: [1.0, 0.0] },
            Vertex { position: [-0.5, 0.5, 0.5], normal: [-1.0, 0.0, 0.0], uv: [1.0, 1.0] },
            Vertex { position: [-0.5, 0.5, -0.5], normal: [-1.0, 0.0, 0.0], uv: [0.0, 1.0] },
        ];

        let indices: Vec<u32> = (0..6)
            .flat_map(|face| {
                let base = face * 4;
                [base, base + 1, base + 2, base, base + 2, base + 3]
            })
            .collect();

        let material = StandardSurfaceParams::plastic(Vec3::new(0.8, 0.2, 0.2), 0.3);

        renderer.add_mesh(
            "TestCube".into(),
            &vertices,
            &indices,
            Mat4::IDENTITY,
            &material,
        );

        self.mesh_count = renderer.meshes.len();
        self.vertex_count = vertices.len();
        self.face_count = indices.len() / 3;
        self.status_message = "Loaded test cube".into();
    }

    fn clear_scene(&mut self) {
        if let Some(renderer) = &mut self.viewport.renderer {
            renderer.clear_meshes();
        }
        self.mesh_count = 0;
        self.vertex_count = 0;
        self.face_count = 0;
        self.current_file = None;
        self.archive = None;
        self.num_samples = 0;
        self.current_frame = 0;
        self.playing = false;
        self.status_message = "Scene cleared".into();
    }
}

impl eframe::App for ViewerApp {
    fn on_exit(&mut self) {
        // Save camera state
        self.settings.camera_distance = self.viewport.camera.distance();
        let (yaw, pitch) = self.viewport.camera.angles();
        self.settings.camera_yaw = yaw;
        self.settings.camera_pitch = pitch;
        self.settings.save();
    }
    
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        
        self.initialize(ctx);

        // Initialize renderer if needed
        if self.viewport.renderer.is_none() {
            if let Some(render_state) = frame.wgpu_render_state() {
                self.viewport.init_renderer(
                    &render_state.device,
                    &render_state.queue,
                    render_state.target_format,
                );
                // Apply saved settings to renderer
                if let Some(renderer) = &mut self.viewport.renderer {
                    renderer.show_grid = self.settings.show_grid;
                    renderer.show_wireframe = self.settings.show_wireframe;
                    renderer.show_shadows = self.settings.show_shadows;
                    renderer.double_sided = self.settings.double_sided;
                    renderer.flip_normals = self.settings.flip_normals;
                    renderer.background_color = self.settings.background_color;
                }
                // Ensure settings file exists
                self.settings.save();
                // Apply saved camera settings
                self.viewport.camera.set_distance(self.settings.camera_distance);
                self.viewport.camera.set_angles(self.settings.camera_yaw, self.settings.camera_pitch);
                // Restore HDR if was enabled
                if self.settings.hdr_enabled {
                    if let Some(path) = self.settings.last_hdr_file.clone() {
                        if path.exists() {
                            self.pending_hdr_file = Some(path);
                        }
                    }
                }
            }
        }
        
        // Load pending file (from CLI argument or recent)
        if self.viewport.renderer.is_some() {
            if let Some(path) = self.pending_file.take() {
                self.load_file(path);
            }
            // Load pending HDR file
            if let Some(path) = self.pending_hdr_file.take() {
                self.load_environment(path);
            }
        }
        
        // Update animation playback
        self.update_animation();

        // Top menu bar
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        // Timeline (above status bar)
        if self.num_samples > 1 {
            TopBottomPanel::bottom("timeline")
                .resizable(false)
                .show(ctx, |ui| {
                    self.timeline_panel(ui);
                });
        }
        
        // Bottom status bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.status_bar(ui);
        });

        // Right side panel
        SidePanel::right("side_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.side_panel(ui);
            });

        // Central viewport
        CentralPanel::default().show(ctx, |ui| {
            let render_state = frame.wgpu_render_state();
            self.viewport.show(ui, render_state);
        });

        // Track window size for saving on exit
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.settings.window_width = rect.width();
                self.settings.window_height = rect.height();
            }
        });
        
        // Request continuous repaint for smooth camera animation
        ctx.request_repaint();
    }
}
