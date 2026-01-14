//! Main application state and UI

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Format FPS for display (hide decimals for whole numbers)
fn format_fps(fps: f32) -> String {
    if (fps - fps.round()).abs() < 0.001 {
        format!("{:.0}", fps)
    } else {
        format!("{:.3}", fps).trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

use egui::{Color32, RichText, TopBottomPanel, CentralPanel, SidePanel};
use glam::{Mat4, Vec3};

use standard_surface::{StandardSurfaceParams, Vertex};

use super::mesh_converter;
use super::settings::Settings;
use super::viewport::Viewport;

/// Scene hierarchy node
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub name: String,
    pub node_type: String,
    pub children: Vec<SceneNode>,
}

impl SceneNode {
    pub fn new(name: &str, node_type: &str) -> Self {
        Self {
            name: name.to_string(),
            node_type: node_type.to_string(),
            children: Vec::new(),
        }
    }
}

/// Main viewer application
pub struct ViewerApp {
    viewport: Viewport,
    initialized: bool,
    settings: Settings,
    
    // File state
    current_file: Option<PathBuf>,
    pending_file: Option<PathBuf>,
    pending_hdr_file: Option<PathBuf>,
    archive: Option<Arc<crate::abc::IArchive>>,
    
    // Animation state
    num_samples: usize,
    current_frame: usize,
    playing: bool,
    playback_fps: f32,
    playback_dir: i32, // 1 = forward, -1 = backward
    last_frame_time: Instant,
    
    // UI state
    status_message: String,
    
    // Scene info
    mesh_count: usize,
    vertex_count: usize,
    face_count: usize,
    scene_bounds: Option<mesh_converter::Bounds>,
    scene_tree: Vec<SceneNode>,
    selected_object: Option<String>,
    
    // Scene cameras
    scene_cameras: Vec<mesh_converter::SceneCamera>,
    active_camera: Option<usize>,  // None = orbit camera, Some(i) = scene camera index

    // Scene lights (for potential lighting override)
    scene_lights: Vec<mesh_converter::SceneLight>,

    // Async loading
    worker: Option<super::worker::WorkerHandle>,
    pending_frame: Option<usize>,  // Frame we've requested but not yet received
    epoch: u64,  // Incremented on each request, used to discard stale results
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
            playback_dir: 1,
            last_frame_time: Instant::now(),
            status_message: "Ready".into(),
            mesh_count: 0,
            vertex_count: 0,
            face_count: 0,
            scene_bounds: None,
            scene_tree: Vec::new(),
            selected_object: None,
            scene_cameras: Vec::new(),
            active_camera: None,
            scene_lights: Vec::new(),
            worker: None,
            pending_frame: None,
            epoch: 0,
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
        
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open...").clicked() {
                    self.open_file_dialog();
                    ui.close();
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
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Clear Recent").clicked() {
                            self.settings.recent_files.clear();
                            self.settings.save();
                            ui.close();
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
                    self.active_camera = None;
                    ui.close();
                }
                
                // Scene cameras submenu
                if !self.scene_cameras.is_empty() {
                    ui.separator();
                    ui.menu_button("Scene Cameras", |ui| {
                        // Orbit camera (default)
                        if ui.selectable_label(self.active_camera.is_none(), "Orbit Camera").clicked() {
                            self.active_camera = None;
                            ui.close();
                        }
                        ui.separator();
                        // Scene cameras
                        for (i, cam) in self.scene_cameras.iter().enumerate() {
                            let is_active = self.active_camera == Some(i);
                            let label = format!("{} ({:.0}mm)", cam.name, cam.focal_length);
                            if ui.selectable_label(is_active, &label).clicked() {
                                self.active_camera = Some(i);
                                ui.close();
                            }
                        }
                    });
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.status_message = "Alembic Viewer v0.1.0".into();
                    ui.close();
                }
            });
        });
    }

    /// Hierarchy panel - object tree
    fn hierarchy_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Hierarchy");
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            let tree = std::mem::take(&mut self.scene_tree);
            let mut selected = self.selected_object.clone();
            for node in &tree {
                Self::show_tree_node(ui, node, &mut selected);
            }
            self.selected_object = selected;
            self.scene_tree = tree;
        });
    }
    
    /// Render a tree node recursively
    fn show_tree_node(ui: &mut egui::Ui, node: &SceneNode, selected: &mut Option<String>) {
        let id = ui.make_persistent_id(&node.name);
        let is_selected = selected.as_ref() == Some(&node.name);
        
        // Icon based on type
        let icon = match node.node_type.as_str() {
            "PolyMesh" => "▲",  // triangle
            "SubD" => "■",      // square
            "Xform" => "↺",     // rotation arrow
            "Camera" => "◎",    // target
            "Light" => "☀",     // sun
            "Curves" => "∿",    // curve
            "Points" => "•",    // bullet
            _ => "○",           // circle
        };
        
        let label = format!("{} {}", icon, node.name);
        
        if node.children.is_empty() {
            // Leaf node
            let response = ui.selectable_label(is_selected, &label);
            if response.clicked() {
                *selected = Some(node.name.clone());
            }
        } else {
            // Parent node with children
            egui::CollapsingHeader::new(&label)
                .id_salt(id)
                .default_open(true)
                .show(ui, |ui| {
                    for child in &node.children {
                        Self::show_tree_node(ui, child, selected);
                    }
                });
        }
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
        
        // Selected object properties
        if let Some(name) = &self.selected_object {
            ui.label(RichText::new("Selected").strong());
            ui.label(format!("Name: {}", name));
            
            // Find node type
            if let Some(node) = self.find_node_by_name(name) {
                ui.label(format!("Type: {}", node.node_type));
            }
            
            // Get object properties from archive
            if let Some(archive) = &self.archive {
                self.show_object_properties_by_name(ui, archive, name);
            }
            
            ui.separator();
        }

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
            if ui.checkbox(&mut self.settings.flat_shading, "Flat Shading").changed() {
                renderer.flat_shading = self.settings.flat_shading;
                changed = true;
            }
            if ui.checkbox(&mut self.settings.show_shadows, "Shadows").changed() {
                renderer.show_shadows = self.settings.show_shadows;
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                if ui.add(egui::Slider::new(&mut self.settings.xray_alpha, 0.01..=1.0).step_by(0.01)).changed() {
                    renderer.xray_alpha = self.settings.xray_alpha;
                    changed = true;
                }
            });
            if ui.checkbox(&mut self.settings.double_sided, "Double Sided").changed() {
                renderer.double_sided = self.settings.double_sided;
                changed = true;
            }
            if ui.checkbox(&mut self.settings.flip_normals, "Flip Normals").changed() {
                renderer.flip_normals = self.settings.flip_normals;
                renderer.update_normals();
                changed = true;
            }
            
            // Anti-aliasing (requires restart to take effect)
            ui.horizontal(|ui| {
                ui.label("AA:");
                let aa_changed = egui::ComboBox::from_id_salt("aa_combo")
                    .width(50.0)
                    .selected_text(format!("{}x", self.settings.antialiasing))
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for val in [0u8, 2, 4, 8] {
                            let label = if val == 0 { "Off".to_string() } else { format!("{}x", val) };
                            if ui.selectable_value(&mut self.settings.antialiasing, val, label).changed() {
                                changed = true;
                            }
                        }
                        changed
                    }).inner.unwrap_or(false);
                if aa_changed {
                    self.settings.save();
                }
                ui.label("(restart)").on_hover_text("Requires restart to apply");
            });
            
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
        
        // HDR visibility (show background sphere but keep reflections)
        if has_env && ui.checkbox(&mut self.settings.hdr_visible, "Show Background").changed() {
            if let Some(renderer) = &mut self.viewport.renderer {
                renderer.hdr_visible = self.settings.hdr_visible;
            }
            self.settings.save();
        }
        
        if ui.button("Load HDR/EXR...").clicked() {
            self.load_environment_dialog();
        }
        
        if has_env
            && ui.button("Clear Environment").clicked() {
                if let Some(renderer) = &mut self.viewport.renderer {
                    renderer.clear_environment();
                }
                self.settings.hdr_enabled = false;
                self.settings.save();
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
        let has_animation = self.num_samples > 1;
        
        ui.horizontal(|ui| {
            // Play/Pause button (disabled for static files)
            let icon = if self.playing { "⏸" } else { "▶" };
            if ui.add_enabled(has_animation, egui::Button::new(icon)).clicked() {
                self.playing = !self.playing;
                self.last_frame_time = Instant::now();
            }
            
            // Stop/reset button
            if ui.add_enabled(has_animation, egui::Button::new("⏹")).clicked() {
                self.playing = false;
                if self.current_frame != 0 {
                    self.request_frame(0);
                }
            }
            
            // Frame counter (left side)
            ui.label(format!("{} / {}", self.current_frame + 1, self.num_samples.max(1)));
            
            // FPS selector
            if has_animation {
                ui.separator();
                ui.label("FPS:");
                egui::ComboBox::from_id_salt("fps_select")
                    .selected_text(format_fps(self.playback_fps))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        // Film/Cinema
                        ui.selectable_value(&mut self.playback_fps, 23.976, "23.976 (Film)");
                        ui.selectable_value(&mut self.playback_fps, 24.0, "24 (Cinema)");
                        ui.selectable_value(&mut self.playback_fps, 48.0, "48 (HFR)");
                        ui.separator();
                        // TV PAL (Europe)
                        ui.selectable_value(&mut self.playback_fps, 25.0, "25 (PAL)");
                        ui.selectable_value(&mut self.playback_fps, 50.0, "50 (PAL HD)");
                        ui.separator();
                        // TV NTSC (US/Japan)
                        ui.selectable_value(&mut self.playback_fps, 29.97, "29.97 (NTSC)");
                        ui.selectable_value(&mut self.playback_fps, 30.0, "30");
                        ui.selectable_value(&mut self.playback_fps, 59.94, "59.94 (NTSC HD)");
                        ui.selectable_value(&mut self.playback_fps, 60.0, "60");
                        ui.separator();
                        // Animation
                        ui.selectable_value(&mut self.playback_fps, 12.0, "12 (Animation)");
                        ui.selectable_value(&mut self.playback_fps, 15.0, "15");
                    });
            }
            
            ui.separator();
            
            // Frame slider - fill remaining width
            let mut frame = self.current_frame as f32;
            let max_frame = (self.num_samples.max(1) - 1) as f32;
            let slider_width = ui.available_width() - 10.0;
            
            ui.spacing_mut().slider_width = slider_width.max(100.0);
            let response = ui.add_enabled(
                has_animation,
                egui::Slider::new(&mut frame, 0.0..=max_frame.max(1.0))
                    .step_by(1.0)
                    .show_value(false)
            );
            // Update immediately during drag for responsive feedback
            if response.changed() || response.dragged() {
                let new_frame = frame as usize;
                if new_frame != self.current_frame {
                    self.current_frame = new_frame;  // Instant visual update
                    self.request_frame(new_frame);   // Async load
                }
            }
        });
    }
    
    fn update_animation(&mut self) {
        if !self.playing || self.num_samples <= 1 {
            return;
        }
        
        // Wait for previous frame to finish before requesting next
        // This prevents epoch mismatch during normal playback
        if self.pending_frame.is_some() {
            return;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time).as_secs_f32();
        let frame_duration = 1.0 / self.playback_fps;

        if elapsed >= frame_duration {
            self.last_frame_time = now;

            // Calculate next frame with direction and looping
            let n = self.num_samples as i32;
            let next = (self.current_frame as i32 + self.playback_dir).rem_euclid(n) as usize;
            self.request_frame(next);
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
        
        match crate::abc::IArchive::open(&path) {
            Ok(archive) => {
                // Detect animation - find max samples across all meshes
                let num_samples = Self::detect_num_samples(&archive);
                
                // Build scene hierarchy tree
                self.scene_tree = Self::build_scene_tree(&archive);
                self.selected_object = None;
                
                // Store archive for animation playback
                let archive = Arc::new(archive);
                self.archive = Some(archive.clone());
                self.num_samples = num_samples;
                self.current_frame = 0;
                self.playing = false;
                
                // Spawn background worker for async frame loading
                self.worker = Some(super::worker::WorkerHandle::spawn(archive));
                self.pending_frame = None;
                
                // Request frame 0
                self.request_frame(0);
                
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
    fn detect_num_samples(archive: &crate::abc::IArchive) -> usize {
        let root = archive.root();
        Self::detect_num_samples_recursive(&root, 1)
    }
    
    fn detect_num_samples_recursive(obj: &crate::abc::IObject, max: usize) -> usize {
        let mut current_max = max;
        
        // Check ALL geometry schemas
        if let Some(g) = crate::geom::IPolyMesh::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::ISubD::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::ICurves::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::IPoints::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::INuPatch::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::IXform::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::ICamera::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::ILight::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        if let Some(g) = crate::geom::IFaceSet::new(obj) {
            current_max = current_max.max(g.num_samples());
        }
        
        // Recurse children
        for child in obj.children() {
            current_max = Self::detect_num_samples_recursive(&child, current_max);
        }
        
        current_max
    }
    
    /// Build scene hierarchy tree from archive
    fn build_scene_tree(archive: &crate::abc::IArchive) -> Vec<SceneNode> {
        let root = archive.root();
        let mut children = Vec::new();
        for child in root.children() {
            children.push(Self::build_scene_node(&child));
        }
        children
    }
    
    fn build_scene_node(obj: &crate::abc::IObject) -> SceneNode {
        let name = obj.name();
        
        // Detect object type
        let node_type = if crate::geom::IPolyMesh::new(obj).is_some() {
            "PolyMesh"
        } else if crate::geom::IXform::new(obj).is_some() {
            "Xform"
        } else if crate::geom::ICamera::new(obj).is_some() {
            "Camera"
        } else if crate::geom::ILight::new(obj).is_some() {
            "Light"
        } else if crate::geom::ICurves::new(obj).is_some() {
            "Curves"
        } else if crate::geom::IPoints::new(obj).is_some() {
            "Points"
        } else if crate::geom::ISubD::new(obj).is_some() {
            "SubD"
        } else {
            "Object"
        };
        
        let mut node = SceneNode::new(name, node_type);
        
        for child in obj.children() {
            node.children.push(Self::build_scene_node(&child));
        }
        
        node
    }
    
    /// Find node by name in scene tree
    fn find_node_by_name(&self, name: &str) -> Option<SceneNode> {
        Self::find_node_recursive(&self.scene_tree, name)
    }
    
    fn find_node_recursive(nodes: &[SceneNode], name: &str) -> Option<SceneNode> {
        for node in nodes {
            if node.name == name {
                return Some(node.clone());
            }
            if let Some(found) = Self::find_node_recursive(&node.children, name) {
                return Some(found);
            }
        }
        None
    }
    
    /// Show object properties by searching archive
    fn show_object_properties_by_name(&self, ui: &mut egui::Ui, archive: &crate::abc::IArchive, name: &str) {
        let root = archive.root();
        Self::show_props_recursive(ui, &root, name, self.current_frame);
    }
    
    fn show_props_recursive(ui: &mut egui::Ui, obj: &crate::abc::IObject, name: &str, frame: usize) -> bool {
        if obj.name() == name {
            // Found the object - show its properties
            if let Some(mesh) = crate::geom::IPolyMesh::new(obj) {
                ui.label(format!("Samples: {}", mesh.num_samples()));
                if let Ok(sample) = mesh.get_sample(frame) {
                    ui.label(format!("Vertices: {}", sample.positions.len()));
                    ui.label(format!("Faces: {}", sample.face_counts.len()));
                }
            } else if let Some(xform) = crate::geom::IXform::new(obj) {
                ui.label(format!("Samples: {}", xform.num_samples()));
                if let Ok(sample) = xform.get_sample(frame) {
                    let matrix = sample.matrix();
                    let (_, rot, trans) = matrix.to_scale_rotation_translation();
                    ui.label(format!("Pos: ({:.2}, {:.2}, {:.2})", trans.x, trans.y, trans.z));
                    let euler: (f32, f32, f32) = rot.to_euler(glam::EulerRot::XYZ);
                    ui.label(format!("Rot: ({:.1}°, {:.1}°, {:.1}°)", 
                        euler.0.to_degrees(), euler.1.to_degrees(), euler.2.to_degrees()));
                }
            } else if let Some(cam) = crate::geom::ICamera::new(obj) {
                ui.label(format!("Samples: {}", cam.num_samples()));
                if let Ok(sample) = cam.get_sample(frame) {
                    ui.label(format!("Focal: {:.1}mm", sample.focal_length));
                    ui.label(format!("Aperture: {:.1}mm", sample.horizontal_aperture));
                }
            } else if let Some(curves) = crate::geom::ICurves::new(obj) {
                ui.label(format!("Samples: {}", curves.num_samples()));
            } else if let Some(points) = crate::geom::IPoints::new(obj) {
                ui.label(format!("Samples: {}", points.num_samples()));
            }
            return true;
        }
        
        // Recurse into children
        for child in obj.children() {
            if Self::show_props_recursive(ui, &child, name, frame) {
                return true;
            }
        }
        false
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
        // Stop worker first
        if let Some(mut worker) = self.worker.take() {
            worker.stop();
        }
        self.pending_frame = None;
        
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
    
    /// Request a frame to be loaded asynchronously.
    fn request_frame(&mut self, frame: usize) {
        if let Some(worker) = &self.worker {
            // Increment epoch on every request - this allows us to discard stale results
            self.epoch = self.epoch.wrapping_add(1);
            worker.request_frame(frame, self.epoch);
            self.pending_frame = Some(frame);
        }
    }
    
    /// Process any ready results from the worker (non-blocking).
    fn process_worker_results(&mut self) {
        let result = match &self.worker {
            Some(worker) => worker.try_recv(),
            None => return,
        };
        
        if let Some(result) = result {
            match result {
                super::worker::WorkerResult::FrameReady { frame, epoch, scene } => {
                    // Discard stale results from older requests
                    if epoch != self.epoch {
                        return;
                    }
                    
                    self.pending_frame = None;
                    self.apply_scene(frame, scene);
                }
            }
        }
    }
    
    /// Apply scene data to renderer (called when worker delivers results).
    fn apply_scene(&mut self, frame: usize, scene: mesh_converter::CollectedScene) {
        let renderer = match &mut self.viewport.renderer {
            Some(r) => r,
            None => return,
        };

        let stats = mesh_converter::compute_stats(&scene.meshes);
        let bounds = mesh_converter::compute_scene_bounds(&scene.meshes, &scene.points);
        self.scene_bounds = if bounds.is_valid() { Some(bounds) } else { None };
        
        // Update scene cameras (only on first frame or when cameras change)
        if !scene.cameras.is_empty() && self.scene_cameras.is_empty() {
            self.scene_cameras = scene.cameras;
        }

        // Update scene lights
        if !scene.lights.is_empty() && self.scene_lights.is_empty() {
            self.scene_lights = scene.lights;
        }

        // Collect names using references (no String cloning)
        let new_mesh_names: std::collections::HashSet<&str> = 
            scene.meshes.iter().map(|m| m.name.as_str()).collect();
        let new_curve_names: std::collections::HashSet<&str> = 
            scene.curves.iter().map(|c| c.name.as_str()).collect();
        
        // Remove meshes that no longer exist (retain avoids intermediate Vec)
        renderer.meshes.retain(|name, _| new_mesh_names.contains(name.as_str()));
        renderer.curves.retain(|name, _| new_curve_names.contains(name.as_str()));

        // Update or add meshes
        for mesh in scene.meshes {
            if renderer.has_mesh(&mesh.name) {
                // Always update transform (cheap uniform write)
                renderer.update_mesh_transform(&mesh.name, mesh.transform);
                
                // Only update vertices if they actually changed (expensive buffer recreation)
                let new_hash = super::renderer::compute_vertex_hash(&mesh.vertices);
                if let Some(old_hash) = renderer.get_vertex_hash(&mesh.name) {
                    if new_hash != old_hash {
                        renderer.update_mesh_vertices(&mesh.name, &mesh.vertices, &mesh.indices);
                    }
                }
            } else {
                // Use material base_color if available, otherwise default gray
                let base_color = mesh.base_color.unwrap_or(Vec3::new(0.7, 0.7, 0.75));
                let material = StandardSurfaceParams::plastic(base_color, 0.4);
                renderer.add_mesh(
                    mesh.name,
                    &mesh.vertices,  // Arc<Vec> derefs to &[T]
                    &mesh.indices,
                    mesh.transform,
                    &material,
                );
            }
        }

        // Update or add curves
        let curves_material = StandardSurfaceParams::plastic(
            Vec3::new(0.9, 0.7, 0.3),
            0.3,
        );
        for curves in scene.curves {
            renderer.add_curves(
                curves.name,
                &curves.vertices,
                &curves.indices,
                curves.transform,
                &curves_material,
            );
        }

        // Update or add points
        let points_material = StandardSurfaceParams::plastic(
            Vec3::new(0.3, 0.8, 0.9),  // cyan-ish color for points
            0.5,
        );
        for pts in scene.points {
            renderer.add_points(
                pts.name,
                &pts.positions,
                pts.transform,
                &points_material,
            );
        }

        // Update stats
        self.mesh_count = stats.mesh_count;
        self.vertex_count = stats.vertex_count;
        self.face_count = stats.triangle_count;
        self.current_frame = frame;
    }
    
    /// Find next or previous ABC file in the same directory
    /// direction: -1 for previous, +1 for next
    fn find_sibling_abc(&self, direction: i32) -> Option<PathBuf> {
        let current = self.current_file.as_ref()?;
        let dir = current.parent()?;
        
        // Collect all .abc files in directory
        let mut abc_files: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|ext| ext.eq_ignore_ascii_case("abc"))
                    .unwrap_or(false)
            })
            .collect();
        
        // Sort alphabetically
        abc_files.sort();
        
        if abc_files.is_empty() {
            return None;
        }
        
        // Find current file index
        let current_idx = abc_files.iter().position(|p| p == current)?;
        
        // Calculate new index with wrapping
        let new_idx = if direction > 0 {
            (current_idx + 1) % abc_files.len()
        } else if current_idx == 0 {
            abc_files.len() - 1
        } else {
            current_idx - 1
        };
        
        // Don't return same file
        if new_idx == current_idx {
            return None;
        }
        
        Some(abc_files[new_idx].clone())
    }
    
    /// Navigate to next or previous ABC file in directory
    fn navigate_sibling(&mut self, direction: i32) {
        if let Some(path) = self.find_sibling_abc(direction) {
            let filename = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            self.status_message = format!("Opening: {}", filename);
            self.pending_file = Some(path);
        } else {
            self.status_message = "No other ABC files in directory".into();
        }
    }
}

impl eframe::App for ViewerApp {
    fn on_exit(&mut self) {
        // Stop worker to clear message queue
        if let Some(mut worker) = self.worker.take() {
            worker.stop();
        }
        
        // Save camera state
        self.settings.camera_distance = self.viewport.camera.distance();
        let (yaw, pitch) = self.viewport.camera.angles();
        self.settings.camera_yaw = yaw;
        self.settings.camera_pitch = pitch;
        self.settings.save();
    }
    
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Update scene camera override
        self.viewport.scene_camera = self.active_camera.and_then(|i| {
            self.scene_cameras.get(i).map(|cam| {
                // Build view matrix from camera transform
                // Camera transform is world-to-local, we need to invert it for view
                let view = cam.transform.inverse();
                super::viewport::SceneCameraOverride {
                    view,
                    fov_y: cam.fov_y(),
                    near: cam.near,
                    far: cam.far,
                }
            })
        });
        
        // Process any ready frames from background worker (non-blocking)
        self.process_worker_results();
        
        // Close on Escape - stop worker first to clear queue
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if let Some(mut worker) = self.worker.take() {
                worker.stop();
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        
        // Navigate ABC files in directory: PageUp = prev, PageDown = next
        if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
            self.navigate_sibling(-1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
            self.navigate_sibling(1);
        }
        
        // H = Home camera (reset to default view)
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.viewport.camera.reset();
            self.status_message = "Camera reset".into();
        }
        
        // F = Fit view (focus on scene bounds)
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            if let Some(bounds) = &self.scene_bounds {
                self.viewport.camera.focus(bounds.center(), bounds.radius().max(0.1));
                self.status_message = format!("Fit to scene (radius: {:.2})", bounds.radius());
            } else {
                self.viewport.camera.focus(glam::Vec3::ZERO, 5.0);
                self.status_message = "No scene bounds".into();
            }
        }

        // Playback controls (unified)
        // Space/Up = play/pause toggle
        let toggle_play = ctx.input(|i| {
            i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::ArrowUp)
        });
        if toggle_play && self.num_samples > 1 {
            self.playing = !self.playing;
        }

        // Left/Right = frame step + set playback direction
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && self.num_samples > 1 {
            self.playing = false;
            self.playback_dir = -1;
            let prev = if self.current_frame == 0 { self.num_samples - 1 } else { self.current_frame - 1 };
            self.request_frame(prev);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) && self.num_samples > 1 {
            self.playing = false;
            self.playback_dir = 1;
            let next = (self.current_frame + 1) % self.num_samples;
            self.request_frame(next);
        }
        // Down = go to first frame
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && self.num_samples > 0 {
            self.playing = false;
            self.request_frame(0);
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
                    renderer.flat_shading = self.settings.flat_shading;
                    renderer.show_shadows = self.settings.show_shadows;
                    renderer.hdr_visible = self.settings.hdr_visible;
                    renderer.xray_alpha = self.settings.xray_alpha;
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

        // Timeline (above status bar) - always visible
        TopBottomPanel::bottom("timeline")
            .resizable(false)
            .show(ctx, |ui| {
                self.timeline_panel(ui);
            });
        
        // Bottom status bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.status_bar(ui);
        });

        // Left panel - object hierarchy
        if !self.scene_tree.is_empty() {
            SidePanel::left("hierarchy_panel")
                .default_width(200.0)
                .show(ctx, |ui| {
                    self.hierarchy_panel(ui);
                });
        }

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
