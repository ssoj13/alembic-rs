//! 3D Viewport widget for egui

use egui::{Response, Sense, Ui, Vec2};

use super::camera::OrbitCamera;
use super::renderer::Renderer;

/// Scene camera override parameters
#[derive(Clone)]
pub struct SceneCameraOverride {
    pub view: glam::Mat4,
    pub fov_y: f32,  // radians
    pub near: f32,
    pub far: f32,
}

/// 3D Viewport state
pub struct Viewport {
    pub camera: OrbitCamera,
    pub renderer: Option<Renderer>,
    texture_id: Option<egui::TextureId>,
    render_texture: Option<RenderTexture>,
    last_size: Vec2,
    /// Optional scene camera override
    pub scene_camera: Option<SceneCameraOverride>,
    /// Pending Ctrl+Click focus pick (normalized 0-1 coords)
    pending_focus_pick: Option<(f32, f32)>,
    /// Pending Shift+LMB object pick (normalized 0-1 coords)
    pending_object_pick: Option<(f32, f32)>,
    /// Current mouse hover position (normalized 0-1 coords)
    pub hover_position: Option<(f32, f32)>,
}

struct RenderTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            camera: OrbitCamera::default(),
            renderer: None,
            texture_id: None,
            render_texture: None,
            last_size: Vec2::ZERO,
            scene_camera: None,
            pending_focus_pick: None,
            pending_object_pick: None,
            hover_position: None,
        }
    }

    /// Initialize renderer (call once when wgpu context is available)
    pub fn init_renderer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) {
        self.renderer = Some(Renderer::new(
            std::sync::Arc::new(device.clone()),
            std::sync::Arc::new(queue.clone()),
            format,
        ));
    }

    /// Show viewport UI and handle input
    pub fn show(&mut self, ui: &mut Ui, wgpu_render_state: Option<&egui_wgpu::RenderState>) -> Response {
        let _span = tracing::info_span!("viewport_show").entered();
        let available = ui.available_size();
        let size = Vec2::new(available.x.max(64.0), available.y.max(64.0));

        // Allocate space for the viewport
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        // Handle camera input
        self.handle_input(ui, &response);

        // Update camera
        self.camera.update(ui.input(|i| i.stable_dt));

        // Render to texture if we have a renderer and render state
        if let Some(render_state) = wgpu_render_state {
            let width = size.x as u32;
            let height = size.y as u32;

            if width > 0 && height > 0 {
                // Update camera uniforms - use scene camera if set, otherwise orbit camera
                let aspect = size.x / size.y;
                let (view_proj, view, position) = if let Some(sc) = &self.scene_camera {
                    // Use scene camera
                    let proj = super::camera::wgpu_projection(sc.fov_y, aspect, sc.near, sc.far);
                    let view = sc.view;
                    // Extract position from inverse view matrix
                    let inv_view = view.inverse();
                    let pos = glam::Vec3::new(inv_view.w_axis.x, inv_view.w_axis.y, inv_view.w_axis.z);
                    (proj * view, view, pos)
                } else {
                    // Use orbit camera
                    (self.camera.view_proj_matrix(aspect), self.camera.view_matrix(), self.camera.position())
                };
                
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_camera(view_proj, view, position);
                    // Update shadow map for key light direction
                    // Key light direction from 3-point rig: (-0.5, -0.7, -0.5)
                    let key_light_dir = glam::Vec3::new(-0.5, -0.7, -0.5);
                    renderer.update_shadow(key_light_dir);
                }

                // Ensure render texture exists and is correct size
                self.ensure_render_texture(render_state, width, height);

                // Render scene
                if let (Some(renderer), Some(rt)) = (&mut self.renderer, &self.render_texture) {
                    renderer.render(&rt.view, width, height, self.camera.distance(), self.camera.near(), self.camera.far());
                    
                    // Poll for hover pick result (must be after render submits GPU commands)
                    if renderer.hover_mode != super::settings::HoverMode::None {
                        renderer.poll_hover_result();
                    }
                }

                // Draw the rendered texture
                if let Some(tex_id) = self.texture_id {
                    ui.painter().image(
                        tex_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
        } else {
            // No renderer - draw placeholder
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 35));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Initializing...",
                egui::FontId::default(),
                egui::Color32::GRAY,
            );
        }

        self.last_size = size;
        response
    }

    fn ensure_render_texture(&mut self, render_state: &egui_wgpu::RenderState, width: u32, height: u32) {
        let needs_recreate = match &self.render_texture {
            Some(rt) => rt.size != (width, height),
            None => true,
        };

        if !needs_recreate {
            return;
        }

        let device = &render_state.device;
        let format = render_state.target_format;

        // Create new render texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport_render_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Register with egui
        let tex_id = render_state.renderer.write().register_native_texture(
            device,
            &view,
            wgpu::FilterMode::Linear,
        );

        // Unregister old texture
        if let Some(old_id) = self.texture_id.take() {
            render_state.renderer.write().free_texture(&old_id);
        }

        self.texture_id = Some(tex_id);
        self.render_texture = Some(RenderTexture {
            texture,
            view,
            size: (width, height),
        });
    }

    fn handle_input(&mut self, ui: &Ui, response: &Response) {
        let input = ui.input(|i| i.clone());

        // Ctrl+LMB drag = continuous focus sampling (disable orbit)
        let ctrl_held = input.modifiers.ctrl;
        
        // Orbit with left mouse drag (only when Ctrl not held)
        if response.dragged_by(egui::PointerButton::Primary) && !input.modifiers.shift && !ctrl_held {
            let delta = response.drag_delta();
            self.camera.orbit(delta.x, delta.y);
        }

        // Pan with middle mouse drag
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.camera.pan(delta.x, delta.y);
        }

        // Zoom with right mouse drag
        if response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            self.camera.zoom(delta.y * 0.1);
        }

        // Zoom with scroll
        if response.hovered() {
            let scroll = input.raw_scroll_delta.y;
            if scroll.abs() > 0.0 {
                self.camera.zoom(scroll * 0.1);
            }
        }

        // Reset camera with Home key
        if response.has_focus() && input.key_pressed(egui::Key::Home) {
            self.camera.reset();
        }

        // Focus on scene with F key
        if response.has_focus() && input.key_pressed(egui::Key::F) {
            // TODO: Calculate scene bounds
            self.camera.focus(glam::Vec3::ZERO, 5.0);
        }
        
        // Ctrl+LMB = continuous focus sampling (click or drag)
        if ctrl_held && (response.clicked() || response.dragged_by(egui::PointerButton::Primary)) {
            if let Some(pos) = input.pointer.hover_pos() {
                let rect = response.rect;
                if rect.contains(pos) {
                    let rel_x = (pos.x - rect.left()) / rect.width();
                    let rel_y = (pos.y - rect.top()) / rect.height();
                    self.pending_focus_pick = Some((rel_x, rel_y));
                }
            }
        }
        
        // Shift+LMB = object picking (selection) - continuous like Ctrl+LMB
        let shift_held = input.modifiers.shift;
        if shift_held && (response.clicked() || response.dragged_by(egui::PointerButton::Primary)) {
            if let Some(pos) = input.pointer.hover_pos() {
                let rect = response.rect;
                if rect.contains(pos) {
                    let rel_x = (pos.x - rect.left()) / rect.width();
                    let rel_y = (pos.y - rect.top()) / rect.height();
                    self.pending_object_pick = Some((rel_x, rel_y));
                }
            }
        }
        
        // Track hover position for highlighting
        if response.hovered() {
            if let Some(pos) = input.pointer.hover_pos() {
                let rect = response.rect;
                if rect.contains(pos) {
                    let rel_x = (pos.x - rect.left()) / rect.width();
                    let rel_y = (pos.y - rect.top()) / rect.height();
                    self.hover_position = Some((rel_x, rel_y));
                } else {
                    self.hover_position = None;
                }
            } else {
                self.hover_position = None;
            }
        } else {
            self.hover_position = None;
        }
    }
    
    /// Take pending focus pick request (normalized 0-1 coordinates)
    pub fn take_focus_pick(&mut self) -> Option<(f32, f32)> {
        self.pending_focus_pick.take()
    }
    
    /// Take pending object pick request (normalized 0-1 coordinates)
    pub fn take_object_pick(&mut self) -> Option<(f32, f32)> {
        self.pending_object_pick.take()
    }
    
    /// Get current render texture size (width, height)
    pub fn render_texture_size(&self) -> Option<(u32, u32)> {
        self.render_texture.as_ref().map(|rt| rt.size)
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new()
    }
}
