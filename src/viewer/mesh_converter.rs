//! Convert Alembic geometry to GPU-ready data

use crate::geom::{IPolyMesh, PolyMeshSample, ICurves, CurvesSample};
use glam::{Mat4, Vec3};
use rayon::prelude::*;
use standard_surface::Vertex;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;  // faster than std::sync::Mutex

/// Axis-aligned bounding box
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::MAX),
            max: Vec3::splat(f32::MIN),
        }
    }
    
    pub fn expand(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }
    
    pub fn merge(&mut self, other: &Bounds) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
    
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    
    pub fn radius(&self) -> f32 {
        (self.max - self.min).length() * 0.5
    }
    
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
}

/// Converted mesh data ready for GPU
pub struct ConvertedMesh {
    pub name: String,
    pub vertices: Arc<Vec<Vertex>>,  // Arc for cheap cloning from cache
    pub indices: Arc<Vec<u32>>,
    pub transform: Mat4,
    pub bounds: Bounds,
}

/// Converted curves data ready for GPU (as line strips)
pub struct ConvertedCurves {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub transform: Mat4,
}

/// Convert CurvesSample to line strips for GPU
pub fn convert_curves(sample: &CurvesSample, name: &str, transform: Mat4) -> Option<ConvertedCurves> {
    if !sample.is_valid() {
        return None;
    }
    
    let positions = &sample.positions;
    let num_vertices = &sample.num_vertices;
    
    // Default color for curves (white)

    
    // Build vertices and line indices
    let mut vertices = Vec::with_capacity(positions.len());
    let mut indices = Vec::new();
    let mut vertex_offset = 0u32;
    
    for &count in num_vertices {
        let count = count as usize;
        if count < 2 {
            vertex_offset += count as u32;
            continue;
        }
        
        // Add vertices for this curve
        for i in 0..count {
            let pos_idx = vertex_offset as usize + i;
            if pos_idx >= positions.len() {
                break;
            }
            
            let pos = positions[pos_idx];
            
            // Get width if available (default 0.01)
            let width = if !sample.widths.is_empty() && pos_idx < sample.widths.len() {
                sample.widths[pos_idx]
            } else {
                0.01
            };
            
            vertices.push(Vertex {
                position: [pos.x, pos.y, pos.z],
                normal: [0.0, 1.0, 0.0], // Up vector as default
                uv: [i as f32 / count as f32, width], // Store width in UV.y
            });
        }
        
        // Add line strip indices (pairs for LINE_LIST)
        for i in 0..(count - 1) {
            indices.push(vertex_offset + i as u32);
            indices.push(vertex_offset + i as u32 + 1);
        }
        
        vertex_offset += count as u32;
    }
    
    if vertices.is_empty() {
        return None;
    }
    
    Some(ConvertedCurves {
        name: name.to_string(),
        vertices,
        indices,
        transform,
    })
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
    
    // Compute world-space bounds
    let mut bounds = Bounds::empty();
    for pos in positions {
        let world_pos = transform.transform_point3(*pos);
        bounds.expand(world_pos);
    }
    
    Some(ConvertedMesh {
        name: name.to_string(),
        vertices: Arc::new(vertices),
        indices: Arc::new(indices),
        transform,
        bounds,
    })
}

/// Collected scene data
pub struct CollectedScene {
    pub meshes: Vec<ConvertedMesh>,
    pub curves: Vec<ConvertedCurves>,
}

/// Cached mesh data for constant geometry
#[derive(Clone)]
pub(crate) struct CachedMesh {
    vertices: Arc<Vec<Vertex>>,  // Arc for zero-copy sharing
    indices: Arc<Vec<u32>>,
    local_bounds: Bounds,
}

/// Thread-safe mesh cache for constant geometry
pub type MeshCache = Arc<Mutex<HashMap<String, CachedMesh>>>;

