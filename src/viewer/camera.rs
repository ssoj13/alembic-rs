//! Camera controls using dolly

use dolly::prelude::*;
use glam::{Mat4, Vec3};

const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

pub fn wgpu_projection(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    // wgpu uses 0..1 depth; no Y flip needed for NDC orientation.
    OPENGL_TO_WGPU_MATRIX * Mat4::perspective_rh(fov_y, aspect, near, far)
}

/// Orbit camera rig for 3D viewport
pub struct OrbitCamera {
    rig: CameraRig,
    /// Vertical FOV in degrees
    pub fov: f32,
    /// Near clip plane
    pub near: f32,
    /// Far clip plane
    pub far: f32,
}

impl OrbitCamera {
    pub fn new(target: Vec3, distance: f32) -> Self {
        let rig = CameraRig::builder()
            .with(YawPitch::new().yaw_degrees(45.0).pitch_degrees(-30.0))
            .with(Smooth::new_rotation(0.0))
            .with(Arm::new(mint::Vector3 { x: 0.0, y: 0.0, z: distance }))
            .with(Smooth::new_position(0.0))
            .with(LookAt::new(mint::Point3 { x: target.x, y: target.y, z: target.z }).tracking_smoothness(0.0))
            .build();

        Self {
            rig,
            fov: 45.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Orbit around target (drag)
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.5;
        self.rig.driver_mut::<YawPitch>().rotate_yaw_pitch(
            -delta_x * sensitivity,
            -delta_y * sensitivity,
        );
    }

    /// Pan camera (shift+drag) - screen-space pan like Houdini/Maya
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Use camera right and up vectors for screen-space movement
        let right: Vec3 = self.rig.final_transform.right();
        let up: Vec3 = self.rig.final_transform.up();
        let dist = self.rig.driver::<Arm>().offset.z;
        
        let sensitivity = 0.002 * dist;  // Scale with distance
        let offset = right * (-delta_x * sensitivity) + up * (delta_y * sensitivity);
        
        let look_at = self.rig.driver_mut::<LookAt>();
        look_at.target.x += offset.x;
        look_at.target.y += offset.y;
        look_at.target.z += offset.z;
    }

    /// Zoom (scroll)
    pub fn zoom(&mut self, delta: f32) {
        let arm = self.rig.driver_mut::<Arm>();
        let current = arm.offset.z;
        let sensitivity = 0.0002 * current.max(1.0);
        let factor = 1.0 - delta * sensitivity;
        arm.offset.z = (current * factor).clamp(0.1, 5000.0);
    }

    /// Focus on bounding box
    pub fn focus(&mut self, center: Vec3, radius: f32) {
        let look_at = self.rig.driver_mut::<LookAt>();
        look_at.target = mint::Point3 { x: center.x, y: center.y, z: center.z };
        self.rig.driver_mut::<Arm>().offset.z = radius * 2.5;
    }

    /// Reset to default view
    pub fn reset(&mut self) {
        self.rig.driver_mut::<YawPitch>().set_rotation_quat(
            mint::Quaternion::from(glam::Quat::from_euler(
                glam::EulerRot::YXZ, 
                45.0_f32.to_radians(), 
                -30.0_f32.to_radians(), 
                0.0
            ))
        );
        self.rig.driver_mut::<LookAt>().target = mint::Point3 { x: 0.0, y: 0.0, z: 0.0 };
        self.rig.driver_mut::<Arm>().offset.z = 5.0;
    }

    /// Get current distance from target
    pub fn distance(&self) -> f32 {
        self.rig.driver::<Arm>().offset.z
    }

    /// Set distance from target
    pub fn set_distance(&mut self, dist: f32) {
        self.rig.driver_mut::<Arm>().offset.z = dist.clamp(0.1, 5000.0);
    }

    /// Get yaw and pitch angles in degrees (from final transform)
    pub fn angles(&self) -> (f32, f32) {
        // Extract euler angles from the final transform rotation
        let rot = self.rig.final_transform.rotation;
        let q = glam::Quat::from_xyzw(rot.v.x, rot.v.y, rot.v.z, rot.s);
        let (yaw, pitch, _) = q.to_euler(glam::EulerRot::YXZ);
        (yaw.to_degrees(), pitch.to_degrees())
    }

    /// Set yaw and pitch angles in degrees
    pub fn set_angles(&mut self, yaw: f32, pitch: f32) {
        let yp = self.rig.driver_mut::<YawPitch>();
        yp.set_rotation_quat(mint::Quaternion::from(glam::Quat::from_euler(
            glam::EulerRot::YXZ,
            yaw.to_radians(),
            pitch.to_radians(),
            0.0,
        )));
    }

    /// Update camera (call each frame)
    pub fn update(&mut self, dt: f32) {
        self.rig.update(dt);
    }

    /// Get camera position
    pub fn position(&self) -> Vec3 {
        let p = self.rig.final_transform.position;
        Vec3::new(p.x, p.y, p.z)
    }

    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4 {
        let t = &self.rig.final_transform;
        let pos = Vec3::new(t.position.x, t.position.y, t.position.z);
        let fwd: Vec3 = t.forward();
        let up: Vec3 = t.up();
        Mat4::look_at_rh(pos, pos + fwd, up)
    }

    /// Get projection matrix
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        wgpu_projection(self.fov.to_radians(), aspect, self.near, self.far)
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
