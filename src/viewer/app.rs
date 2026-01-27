//! Main application state and UI

use std::collections::HashSet;
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
    playback_dir: i32, // 1 = forward, -1 = backward
    last_frame_time: Instant,
    scene_is_static: bool,
    last_scene_hash: Option<u64>,
    
    // UI state
    status_message: String,
    
    // Scene info
    mesh_count: usize,
    vertex_count: usize,
    face_count: usize,
    scene_bounds: Option<mesh_converter::Bounds>,
    scene_tree: Vec<SceneNode>,
    selected_object: Option<String>,
    object_filter: String,  // Wildcard filter for hierarchy (e.g., "wheel*")
    expanded_nodes: HashSet<String>,  // Track expanded tree nodes (Shift+click = recursive)

    // Scene cameras
    scene_cameras: Vec<mesh_converter::SceneCamera>,
    active_camera: Option<usize>,  // None = orbit camera, Some(i) = scene camera index

    // Scene lights (for potential lighting override)
    scene_lights: Vec<mesh_converter::SceneLight>,

    // Async loading
    worker: Option<super::worker::WorkerHandle>,
    pending_frame: Option<usize>,  // Frame we've requested but not yet received
    epoch: u64,  // Incremented on each request, used to discard stale results
    is_fullscreen: bool,
    _trace_guard: Option<tracing_chrome::FlushGuard>,
}

impl ViewerApp {
    fn hash_scene_object(path: &str, transform: Mat4, data_hash: u64) -> u64 {
        let mut hasher = spooky_hash::SpookyHash::new(0, 0);
        hasher.update(path.as_bytes());
        let cols = transform.to_cols_array();
        hasher.update(bytemuck::cast_slice(&cols));
        hasher.update(&data_hash.to_le_bytes());
        let (h1, h2) = hasher.finalize();
        h1 ^ h2
    }