/// Create a new empty mesh cache
pub fn new_mesh_cache() -> MeshCache {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Recursively collect all geometry from an archive (without caching)
pub fn collect_scene(archive: &crate::abc::IArchive, sample_index: usize) -> CollectedScene {
    collect_scene_cached(archive, sample_index, None)
}

/// Pending mesh conversion task
struct MeshTask {
    name: String,
    sample: PolyMeshSample,
    transform: Mat4,
    is_constant: bool,
}

/// Cached mesh result (from cache hit)
struct CachedResult {
    name: String,
    vertices: Arc<Vec<Vertex>>,
    indices: Arc<Vec<u32>>,
    transform: Mat4,
    local_bounds: Bounds,
}

/// Recursively collect all geometry with optional caching for constant meshes
pub fn collect_scene_cached(archive: &crate::abc::IArchive, sample_index: usize, cache: Option<&MeshCache>) -> CollectedScene {
    let mut mesh_tasks = Vec::new();
    let mut cached_results = Vec::new();
    let mut curves = Vec::new();
    let root = archive.root();
    
    // Phase 1: Collect all mesh samples and curves (sequential file reads)
    collect_samples_recursive(
        &root, 
        Mat4::IDENTITY, 
        sample_index, 
        &mut mesh_tasks, 
        &mut cached_results, 
        &mut curves, 
        cache,
    );
    
    // Phase 2: Convert meshes in parallel (CPU-bound)
    let converted: Vec<ConvertedMesh> = mesh_tasks
        .into_par_iter()
        .filter_map(|task| {
            convert_polymesh(&task.sample, &task.name, task.transform).map(|converted| {
                // Cache constant meshes
                if task.is_constant {
                    if let Some(cache) = cache {
                        let mut local_bounds = Bounds::empty();
                        for pos in &task.sample.positions {
                            local_bounds.expand(*pos);
                        }
                        cache.lock().insert(task.name.clone(), CachedMesh {
                            vertices: Arc::clone(&converted.vertices),
                            indices: Arc::clone(&converted.indices),
                            local_bounds,
                        });
                    }
                }
                converted
            })
        })
        .collect();
    
    // Phase 3: Combine results - cached meshes + converted meshes
    let mut meshes = Vec::with_capacity(cached_results.len() + converted.len());
    
    // Add cached results (just need to transform bounds)
    for cached in cached_results {
        let bounds = transform_bounds(&cached.local_bounds, cached.transform);
        meshes.push(ConvertedMesh {
            name: cached.name,
            vertices: cached.vertices,
            indices: cached.indices,
            transform: cached.transform,
            bounds,
        });
    }
    
    meshes.extend(converted);
    
    CollectedScene { meshes, curves }
}

/// Transform local bounds to world space
fn transform_bounds(local: &Bounds, transform: Mat4) -> Bounds {
    let corners = [
        Vec3::new(local.min.x, local.min.y, local.min.z),
        Vec3::new(local.max.x, local.min.y, local.min.z),
        Vec3::new(local.min.x, local.max.y, local.min.z),
        Vec3::new(local.max.x, local.max.y, local.min.z),
        Vec3::new(local.min.x, local.min.y, local.max.z),
        Vec3::new(local.max.x, local.min.y, local.max.z),
        Vec3::new(local.min.x, local.max.y, local.max.z),
        Vec3::new(local.max.x, local.max.y, local.max.z),
    ];
    let mut bounds = Bounds::empty();
    for corner in corners {
        bounds.expand(transform.transform_point3(corner));
    }
    bounds
}

/// Phase 1: Collect all mesh samples (sequential reads from file)
fn collect_samples_recursive(
    obj: &crate::abc::IObject,
    parent_transform: Mat4,
    sample_index: usize,
    mesh_tasks: &mut Vec<MeshTask>,
    cached_results: &mut Vec<CachedResult>,
    curves: &mut Vec<ConvertedCurves>,
    cache: Option<&MeshCache>,
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
    
    let world_transform = if inherits {
        parent_transform * local_transform
    } else {
        local_transform
    };
    
    // Check if this object is a PolyMesh
    if let Some(polymesh) = IPolyMesh::new(obj) {
        let mesh_name = polymesh.name().to_string();
        let is_constant = polymesh.is_constant();
        
        // Try cache first for constant meshes
        let cached = if is_constant {
            cache.and_then(|c| c.lock().get(&mesh_name).cloned())
        } else {
            None
        };
        
        if let Some(cached_mesh) = cached {
            // Cache hit - just store for later bounds transform
            cached_results.push(CachedResult {
                name: mesh_name,
                vertices: cached_mesh.vertices,
                indices: cached_mesh.indices,
                transform: world_transform,
                local_bounds: cached_mesh.local_bounds,
            });
        } else {
            // Cache miss - read sample and queue for parallel conversion
            if let Ok(sample) = polymesh.get_sample(sample_index) {
                mesh_tasks.push(MeshTask {
                    name: mesh_name,
                    sample,
                    transform: world_transform,
                    is_constant,
                });
            }
        }
    }
    
    // Check if this object is Curves
    if let Some(icurves) = ICurves::new(obj) {
        if let Ok(sample) = icurves.get_sample(sample_index) {
            if let Some(converted) = convert_curves(&sample, icurves.name(), world_transform) {
                curves.push(converted);
            }
        }
    }
    
    // Recurse into children
    for child in obj.children() {
        collect_samples_recursive(&child, world_transform, sample_index, mesh_tasks, cached_results, curves, cache);
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

/// Compute combined bounds of all meshes
pub fn compute_scene_bounds(meshes: &[ConvertedMesh]) -> Bounds {
    let mut bounds = Bounds::empty();
    for mesh in meshes {
        bounds.merge(&mesh.bounds);
    }
    bounds
}
