//! Math type re-exports and Alembic-specific math utilities.
//!
//! This module re-exports types from `glam` and provides additional
//! types specific to Alembic (like bounding boxes).

// Re-export glam types
pub use glam::{
    // Single precision vectors
    Vec2, Vec3, Vec3A, Vec4,
    // Double precision vectors
    DVec2, DVec3, DVec4,
    // Integer vectors
    IVec2, IVec3, IVec4,
    UVec2, UVec3, UVec4,
    // Single precision matrices
    Mat2, Mat3, Mat3A, Mat4,
    // Double precision matrices
    DMat2, DMat3, DMat4,
    // Quaternions
    Quat, DQuat,
    // Affine transforms
    Affine2, Affine3A, DAffine2, DAffine3,
};

use bytemuck::{Pod, Zeroable};
use std::fmt;

/// 3D bounding box with single precision.
#[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct BBox3f {
    pub min: Vec3,
    pub max: Vec3,
}

impl BBox3f {
    /// Empty bounding box (inverted, will expand on first point).
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    /// Create a new bounding box from min and max points.
    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Create a bounding box from a single point.
    #[inline]
    pub fn from_point(p: Vec3) -> Self {
        Self { min: p, max: p }
    }

    /// Check if this box is empty (has no volume).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// Expand this box to include a point.
    #[inline]
    pub fn expand_by_point(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    /// Expand this box to include another box.
    #[inline]
    pub fn expand_by_box(&mut self, other: &Self) {
        if !other.is_empty() {
            self.min = self.min.min(other.min);
            self.max = self.max.max(other.max);
        }
    }

    /// Get the center of the box.
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the size (extents) of the box.
    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

impl Default for BBox3f {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl fmt::Debug for BBox3f {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BBox3f({:?} - {:?})", self.min, self.max)
    }
}

/// 3D bounding box with double precision.
#[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct BBox3d {
    pub min: DVec3,
    pub max: DVec3,
}

impl BBox3d {
    /// Empty bounding box (inverted, will expand on first point).
    pub const EMPTY: Self = Self {
        min: DVec3::splat(f64::INFINITY),
        max: DVec3::splat(f64::NEG_INFINITY),
    };

    /// Create a new bounding box from min and max points.
    #[inline]
    pub const fn new(min: DVec3, max: DVec3) -> Self {
        Self { min, max }
    }

    /// Create a bounding box from a single point.
    #[inline]
    pub fn from_point(p: DVec3) -> Self {
        Self { min: p, max: p }
    }

    /// Check if this box is empty (has no volume).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// Expand this box to include a point.
    #[inline]
    pub fn expand_by_point(&mut self, p: DVec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    /// Expand this box to include another box.
    #[inline]
    pub fn expand_by_box(&mut self, other: &Self) {
        if !other.is_empty() {
            self.min = self.min.min(other.min);
            self.max = self.max.max(other.max);
        }
    }

    /// Get the center of the box.
    #[inline]
    pub fn center(&self) -> DVec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the size (extents) of the box.
    #[inline]
    pub fn size(&self) -> DVec3 {
        self.max - self.min
    }

    /// Convert to single precision.
    #[inline]
    pub fn as_f32(&self) -> BBox3f {
        BBox3f {
            min: self.min.as_vec3(),
            max: self.max.as_vec3(),
        }
    }
}

impl Default for BBox3d {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl fmt::Debug for BBox3d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BBox3d({:?} - {:?})", self.min, self.max)
    }
}

impl From<BBox3f> for BBox3d {
    fn from(b: BBox3f) -> Self {
        Self {
            min: b.min.as_dvec3(),
            max: b.max.as_dvec3(),
        }
    }
}

/// Chrono type - time value (seconds).
pub type Chrono = f64;

/// Index type for samples.
pub type Index = i64;

/// Special value indicating unknown index.
pub const INDEX_UNKNOWN: Index = i64::MAX;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox3f() {
        let mut b = BBox3f::EMPTY;
        assert!(b.is_empty());

        b.expand_by_point(Vec3::ZERO);
        assert!(!b.is_empty());
        assert_eq!(b.min, Vec3::ZERO);
        assert_eq!(b.max, Vec3::ZERO);

        b.expand_by_point(Vec3::ONE);
        assert_eq!(b.min, Vec3::ZERO);
        assert_eq!(b.max, Vec3::ONE);
        assert_eq!(b.center(), Vec3::splat(0.5));
        assert_eq!(b.size(), Vec3::ONE);
    }

    #[test]
    fn test_bbox3d() {
        let mut b = BBox3d::EMPTY;
        assert!(b.is_empty());

        b.expand_by_point(DVec3::new(-1.0, -1.0, -1.0));
        b.expand_by_point(DVec3::new(1.0, 1.0, 1.0));

        assert_eq!(b.center(), DVec3::ZERO);
        assert_eq!(b.size(), DVec3::splat(2.0));
    }

    #[test]
    fn test_bbox_pod() {
        // Verify that BBox types are Pod-compatible
        assert_eq!(std::mem::size_of::<BBox3f>(), 24);  // 2 * Vec3 = 2 * 12
        assert_eq!(std::mem::size_of::<BBox3d>(), 48);  // 2 * DVec3 = 2 * 24
    }
}
