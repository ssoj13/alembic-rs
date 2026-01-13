//! Convert Alembic PolyMesh to GPU-ready triangulated mesh

use crate::geom::{IPolyMesh, PolyMeshSample};
use glam::{Mat4, Vec3};
use standard_surface::Vertex;

/// Converted mesh data ready for GPU
pub struct ConvertedMesh {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub transform: Mat4,
}

/// Convert PolyMeshSample to triangulated GPU mesh
pub fn convert_polymesh(sample: &PolyMeshSample, name: &str, transform: Mat4) -> Option<ConvertedMesh> {
    if !sample.is_valid() {
        return None;
    }

    let positions = &sample.positions;
    let face_counts = &sample.face_counts;
    let face_indices = &sample.face_indices;
    
    // Get normals - compute face normals if not provided
    let normals = sample.normals.as_ref();
    let computed_normals;
    let face_normals = if normals.is_none() {
        computed_normals = sample.compute_face_normals();
        Some(&computed_normals)
    } else {
        None
    };
    
    let uvs = sample.uvs.as_ref();
    
    // Count triangles for pre-allocation
    let tri_count: usize = face_counts.iter().map(|&c| (c as usize).saturating_sub(2)).sum();
    
    let mut vertices = Vec::with_capacity(tri_count * 3);
    let mut indices = Vec::with_capacity(tri_count * 3);
    
    let mut idx_offset = 0usize;
    let mut face_idx = 0usize;
    
    for &count in face_counts {
        let count = count as usize;
        if count < 3 {
            idx_offset += count;
            face_idx += 1;
            continue;
        }
        
        // Get face vertex indices
        let face_vertex_indices: Vec<usize> = (0..count)
            .map(|i| face_indices[idx_offset + i] as usize)
            .collect();
        
        // Fan triangulation: v0, v1, v2, then v0, v2, v3, etc.
        for i in 1..count - 1 {
            let i0 = face_vertex_indices[0];
            let i1 = face_vertex_indices[i];
            let i2 = face_vertex_indices[i + 1];
            
            // Get positions
            let p0 = positions.get(i0).copied().unwrap_or(Vec3::ZERO);
            let p1 = positions.get(i1).copied().unwrap_or(Vec3::ZERO);
            let p2 = positions.get(i2).copied().unwrap_or(Vec3::ZERO);
            
            // Get normals
            let (n0, n1, n2) = if let Some(norms) = normals {
                // Per-vertex or per-face-vertex normals from file
                let fv0 = idx_offset;
                let fv1 = idx_offset + i;
                let fv2 = idx_offset + i + 1;
                
                // Try face-varying first, fall back to vertex-indexed
                if norms.len() > fv2 {
                    (norms[fv0], norms[fv1], norms[fv2])
                } else if norms.len() > i2 {
                    (norms[i0], norms[i1], norms[i2])
                } else {
                    (Vec3::Y, Vec3::Y, Vec3::Y)
                }
            } else if let Some(fn_list) = &face_normals {
                // Use computed face normal
                let fn_val = fn_list.get(face_idx).copied().unwrap_or(Vec3::Y);
                (fn_val, fn_val, fn_val)
            } else {
                (Vec3::Y, Vec3::Y, Vec3::Y)
            };
            
            // Get UVs
            let (uv0, uv1, uv2) = if let Some(uv_data) = uvs {
                let fv0 = idx_offset;
                let fv1 = idx_offset + i;
                let fv2 = idx_offset + i + 1;
                
                if uv_data.len() > fv2 {
                    (uv_data[fv0], uv_data[fv1], uv_data[fv2])
                } else {
                    (glam::Vec2::ZERO, glam::Vec2::ZERO, glam::Vec2::ZERO)
                }
            } else {
                (glam::Vec2::ZERO, glam::Vec2::ZERO, glam::Vec2::ZERO)
            };
            
            // Add vertices
            let base_idx = vertices.len() as u32;
            
            vertices.push(Vertex {
                position: p0.into(),
                normal: n0.normalize_or_zero().into(),
                uv: uv0.into(),
            });
            vertices.push(Vertex {
                position: p1.into(),
                normal: n1.normalize_or_zero().into(),
                uv: uv1.into(),
            });
            vertices.push(Vertex {
                position: p2.into(),
                normal: n2.normalize_or_zero().into(),
                uv: uv2.into(),
            });
            
            indices.push(base_idx);
            indices.push(base_idx + 1);
            indices.push(base_idx + 2);
        }
        
        idx_offset += count;
        face_idx += 1;
    }
    
    Some(ConvertedMesh {
        name: name.to_string(),
        vertices,
        indices,
        transform,
    })
}

/// Recursively collect all PolyMeshes from an archive
pub fn collect_meshes(archive: &crate::abc::IArchive, sample_index: usize) -> Vec<ConvertedMesh> {
    let mut meshes = Vec::new();
    let root = archive.root();
    collect_meshes_recursive(&root, Mat4::IDENTITY, sample_index, &mut meshes);
    meshes
}

fn collect_meshes_recursive(
    obj: &crate::abc::IObject,
    parent_transform: Mat4,
    sample_index: usize,
    meshes: &mut Vec<ConvertedMesh>,
) {
    // Check if this object is an Xform
    let (local_transform, inherits) = if let Some(xform) = crate::geom::IXform::new(obj) {
        if let Ok(sample) = xform.get_sample(sample_index) {
            (sample.matrix(), sample.inherits)
        } else {
            (Mat4::IDENTITY, true)
        }
    } else {
        (Mat4::IDENTITY, true)
    };
    
    // If inherits=false, don't multiply by parent transform
    let world_transform = if inherits {
        parent_transform * local_transform
    } else {
        local_transform
    };
    
    // Check if this object is a PolyMesh
    if let Some(polymesh) = IPolyMesh::new(obj) {
        if let Ok(sample) = polymesh.get_sample(sample_index) {
            if let Some(converted) = convert_polymesh(&sample, polymesh.name(), world_transform) {
                meshes.push(converted);
            }
        }
    }
    
    // Recurse into children
    for child in obj.children() {
        collect_meshes_recursive(&child, world_transform, sample_index, meshes);
    }
}

/// Get mesh statistics
pub struct MeshStats {
    pub mesh_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
}

pub fn compute_stats(meshes: &[ConvertedMesh]) -> MeshStats {
    MeshStats {
        mesh_count: meshes.len(),
        vertex_count: meshes.iter().map(|m| m.vertices.len()).sum(),
        triangle_count: meshes.iter().map(|m| m.indices.len() / 3).sum(),
    }
}
