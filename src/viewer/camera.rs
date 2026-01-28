//! Simple Maya-style orbit camera using glam (no external rig library)

use glam::{Mat4, Vec3, Quat};

const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

pub fn wgpu_projection(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    OPENGL_TO_WGPU_MATRIX * Mat4::perspective_rh(fov_y, aspect, near, far)
}

/// Maya-style orbit camera: pivot point + yaw/pitch + arm distance
pub struct OrbitCamera {
    /// Point of interest (orbit pivot)
    pub target: Vec3,
    /// Horizontal rotation in degrees
    pub yaw: f32,
    /// Vertical rotation in degrees (clamped to avoid gimbal flip)
    pub pitch: f32,
    /// Distance from target
    pub distance: f32,
    /// Vertical FOV in degrees
    pub fov: f32,

    // Inertia velocities (per second)
    vel_yaw: f32,
    vel_pitch: f32,
    vel_pan: Vec3,
    vel_zoom: f32,
    /// Inertia decay time in milliseconds (0 = no inertia)
    pub inertia_ms: f32,
    /// True while user is actively dragging
    dragging: bool,
}

impl OrbitCamera {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            yaw: 45.0,
            pitch: -30.0,
            distance,
            fov: 45.0,
            vel_yaw: 0.0,
            vel_pitch: 0.0,
            vel_pan: Vec3::ZERO,
            vel_zoom: 0.0,
            inertia_ms: 150.0,
            dragging: false,
        }
    }

    pub fn near(&self) -> f32 { 0.1 }
    pub fn far(&self) -> f32 { 10000.0 }

    /// Call when user starts dragging
    pub fn begin_drag(&mut self) {
        self.dragging = true;
        // Kill residual velocity on new drag
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
        self.vel_pan = Vec3::ZERO;
        self.vel_zoom = 0.0;
    }

    /// Call when user stops dragging
    pub fn end_drag(&mut self) {
        self.dragging = false;
    }

    /// Orbit around target (LMB drag) - Maya tumble
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.3;
        let dy = -delta_x * sensitivity;
        let dp = -delta_y * sensitivity;
        self.yaw += dy;
        self.pitch = (self.pitch + dp).clamp(-89.0, 89.0);
        // Store velocity as degrees/frame for inertia
        self.vel_yaw = dy;
        self.vel_pitch = dp;
    }

    /// Pan camera (MMB drag) - screen-space translation of pivot
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let rot = self.rotation();
        let right = rot * Vec3::X;
        let up = rot * Vec3::Y;
        let sensitivity = 0.002 * self.distance;
        let offset = right * (-delta_x * sensitivity) + up * (delta_y * sensitivity);
        self.target += offset;
        self.vel_pan = offset;
    }

    /// Zoom (RMB drag / scroll)
    pub fn zoom(&mut self, delta: f32) {
        let sensitivity = 0.0002 * self.distance.max(1.0);
        let factor = 1.0 - delta * sensitivity;
        self.distance = (self.distance * factor).clamp(0.01, 50000.0);
        self.vel_zoom = delta * sensitivity;
    }

    /// Focus on bounding box center with given radius
    pub fn focus(&mut self, center: Vec3, radius: f32) {
        self.target = center;
        self.distance = radius * 2.5;
        self.kill_inertia();
    }

    /// Reset to default view
    pub fn reset(&mut self) {
        self.target = Vec3::ZERO;
        self.yaw = 45.0;
        self.pitch = -30.0;
        self.distance = 5.0;
        self.kill_inertia();
    }

    /// Set distance from target
    pub fn set_distance(&mut self, dist: f32) {
        self.distance = dist.clamp(0.01, 50000.0);
    }

    /// Get yaw and pitch angles in degrees
    pub fn angles(&self) -> (f32, f32) {
        (self.yaw, self.pitch)
    }

    /// Set yaw and pitch angles in degrees
    pub fn set_angles(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch.clamp(-89.0, 89.0);
    }

    /// Returns true if inertia is still active (needs repaint)
    pub fn update(&mut self, dt: f32) -> bool {
        if self.dragging || self.inertia_ms <= 0.0 {
            return false;
        }

        let has_velocity = self.vel_yaw.abs() > 0.001
            || self.vel_pitch.abs() > 0.001
            || self.vel_pan.length_squared() > 1e-8
            || self.vel_zoom.abs() > 1e-6;

        if !has_velocity {
            return false;
        }

        // Exponential decay: half-life = inertia_ms / 1000 * ln(2)
        let decay = (-dt / (self.inertia_ms * 0.001)).exp();

        // Apply velocities
        self.yaw += self.vel_yaw;
        self.pitch = (self.pitch + self.vel_pitch).clamp(-89.0, 89.0);
        self.target += self.vel_pan;
        let zoom_factor = 1.0 - self.vel_zoom;
        self.distance = (self.distance * zoom_factor).clamp(0.01, 50000.0);

        // Decay
        self.vel_yaw *= decay;
        self.vel_pitch *= decay;
        self.vel_pan *= decay;
        self.vel_zoom *= decay;

        // Stop when negligible
        if self.vel_yaw.abs() < 0.001 { self.vel_yaw = 0.0; }
        if self.vel_pitch.abs() < 0.001 { self.vel_pitch = 0.0; }
        if self.vel_pan.length_squared() < 1e-8 { self.vel_pan = Vec3::ZERO; }
        if self.vel_zoom.abs() < 1e-6 { self.vel_zoom = 0.0; }

        true
    }

    /// Kill all inertia velocity
    pub fn kill_inertia(&mut self) {
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
        self.vel_pan = Vec3::ZERO;
        self.vel_zoom = 0.0;
    }

    /// Camera rotation quaternion (yaw around Y, then pitch around local X)
    fn rotation(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::YXZ, self.yaw.to_radians(), self.pitch.to_radians(), 0.0)
    }

    /// Get camera world position
    pub fn position(&self) -> Vec3 {
        let rot = self.rotation();
        let offset = rot * Vec3::new(0.0, 0.0, self.distance);
        self.target + offset
    }

    /// Get view matrix (look from position toward target)
    pub fn view_matrix(&self) -> Mat4 {
        let pos = self.position();
        Mat4::look_at_rh(pos, self.target, Vec3::Y)
    }

    /// Get projection matrix
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        wgpu_projection(self.fov.to_radians(), aspect, self.near(), self.far())
    }

    /// Get combined view-projection matrix
    pub fn view_proj_matrix(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 5.0)
    }
}
