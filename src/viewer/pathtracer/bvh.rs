//! BVH node and primitive types for GPU path tracing.
//!
//! Flat array layout optimized for GPU traversal:
//! - 32-byte nodes (cache-line friendly)
//! - Triangles packed with vertex data for coherent access

use bytemuck::{Pod, Zeroable};

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    pub const EMPTY: Self = Self {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    /// Grow to include a point.
    #[inline]
    pub fn grow_point(&mut self, p: [f32; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
    }

    /// Grow to include another AABB.
    #[inline]
    pub fn grow(&mut self, other: &Aabb) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(other.min[i]);
            self.max[i] = self.max[i].max(other.max[i]);
        }
    }

    /// Surface area (for SAH cost).
    #[inline]
    pub fn area(&self) -> f32 {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        let dz = self.max[2] - self.min[2];
        2.0 * (dx * dy + dy * dz + dz * dx)
    }

    /// Longest axis (0=x, 1=y, 2=z).
    #[inline]
    pub fn longest_axis(&self) -> usize {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        let dz = self.max[2] - self.min[2];
        if dx >= dy && dx >= dz {
            0
        } else if dy >= dz {
            1
        } else {
            2
        }
    }

    /// Centroid of the AABB.
    #[inline]
    pub fn centroid(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }
}

/// GPU-friendly BVH node (32 bytes, matches WGSL struct).
///
/// Internal node: left_or_first = left child index, count = 0
/// Leaf node: left_or_first = first triangle index, count > 0
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BvhNode {
    pub aabb_min: [f32; 3],
    pub left_or_first: u32,
    pub aabb_max: [f32; 3],
    pub count: u32,
}

/// Triangle primitive for GPU storage (112 bytes).
/// Packed: 3 vertices × (pos + normal) = 3 × 2 × vec3, plus material and object IDs.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuTriangle {
    pub v0: [f32; 3],
    pub material_id: u32,
    pub v1: [f32; 3],
    pub object_id: u32,
    pub v2: [f32; 3],
    pub _pad1: u32,
    pub n0: [f32; 3],
    pub _pad2: u32,
    pub n1: [f32; 3],
    pub _pad3: u32,
    pub n2: [f32; 3],
    pub _pad4: u32,
}

/// Standard Surface material params for GPU (144 bytes).
///
/// Matches StandardSurfaceParams layout from the rasterizer.
/// All colors use vec4 packing: rgb = color, a = weight.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuMaterial {
    /// Base color (rgb) and weight (a)
    pub base_color_weight: [f32; 4],
    /// Specular color (rgb) and weight (a)
    pub specular_color_weight: [f32; 4],
    /// Transmission color (rgb) and weight (a)
    pub transmission_color_weight: [f32; 4],
    /// Subsurface color (rgb) and weight (a)
    pub subsurface_color_weight: [f32; 4],
    /// Coat color (rgb) and weight (a)
    pub coat_color_weight: [f32; 4],
    /// Emission color (rgb) and weight (a)
    pub emission_color_weight: [f32; 4],
    /// Opacity (rgb), a unused
    pub opacity: [f32; 4],
    /// x=diffuse_roughness, y=metalness, z=specular_roughness, w=specular_IOR
    pub params1: [f32; 4],
    /// x=specular_anisotropy, y=coat_roughness, z=coat_IOR, w=visible (0=hidden, 1=visible)
    pub params2: [f32; 4],
}

/// CPU-side triangle used during BVH build (before GPU upload).
#[derive(Debug, Clone)]
pub struct Triangle {
    pub v0: [f32; 3],
    pub v1: [f32; 3],
    pub v2: [f32; 3],
    pub n0: [f32; 3],
    pub n1: [f32; 3],
    pub n2: [f32; 3],
    pub material_id: u32,
    pub object_id: u32,
}

impl Triangle {
    /// Compute AABB of this triangle.
    pub fn aabb(&self) -> Aabb {
        let mut b = Aabb::EMPTY;
        b.grow_point(self.v0);
        b.grow_point(self.v1);
        b.grow_point(self.v2);
        b
    }

    /// Centroid of the triangle.
    pub fn centroid(&self) -> [f32; 3] {
        [
            (self.v0[0] + self.v1[0] + self.v2[0]) / 3.0,
            (self.v0[1] + self.v1[1] + self.v2[1]) / 3.0,
            (self.v0[2] + self.v1[2] + self.v2[2]) / 3.0,
        ]
    }

    /// Convert to GPU-friendly packed format.
    pub fn to_gpu(&self) -> GpuTriangle {
        GpuTriangle {
            v0: self.v0,
            material_id: self.material_id,
            v1: self.v1,
            object_id: self.object_id,
            v2: self.v2,
            _pad1: 0,
            n0: self.n0,
            _pad2: 0,
            n1: self.n1,
            _pad3: 0,
            n2: self.n2,
            _pad4: 0,
        }
    }
}