    fn compute_scene_hash(scene: &mesh_converter::CollectedScene) -> u64 {
        let mut acc = 0u64;
        for mesh in &scene.meshes {
            let data_hash = super::renderer::compute_mesh_hash(&mesh.vertices, &mesh.indices);
            acc ^= Self::hash_scene_object(&mesh.path, mesh.transform, data_hash);
        }
        for curves in &scene.curves {
            let data_hash = super::renderer::compute_curves_hash(&curves.vertices, &curves.indices);
            acc ^= Self::hash_scene_object(&curves.path, curves.transform, data_hash);
        }
        for pts in &scene.points {
            let data_hash = super::renderer::compute_points_hash(&pts.positions, &pts.widths);
            acc ^= Self::hash_scene_object(&pts.path, pts.transform, data_hash);
        }
        acc
    }
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        initial_file: Option<PathBuf>,
        trace_guard: Option<tracing_chrome::FlushGuard>,
    ) -> Self {
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
            playback_dir: 1,
            last_frame_time: Instant::now(),
            scene_is_static: false,
            last_scene_hash: None,
            status_message: "Ready".into(),
            mesh_count: 0,
            vertex_count: 0,
            face_count: 0,
            scene_bounds: None,
            scene_tree: Vec::new(),
            selected_object: None,
            object_filter: String::new(),
            expanded_nodes: HashSet::new(),
            scene_cameras: Vec::new(),
            active_camera: None,
            scene_lights: Vec::new(),
            worker: None,
            pending_frame: None,
            epoch: 0,
            is_fullscreen: false,
            _trace_guard: trace_guard,
        }
    }

    fn initialize(&mut self, ctx: &egui::Context) {
        if self.initialized {
            return;
        }

        // Load custom font with good Unicode support
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "noto_sans".to_owned(),
            egui::FontData::from_static(include_bytes!("../../assets/NotoSans-Regular.ttf")).into(),
        );
        // Use Noto Sans as primary font
        fonts.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "noto_sans".to_owned());
        fonts.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("noto_sans".to_owned());  // fallback for monospace
        ctx.set_fonts(fonts);

        self.initialized = true;
        self.status_message = "Viewport ready".into();
    }

    fn menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Collect recent files to avoid borrow issues
        let recent: Vec<PathBuf> = self.settings.recent_files().into_iter().cloned().collect();
        
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open...").clicked() {
                    self.open_file_dialog();
                    ui.close();
                }
                
                // Export As... (only enabled when file is loaded)
                let has_file = self.current_file.is_some();
                if ui.add_enabled(has_file, egui::Button::new("Export As...")).clicked() {
                    self.export_file_dialog();
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
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("View", |ui| {
                if let Some(renderer) = &mut self.viewport.renderer {
                    if ui.checkbox(&mut self.settings.show_grid, "Show Grid").changed() {
                        renderer.show_grid = self.settings.show_grid;
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.show_wireframe, "Wireframe").changed() {
                        renderer.show_wireframe = self.settings.show_wireframe;
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

        // Filter input
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(egui::TextEdit::singleline(&mut self.object_filter)
                .hint_text("wheel*")
                .desired_width(ui.available_width()));
        });
        if !self.object_filter.is_empty()
            && ui.small_button("✕ Clear").clicked() {
                self.object_filter.clear();
            }
        ui.separator();

        let filter = self.object_filter.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let tree = std::mem::take(&mut self.scene_tree);
            let mut selected = self.selected_object.clone();
            let mut expanded = std::mem::take(&mut self.expanded_nodes);
            for node in &tree {
                Self::show_tree_node(ui, node, &mut selected, &filter, &mut expanded, 0);
            }
            self.selected_object = selected;
            self.expanded_nodes = expanded;
            self.scene_tree = tree;
        });
    }
    
    /// Check if name matches wildcard filter (e.g., "wheel*" matches "wheel_lb")
    fn matches_filter(name: &str, filter: &str) -> bool {
        if filter.is_empty() {
            return true;
        }
        let name_lower = name.to_lowercase();
        // Split by * for wildcard matching
        let parts: Vec<&str> = filter.split('*').collect();
        
        if parts.len() == 1 {
            // No wildcard - exact substring match
            return name_lower.contains(filter);
        }
        
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if let Some(found) = name_lower[pos..].find(part) {
                if i == 0 && found != 0 {
                    // First part must match at start (no leading *)
                    return false;
                }
                pos += found + part.len();
            } else {
                return false;
            }
        }
        // If filter ends with *, any trailing chars are OK
        // If not, must match to end
        if !filter.ends_with('*') && pos != name_lower.len() {
            return false;
        }
        true
    }
    
    /// Check if node or any descendant matches filter
    fn node_matches_filter(node: &SceneNode, filter: &str) -> bool {
        if Self::matches_filter(&node.name, filter) {
            return true;
        }
        for child in &node.children {
            if Self::node_matches_filter(child, filter) {
                return true;
            }
        }
        false
    }
    
    /// Render tree node with custom expand state (supports Shift+click for recursive toggle)
    fn show_tree_node(
        ui: &mut egui::Ui,
        node: &SceneNode,
        selected: &mut Option<String>,
        filter: &str,
        expanded: &mut HashSet<String>,
        depth: usize,
    ) {
        // Skip nodes that don't match filter
        if !filter.is_empty() && !Self::node_matches_filter(node, filter) {
            return;
        }
        
        let is_selected = selected.as_ref() == Some(&node.name);
        let matches_directly = Self::matches_filter(&node.name, filter);
        let has_children = !node.children.is_empty();
        let is_expanded = expanded.contains(&node.name);
        
        // Icon based on type
        let icon = match node.node_type.as_str() {
            "PolyMesh" => "▲",  // triangle
            "SubD" => "■",      // square
            "Xform" => "↺",     // rotation
            "Camera" => "◎",    // target
            "Light" => "☀",     // sun
            "Curves" => "∿",    // wave
            "Points" => "•",    // bullet
            _ => "○",           // circle
        };
        
        // Arrow for expandable nodes
        let arrow = if has_children {
            if is_expanded { "▼" } else { "▶" }
        } else {
            "  " // spacing for leaf nodes
        };
        
        let label_text = format!("{} {} {}", arrow, icon, node.name);
        let label = if !filter.is_empty() && matches_directly {
            RichText::new(label_text).color(Color32::YELLOW)
        } else if is_selected {
            RichText::new(label_text).color(Color32::LIGHT_BLUE)
        } else {
            RichText::new(label_text)
        };
        
        // Indent based on depth
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            let response = ui.selectable_label(is_selected, label);
            
            if response.clicked() {
                // Select the node
                *selected = Some(node.name.clone());
                
                // Toggle expand state for nodes with children
                if has_children {
                    let shift = ui.input(|i| i.modifiers.shift);
                    if shift {
                        // Recursive toggle
                        Self::toggle_recursive(node, expanded, !is_expanded);
                    } else {
                        // Single toggle
                        if is_expanded {
                            expanded.remove(&node.name);
                        } else {
                            expanded.insert(node.name.clone());
                        }
                    }
                }
            }
        });
        
        // Show children if expanded
        if has_children && is_expanded {
            for child in &node.children {
                Self::show_tree_node(ui, child, selected, filter, expanded, depth + 1);
            }
        }
    }
    
    /// Recursively set expand state for node and all descendants
    fn toggle_recursive(node: &SceneNode, expanded: &mut HashSet<String>, expand: bool) {
        if expand {
            expanded.insert(node.name.clone());
        } else {
            expanded.remove(&node.name);
        }
        for child in &node.children {
            Self::toggle_recursive(child, expanded, expand);
        }
    }
    
    /// Render tree node with filtering (old version, kept for reference)
    #[allow(dead_code)]
    fn show_tree_node_filtered(ui: &mut egui::Ui, node: &SceneNode, selected: &mut Option<String>, filter: &str) {
        // Skip nodes that don't match filter (and have no matching descendants)
        if !filter.is_empty() && !Self::node_matches_filter(node, filter) {
            return;
        }
        
        let id = ui.make_persistent_id(&node.name);
        let is_selected = selected.as_ref() == Some(&node.name);
        let matches_directly = Self::matches_filter(&node.name, filter);
        
        // Icon based on type
        let icon = match node.node_type.as_str() {
            "PolyMesh" => "▲",
            "SubD" => "■",
            "Xform" => "↺",
            "Camera" => "◎",
            "Light" => "☀",
            "Curves" => "∿",
            "Points" => "•",
            _ => "○",
        };
        
        let label = format!("{} {}", icon, node.name);
        // Highlight matching nodes
        let label = if !filter.is_empty() && matches_directly {
            RichText::new(label).color(Color32::YELLOW)
        } else {
            RichText::new(label)
        };
        
        if node.children.is_empty() {
            let response = ui.selectable_label(is_selected, label);
            if response.clicked() {
                *selected = Some(node.name.clone());
            }
        } else {
            egui::CollapsingHeader::new(label)
                .id_salt(id)
                .default_open(!filter.is_empty())
                .show(ui, |ui| {
                    for child in &node.children {
                        Self::show_tree_node_filtered(ui, child, selected, filter);
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
            if ui.checkbox(&mut self.settings.ssao, "SSAO").changed() {
                renderer.use_ssao = self.settings.ssao;
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("SSAO:");
                if ui.add(egui::Slider::new(&mut self.settings.ssao_strength, 0.0..=2.0).step_by(0.05)).changed() {
                    renderer.ssao_strength = self.settings.ssao_strength;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(egui::Slider::new(&mut self.settings.ssao_radius, 0.005..=0.1).step_by(0.005)).changed() {
                    renderer.ssao_radius = self.settings.ssao_radius;
                    changed = true;
                }
            });
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
            if ui.checkbox(&mut self.settings.auto_normals, "Auto Normals").changed() {
                renderer.auto_normals = self.settings.auto_normals;
                changed = true;
            }
            
            // Smooth normals - recalculate dynamically
            ui.horizontal(|ui| {
                let checkbox_changed = ui.checkbox(&mut self.settings.smooth_normals, "SmoothN").changed();
                let slider_changed = ui.add(egui::Slider::new(&mut self.settings.smooth_angle, 0.0..=180.0)
                    .suffix("\u{00b0}")
                    .fixed_decimals(0)).changed();
                if checkbox_changed || slider_changed {
                    renderer.recalculate_smooth_normals(
                        self.settings.smooth_angle,
                        self.settings.smooth_normals,
                        true,
                    );
                    changed = true;
                }
            });
            
            ui.separator();
            // Path tracing toggle
            if ui.checkbox(&mut self.settings.path_tracing, "Path Tracing").changed() {
                renderer.use_path_tracing = self.settings.path_tracing;
                if self.settings.path_tracing {
                    // Lazy-init path tracer on first enable
                    renderer.init_path_tracer(1280, 720);
                    renderer.upload_scene_to_path_tracer();
                }
                changed = true;
            }
            ui.separator();
            
            // Anti-aliasing (requires restart to take effect)
            ui.horizontal(|ui| {
                ui.label("AA:");
                let aa_changed = egui::ComboBox::from_id_salt("aa_combo")
                    .width(50.0)
                    .selected_text(format!("{}x", self.settings.antialiasing))
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for val in [1u8, 2, 4, 8] {
                            let label = format!("{}x", val);
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
        
        // Lighting section
        ui.separator();
        ui.label(RichText::new("Lighting").strong());
        
        let has_scene_lights = !self.scene_lights.is_empty();
        ui.add_enabled_ui(has_scene_lights, |ui| {
            let label = if has_scene_lights {
                format!("Scene Lights ({})", self.scene_lights.len())
            } else {
                "Scene Lights (none)".to_string()
            };
            if ui.checkbox(&mut self.settings.use_scene_lights, label).changed() {
                if let Some(renderer) = &self.viewport.renderer {
                    if self.settings.use_scene_lights && has_scene_lights {
                        renderer.set_scene_lights(&self.scene_lights);
                    } else {
                        renderer.set_default_lights();
                    }
                }
                self.settings.save();
            }
        });
        if !has_scene_lights {
            ui.label("(Default 3-point lighting)");
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
        
        // Grid
        if ui.checkbox(&mut self.settings.show_grid, "Grid").changed() {
            if let Some(renderer) = &mut self.viewport.renderer {
                renderer.show_grid = self.settings.show_grid;
            }
            self.settings.save();
        }
        
        // Floor plane - checkbox directly controls floor mesh existence
        if ui.checkbox(&mut self.settings.show_floor, "Floor").changed() {
            if let Some(renderer) = &mut self.viewport.renderer {
                if self.settings.show_floor {
                    renderer.set_floor(&self.scene_bounds);
                } else {
                    renderer.clear_floor();
                }
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
                let prev_fps = self.settings.playback_fps;
                egui::ComboBox::from_id_salt("fps_select")
                    .selected_text(format_fps(self.settings.playback_fps))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        // Film/Cinema
                        ui.selectable_value(&mut self.settings.playback_fps, 23.976, "23.976 (Film)");
                        ui.selectable_value(&mut self.settings.playback_fps, 24.0, "24 (Cinema)");
                        ui.selectable_value(&mut self.settings.playback_fps, 48.0, "48 (HFR)");
                        ui.separator();
                        // TV PAL (Europe)
                        ui.selectable_value(&mut self.settings.playback_fps, 25.0, "25 (PAL)");
                        ui.selectable_value(&mut self.settings.playback_fps, 50.0, "50 (PAL HD)");
                        ui.separator();
                        // TV NTSC (US/Japan)
                        ui.selectable_value(&mut self.settings.playback_fps, 29.97, "29.97 (NTSC)");
                        ui.selectable_value(&mut self.settings.playback_fps, 30.0, "30");
                        ui.selectable_value(&mut self.settings.playback_fps, 59.94, "59.94 (NTSC HD)");
                        ui.selectable_value(&mut self.settings.playback_fps, 60.0, "60");
                        ui.separator();
                        // Animation
                        ui.selectable_value(&mut self.settings.playback_fps, 12.0, "12 (Animation)");
                        ui.selectable_value(&mut self.settings.playback_fps, 15.0, "15");
                    });
                if self.settings.playback_fps != prev_fps {
                    self.settings.save();
                }
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
            
            // Draw keyframe markers under slider if animated
            if has_animation && max_frame > 0.0 {
                let rect = response.rect;
                let painter = ui.painter();
                let marker_y = rect.max.y + 2.0;
                
                // Draw tick marks for keyframes (every 10 frames, or every frame if < 20)
                let step = if self.num_samples > 20 { 10 } else { 1 };
                for i in (0..self.num_samples).step_by(step) {
                    let t = i as f32 / max_frame;
                    let x = rect.min.x + t * rect.width();
                    let color = if i == self.current_frame {
                        egui::Color32::from_rgb(100, 200, 100)  // Current frame - green
                    } else {
                        egui::Color32::from_rgb(100, 100, 100)  // Other frames - gray
                    };
                    painter.line_segment(
                        [egui::pos2(x, marker_y), egui::pos2(x, marker_y + 4.0)],
                        egui::Stroke::new(1.0, color),
                    );
                }
                
                // Highlight current frame marker
                let t = self.current_frame as f32 / max_frame;
                let x = rect.min.x + t * rect.width();
                painter.line_segment(
                    [egui::pos2(x, marker_y), egui::pos2(x, marker_y + 6.0)],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 50)),
                );
            }
        });
    }
    
    fn update_animation(&mut self) {
        let _span = tracing::info_span!("update_animation").entered();
        if self.scene_is_static {
            return;
        }
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
        let frame_duration = 1.0 / self.settings.playback_fps;

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
    
    /// Export current archive to a new file using the Rust writer
    fn export_file_dialog(&mut self) {
        let archive = match &self.archive {
            Some(a) => a.clone(),
            None => {
                self.status_message = "No file loaded to export".into();
                return;
            }
        };
        
        // Suggest output filename based on input
        let default_name = self.current_file
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}_export.abc", s.to_string_lossy()))
            .unwrap_or_else(|| "export.abc".to_string());
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Alembic", &["abc"])
            .set_file_name(&default_name)
            .save_file()
        {
            self.status_message = "Exporting...".into();
            
            match super::export::export_archive(&archive, &path) {
                Ok(stats) => {
                    self.status_message = format!(
                        "Exported {} objects to {}",
                        stats.total(),
                        path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
                    );
                }
                Err(e) => {
                    self.status_message = format!("Export failed: {}", e);
                }
            }
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
        let root = archive.getTop();
        Self::detect_num_samples_recursive(&root, 1)
    }
    
    fn detect_num_samples_recursive(obj: &crate::abc::IObject, max: usize) -> usize {
        let mut current_max = max;
        
        // Check ALL geometry schemas
        if let Some(g) = crate::geom::IPolyMesh::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::ISubD::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::ICurves::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::IPoints::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::INuPatch::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::IXform::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::ICamera::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::ILight::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        if let Some(g) = crate::geom::IFaceSet::new(obj) {
            current_max = current_max.max(g.getNumSamples());
        }
        
        // Recurse children
        for child in obj.getChildren() {
            current_max = Self::detect_num_samples_recursive(&child, current_max);
        }
        
        current_max
    }
    
    /// Build scene hierarchy tree from archive
    fn build_scene_tree(archive: &crate::abc::IArchive) -> Vec<SceneNode> {
        let root = archive.getTop();
        let mut children = Vec::new();
        for child in root.getChildren() {
            children.push(Self::build_scene_node(&child));
        }
        children
    }
    
    fn build_scene_node(obj: &crate::abc::IObject) -> SceneNode {
        let name = obj.getName();
        
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
        
        for child in obj.getChildren() {
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
        let root = archive.getTop();
        Self::show_props_recursive(ui, &root, name, self.current_frame);
    }
    
    fn show_props_recursive(ui: &mut egui::Ui, obj: &crate::abc::IObject, name: &str, frame: usize) -> bool {
        if obj.getName() == name {
            // Found the object - show its properties
            if let Some(mesh) = crate::geom::IPolyMesh::new(obj) {
                ui.label("Type: PolyMesh");
                let num_samples = mesh.getNumSamples();
                ui.label(format!("Samples: {}", num_samples));
                let sample_idx = if num_samples > 0 {
                    // Clamp to last sample to mirror SampleSelector behavior.
                    frame.min(num_samples - 1)
                } else {
                    0
                };
                if num_samples > 0 {
                    if let Ok(sample) = mesh.getSample(sample_idx) {
                        ui.label(format!("Vertices: {}", sample.positions.len()));
                        ui.label(format!("Faces: {}", sample.face_counts.len()));
                        // Compute and show bounds center (world space since mesh data is baked).
                        if !sample.positions.is_empty() {
                            let mut min = sample.positions[0];
                            let mut max = sample.positions[0];
                            for p in &sample.positions {
                                min = min.min(*p);
                                max = max.max(*p);
                            }
                            let center = (min + max) * 0.5;
                            ui.label(format!(
                                "Center: ({:.2}, {:.2}, {:.2})",
                                center.x, center.y, center.z
                            ));
                            ui.label(format!(
                                "Min: ({:.2}, {:.2}, {:.2})",
                                min.x, min.y, min.z
                            ));
                            ui.label(format!(
                                "Max: ({:.2}, {:.2}, {:.2})",
                                max.x, max.y, max.z
                            ));
                        }
                    }
                }
            } else if let Some(xform) = crate::geom::IXform::new(obj) {
                ui.label("Type: Xform");
                let num_samples = xform.getNumSamples();
                ui.label(format!("Samples: {}", num_samples));
                let sample_idx = if num_samples > 0 {
                    // Clamp to last sample to mirror SampleSelector behavior.
                    frame.min(num_samples - 1)
                } else {
                    0
                };
                if num_samples > 0 {
                    if let Ok(sample) = xform.getSample(sample_idx) {
                        let matrix = sample.matrix();
                        let (_, rot, trans) = matrix.to_scale_rotation_translation();
                        ui.label(format!("Pos: ({:.2}, {:.2}, {:.2})", trans.x, trans.y, trans.z));
                        let euler: (f32, f32, f32) = rot.to_euler(glam::EulerRot::XYZ);
                        ui.label(format!("Rot: ({:.1}°, {:.1}°, {:.1}°)", 
                            euler.0.to_degrees(), euler.1.to_degrees(), euler.2.to_degrees()));
                    }
                }
            } else if let Some(cam) = crate::geom::ICamera::new(obj) {
                ui.label("Type: Camera");
                let num_samples = cam.getNumSamples();
                ui.label(format!("Samples: {}", num_samples));
                let sample_idx = if num_samples > 0 {
                    // Clamp to last sample to mirror SampleSelector behavior.
                    frame.min(num_samples - 1)
                } else {
                    0
                };
                if num_samples > 0 {
                    if let Ok(sample) = cam.getSample(sample_idx) {
                        ui.label(format!("Focal: {:.1}mm", sample.focal_length));
                        ui.label(format!("Aperture: {:.1}mm", sample.horizontal_aperture));
                    }
                }
            } else if let Some(subd) = crate::geom::ISubD::new(obj) {
                ui.label("Type: SubD".to_string());
                let num_samples = subd.getNumSamples();
                ui.label(format!("Samples: {}", num_samples));
                let sample_idx = if num_samples > 0 {
                    // Clamp to last sample to mirror SampleSelector behavior.
                    frame.min(num_samples - 1)
                } else {
                    0
                };
                if num_samples > 0 {
                    if let Ok(sample) = subd.getSample(sample_idx) {
                        ui.label(format!("Vertices: {}", sample.positions.len()));
                        ui.label(format!("Faces: {}", sample.face_counts.len()));
                    }
                }
            } else if let Some(curves) = crate::geom::ICurves::new(obj) {
                ui.label("Type: Curves".to_string());
                let num_samples = curves.getNumSamples();
                ui.label(format!("Samples: {}", num_samples));
                let sample_idx = if num_samples > 0 {
                    // Clamp to last sample to mirror SampleSelector behavior.
                    frame.min(num_samples - 1)
                } else {
                    0
                };
                if num_samples > 0 {
                    if let Ok(sample) = curves.getSample(sample_idx) {
                        ui.label(format!("Points: {}", sample.positions.len()));
                        ui.label(format!("Curves: {}", sample.num_curves()));
                    }
                }
            } else if let Some(points) = crate::geom::IPoints::new(obj) {
                ui.label("Type: Points".to_string());
                let num_samples = points.getNumSamples();
                ui.label(format!("Samples: {}", num_samples));
                let sample_idx = if num_samples > 0 {
                    // Clamp to last sample to mirror SampleSelector behavior.
                    frame.min(num_samples - 1)
                } else {
                    0
                };
                if num_samples > 0 {
                    if let Ok(sample) = points.getSample(sample_idx) {
                        ui.label(format!("Point count: {}", sample.positions.len()));
                        if sample.has_widths() {
                            ui.label("Has widths: Yes");
                        }
                        if sample.has_velocities() {
                            ui.label("Has velocities: Yes");
                        }
                    }
                }
            } else if let Some(light) = crate::geom::ILight::new(obj) {
                ui.label("Type: Light".to_string());
                ui.label(format!("Samples: {}", light.getNumSamples()));
            } else if let Some(mat) = crate::material::IMaterial::new(obj) {
                ui.label("Type: Material".to_string());
                let targets = mat.target_names();
                ui.label(format!("Targets: {}", targets.join(", ")));
                if mat.has_inheritance() {
                    if let Some(parent) = mat.inherits_path() {
                        ui.label(format!("Inherits: {}", parent));
                    }
                }
            }
            return true;
        }
        
        // Recurse into children
        for child in obj.getChildren() {
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
            None,  // no smooth data for test cube
        );

        self.mesh_count = renderer.meshes.len();
        self.vertex_count = vertices.len();
        self.face_count = indices.len() / 3;
        
        // Update scene bounds from renderer
        self.update_bounds_from_renderer();
        
        self.status_message = "Loaded test cube".into();
    }
    
    /// Update scene_bounds from renderer's computed bounds
    fn update_bounds_from_renderer(&mut self) {
        if let Some(renderer) = &mut self.viewport.renderer {
            if let Some((min, max)) = renderer.compute_scene_bounds() {
                let bounds = mesh_converter::Bounds { min, max };
                renderer.set_scene_bounds(bounds.center(), bounds.radius());
                self.scene_bounds = Some(bounds);
            } else {
                self.scene_bounds = None;
            }
        }
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
        self.scene_is_static = false;
        self.last_scene_hash = None;
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
        let _span = tracing::info_span!("process_worker_results").entered();
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
        let _span = tracing::info_span!("apply_scene").entered();
        let renderer = match &mut self.viewport.renderer {
            Some(r) => r,
            None => return,
        };

        self.scene_is_static = scene.is_static;
        if self.scene_is_static {
            self.playing = false;
            self.pending_frame = None;
        }

        let scene_hash = Self::compute_scene_hash(&scene);
        if self.last_scene_hash == Some(scene_hash) {
            self.current_frame = frame;
            return;
        }
        self.last_scene_hash = Some(scene_hash);

        let stats = mesh_converter::compute_stats(&scene.meshes);
        let bounds = mesh_converter::compute_scene_bounds(&scene.meshes, &scene.points, &scene.curves);
        self.scene_bounds = if bounds.is_valid() { Some(bounds) } else { None };
        
        // Update shadow bounds
        if let Some(ref b) = self.scene_bounds {
            renderer.set_scene_bounds(b.center(), b.radius());
        }
        
        // Update floor size if enabled (scene_bounds changed)
        if self.settings.show_floor {
            renderer.set_floor(&self.scene_bounds);
        }
        
        // Always update scene cameras (clear stale data when loading new file)
        self.scene_cameras = scene.cameras;

        // Always update scene lights (clear stale data when loading new file)
        self.scene_lights = scene.lights;
        // Apply scene lights if setting enabled
        if self.settings.use_scene_lights && !self.scene_lights.is_empty() {
            renderer.set_scene_lights(&self.scene_lights);
        }

        // Collect paths using references (use path for uniqueness, not name)
        // IMPORTANT: Different objects may have the same name (e.g., brake_discShape)
        let new_mesh_paths: std::collections::HashSet<&str> = 
            scene.meshes.iter().map(|m| m.path.as_str()).collect();
        let new_curve_paths: std::collections::HashSet<&str> = 
            scene.curves.iter().map(|c| c.path.as_str()).collect();
        let new_point_paths: std::collections::HashSet<&str> =
            scene.points.iter().map(|p| p.path.as_str()).collect();
        
        // Remove meshes that no longer exist (retain avoids intermediate Vec)
        renderer.meshes.retain(|path, _| new_mesh_paths.contains(path.as_str()));
        renderer.curves.retain(|path, _| new_curve_paths.contains(path.as_str()));
        renderer.points.retain(|path, _| new_point_paths.contains(path.as_str()));

        let mut smooth_dirty = false;

        // Update or add meshes (use path as key for uniqueness)
        for mesh in scene.meshes {
            if renderer.has_mesh(&mesh.path) {
                // Always update transform (cheap uniform write)
                renderer.update_mesh_transform(&mesh.path, mesh.transform);
                
                // Only update vertices if they actually changed (expensive buffer recreation)
                let new_hash = super::renderer::compute_mesh_hash(&mesh.vertices, &mesh.indices);
                if let Some(old_hash) = renderer.get_vertex_hash(&mesh.path) {
                    if new_hash != old_hash {
                        renderer.update_mesh_vertices(&mesh.path, &mesh.vertices, &mesh.indices);
                        smooth_dirty = true;
                    }
                }
            } else {
                // Build material from mesh properties
                let base_color = mesh.base_color.unwrap_or(Vec3::new(0.7, 0.7, 0.75));
                let roughness = mesh.roughness.unwrap_or(0.4);
                let metallic = mesh.metallic.unwrap_or(0.0);
                
                let mut material = if metallic > 0.5 {
                    StandardSurfaceParams::metal(base_color, roughness)
                } else {
                    StandardSurfaceParams::plastic(base_color, roughness)
                };
                material.set_metalness(metallic);
                
                renderer.add_mesh(
                    mesh.path,  // Use path for unique key
                    &mesh.vertices,
                    &mesh.indices,
                    mesh.transform,
                    &material,
                    mesh.smooth_data,
                );
                smooth_dirty = true;
            }
        }

        // Update or add curves
        let curves_material = StandardSurfaceParams::plastic(
            Vec3::new(0.9, 0.7, 0.3),
            0.3,
        );
        for curves in scene.curves {
            if renderer.has_curves(&curves.path) {
                renderer.update_curves_transform(&curves.path, curves.transform);
                let new_hash = super::renderer::compute_curves_hash(&curves.vertices, &curves.indices);
                if let Some(old_hash) = renderer.get_curves_hash(&curves.path) {
                    if new_hash != old_hash {
                        renderer.update_curves_vertices(&curves.path, &curves.vertices, &curves.indices);
                    }
                }
            } else {
                renderer.add_curves(
                    curves.path,  // Use path for unique key
                    &curves.vertices,
                    &curves.indices,
                    curves.transform,
                    &curves_material,
                );
            }
        }

        // Update or add points
        let points_material = StandardSurfaceParams::plastic(
            Vec3::new(0.3, 0.8, 0.9),  // cyan-ish color for points
            0.5,
        );
        for pts in scene.points {
            if renderer.has_points(&pts.path) {
                renderer.update_points_transform(&pts.path, pts.transform);
                let new_hash = super::renderer::compute_points_hash(&pts.positions, &pts.widths);
                if let Some(old_hash) = renderer.get_points_hash(&pts.path) {
                    if new_hash != old_hash {
                        renderer.update_points_vertices(&pts.path, &pts.positions, &pts.widths);
                    }
                }
            } else {
                renderer.add_points(
                    pts.path,  // Use path for unique key
                    &pts.positions,
                    &pts.widths,
                    pts.transform,
                    &points_material,
                );
            }
        }

        if self.settings.smooth_normals && smooth_dirty {
            renderer.recalculate_smooth_normals(
                self.settings.smooth_angle,
                true,
                false,
            );
        }

        // Update stats
        self.mesh_count = stats.mesh_count;
        self.vertex_count = stats.vertex_count;
        self.face_count = stats.triangle_count;
        self.current_frame = frame;
    }
    
    /// Find next or previous file with given extensions in the same directory
    /// direction: -1 for previous, +1 for next
    fn find_sibling_file(current: &PathBuf, direction: i32, extensions: &[&str]) -> Option<PathBuf> {
        let dir = current.parent()?;
        
        // Collect all matching files in directory
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|ext| {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        extensions.iter().any(|e| e.eq_ignore_ascii_case(&ext_str))
                    })
                    .unwrap_or(false)
            })
            .collect();
        
        // Sort alphabetically
        files.sort();
        
        if files.is_empty() {
            return None;
        }
        
        // Find current file index
        let current_idx = files.iter().position(|p| p == current)?;
        
        // Calculate new index with wrapping
        let new_idx = if direction > 0 {
            (current_idx + 1) % files.len()
        } else if current_idx == 0 {
            files.len() - 1
        } else {
            current_idx - 1
        };
        
        // Don't return same file
        if new_idx == current_idx {
            return None;
        }
        
        Some(files[new_idx].clone())
    }
    
    /// Navigate to next or previous ABC file in directory
    fn navigate_sibling_abc(&mut self, direction: i32) {
        if let Some(current) = &self.current_file {
            if let Some(path) = Self::find_sibling_file(current, direction, &["abc"]) {
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
    
    /// Navigate to next or previous HDR/EXR file in directory
    fn navigate_sibling_hdr(&mut self, direction: i32) {
        if let Some(current) = &self.settings.last_hdr_file {
            if let Some(path) = Self::find_sibling_file(current, direction, &["hdr", "exr"]) {
                let filename = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                self.status_message = format!("Loading HDR: {}", filename);
                self.pending_hdr_file = Some(path);
            } else {
                self.status_message = "No other HDR/EXR files in directory".into();
            }
        } else {
            self.status_message = "No HDR file loaded".into();
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
        let _span = tracing::info_span!("viewer_update").entered();
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
        
        if self.active_camera.is_none() {
            if let Some(bounds) = &self.scene_bounds {
                let center = bounds.center();
                let radius = bounds.radius().max(0.1);
                let cam_pos = self.viewport.camera.position();
                let dist = (cam_pos - center).length();
                let margin = radius * 2.0;
                // Near: small fraction of distance, but not too small for Z precision
                let min_near = 0.1;
                let max_near = dist * 0.1; // Never clip more than 10% of view distance
                let target_near = (dist - margin).clamp(min_near, max_near);
                // Far: enough to see entire scene
                let target_far = (dist + margin * 2.0).max(radius * 4.0);
                let dt = ctx.input(|i| i.stable_dt);
                let t = (dt * 6.0).clamp(0.0, 1.0);
                self.viewport.camera.near =
                    self.viewport.camera.near + (target_near - self.viewport.camera.near) * t;
                self.viewport.camera.far =
                    self.viewport.camera.far + (target_far - self.viewport.camera.far) * t;
            }
        }
        
        // Process any ready frames from background worker (non-blocking)
        self.process_worker_results();
        
        // Escape - exit fullscreen first, then close app
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.is_fullscreen {
                self.is_fullscreen = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            } else {
                if let Some(mut worker) = self.worker.take() {
                    worker.stop();
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        
        // Z = Toggle fullscreen
        if ctx.input(|i| i.key_pressed(egui::Key::Z)) {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        }
        
        // Navigate ABC files in directory: PageUp = prev, PageDown = next
        // Navigate HDR files: Ctrl+PageUp = prev, Ctrl+PageDown = next
        if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
            if ctx.input(|i| i.modifiers.ctrl) {
                self.navigate_sibling_hdr(-1);
            } else {
                self.navigate_sibling_abc(-1);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
            if ctx.input(|i| i.modifiers.ctrl) {
                self.navigate_sibling_hdr(1);
            } else {
                self.navigate_sibling_abc(1);
            }
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
                    renderer.use_ssao = self.settings.ssao;
                    renderer.ssao_strength = self.settings.ssao_strength;
                    renderer.ssao_radius = self.settings.ssao_radius;
                    renderer.hdr_visible = self.settings.hdr_visible;
                    renderer.xray_alpha = self.settings.xray_alpha;
                    renderer.double_sided = self.settings.double_sided;
                    renderer.auto_normals = self.settings.auto_normals;
                    renderer.background_color = self.settings.background_color;
                    // Set floor if enabled (uses scene_bounds for sizing)
                    if self.settings.show_floor {
                        renderer.set_floor(&self.scene_bounds);
                    }
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
            self.menu_bar(ctx, ui);
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
            let response = SidePanel::left("hierarchy_panel")
                .default_width(self.settings.hierarchy_panel_width)
                .min_width(100.0)
                .max_width(500.0)
                .resizable(true)
                .show(ctx, |ui| {
                    self.hierarchy_panel(ui);
                });
            // Save panel width on resize
            if response.response.rect.width() != self.settings.hierarchy_panel_width {
                self.settings.hierarchy_panel_width = response.response.rect.width();
                self.settings.save();
            }
        }

        // Right side panel
        let response = SidePanel::right("side_panel")
            .default_width(self.settings.side_panel_width)
            .min_width(150.0)
            .max_width(400.0)
            .resizable(true)
            .show(ctx, |ui| {
                self.side_panel(ui);
            });
        // Save panel width on resize
        if response.response.rect.width() != self.settings.side_panel_width {
            self.settings.side_panel_width = response.response.rect.width();
            self.settings.save();
        }

        // Central viewport
        CentralPanel::default().show(ctx, |ui| {
            let render_state = frame.wgpu_render_state();
            self.viewport.show(ui, render_state);
        });


        // Track window size and position for saving on exit
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.settings.window_width = rect.width();
                self.settings.window_height = rect.height();
            }
            if let Some(pos) = i.viewport().outer_rect {
                self.settings.window_x = Some(pos.min.x);
                self.settings.window_y = Some(pos.min.y);
            }
        });
        
        // Request repaint only when animation is playing
        // egui handles repaint for pointer interactions automatically
        // This saves CPU when idle (was causing 100% CPU usage)
        if self.playing || self.settings.path_tracing {
            ctx.request_repaint();
        }
    }
}
