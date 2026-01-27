//! Serialize BVH + triangles into GPU storage buffers.

use super::bvh::{BvhNode, GpuMaterial, GpuTriangle, Triangle};
use super::build::Bvh;

/// Complete scene data ready for GPU upload.
pub struct GpuSceneData {
    /// Flat BVH node array (bytemuck-castable).
    pub nodes: Vec<BvhNode>,
    /// Packed triangle data in BVH traversal order.
    pub triangles: Vec<GpuTriangle>,
    /// Materials array.
    pub materials: Vec<GpuMaterial>,
    /// Total triangle count.
    pub tri_count: u32,
    /// Total node count.
    pub node_count: u32,
}

/// Build GPU-ready scene data from BVH + triangles + materials.
///
/// Reorders triangles according to BVH leaf order for better GPU cache locality.
pub fn build_gpu_data(
    bvh: &Bvh,
    triangles: &[Triangle],
    materials: &[GpuMaterial],
) -> GpuSceneData {
    // Reorder triangles in BVH traversal order
    let gpu_tris: Vec<GpuTriangle> = bvh
        .tri_indices
        .iter()
        .map(|&idx| triangles[idx].to_gpu())
        .collect();

    GpuSceneData {
        nodes: bvh.nodes.clone(),
        triangles: gpu_tris,
        materials: materials.to_vec(),
        tri_count: bvh.tri_indices.len() as u32,
        node_count: bvh.nodes.len() as u32,
    }
}

/// Convert GpuSceneData to raw byte slices for wgpu buffer creation.
impl GpuSceneData {
    /// BVH nodes as bytes.
    pub fn nodes_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.nodes)
    }

    /// Triangle data as bytes.
    pub fn triangles_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.triangles)
    }

    /// Material data as bytes.
    pub fn materials_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.materials)
    }
}
