//! Convert viewer scene meshes to path tracer triangle data.
//!
//! Bridges the rasterizer's SceneMesh (Vertex + indices) to
//! the path tracer's Triangle format for BVH construction.

use super::bvh::{GpuMaterial, Triangle};

/// Extract triangles from a standard_surface Vertex/index buffer.
///
/// Each triangle gets the same material_id. Vertices are transformed
/// by the provided model matrix columns (row-major 4x4).
pub fn extract_triangles(
    vertices: &[standard_surface::Vertex],
    indices: &[u32],
    transform: &glam::Mat4,
    material_id: u32,
    object_id: u32,
) -> Vec<Triangle> {
    let mut tris = Vec::with_capacity(indices.len() / 3);

    for chunk in indices.chunks_exact(3) {
        let (i0, i1, i2) = (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);
        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            continue;
        }

        let v0 = vertices[i0];
        let v1 = vertices[i1];
        let v2 = vertices[i2];

        // Transform positions to world space
        let p0 = transform.transform_point3(glam::Vec3::from(v0.position));
        let p1 = transform.transform_point3(glam::Vec3::from(v1.position));
        let p2 = transform.transform_point3(glam::Vec3::from(v2.position));

        // Transform normals (use normal matrix = transpose(inverse(upper3x3)))
        let normal_mat = transform.inverse().transpose();
        let n0 = normal_mat.transform_vector3(glam::Vec3::from(v0.normal)).normalize_or_zero();
        let n1 = normal_mat.transform_vector3(glam::Vec3::from(v1.normal)).normalize_or_zero();
        let n2 = normal_mat.transform_vector3(glam::Vec3::from(v2.normal)).normalize_or_zero();

        tris.push(Triangle {
            v0: p0.to_array(),
            v1: p1.to_array(),
            v2: p2.to_array(),
            n0: n0.to_array(),
            n1: n1.to_array(),
            n2: n2.to_array(),
            material_id,
            object_id,
        });
    }

    tris
}

/// Convert StandardSurfaceParams to GpuMaterial for path tracing.
/// Copies all Standard Surface parameters to GPU format.
pub fn material_from_params(params: &standard_surface::StandardSurfaceParams) -> GpuMaterial {
    GpuMaterial {
        base_color_weight: params.base_color_weight.to_array(),
        specular_color_weight: params.specular_color_weight.to_array(),
        transmission_color_weight: params.transmission_color_weight.to_array(),
        subsurface_color_weight: params.subsurface_color_weight.to_array(),
        coat_color_weight: params.coat_color_weight.to_array(),
        emission_color_weight: params.emission_color_weight.to_array(),
        opacity: params.opacity.to_array(),
        params1: [
            params.params1.x,                  // diffuse_roughness
            params.params1.y,                  // metalness
            params.params1.z.max(0.04),        // specular_roughness (clamp for GGX stability)
            params.params1.w,                  // specular_IOR
        ],
        params2: [
            params.params2.x,                  // specular_anisotropy
            params.params2.y.max(0.04),        // coat_roughness
            params.params2.z,                  // coat_IOR
            1.0,                               // visible (1=vis, 0=hidden)
        ],
    }
}

/// Create a default material (grey plastic).
pub fn default_material() -> GpuMaterial {
    GpuMaterial {
        base_color_weight: [0.8, 0.8, 0.8, 1.0],
        specular_color_weight: [1.0, 1.0, 1.0, 1.0],
        transmission_color_weight: [1.0, 1.0, 1.0, 0.0],
        subsurface_color_weight: [1.0, 1.0, 1.0, 0.0],
        coat_color_weight: [1.0, 1.0, 1.0, 0.0],
        emission_color_weight: [1.0, 1.0, 1.0, 0.0],
        opacity: [1.0, 1.0, 1.0, 1.0],
        params1: [0.0, 0.0, 0.2, 1.5], // diffuse_rough, metalness, spec_rough, IOR
        params2: [0.0, 0.1, 1.5, 1.0], // anisotropy, coat_rough, coat_IOR, visible
    }
}
