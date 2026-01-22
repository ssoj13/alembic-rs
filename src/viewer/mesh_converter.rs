//! Convert Alembic geometry to GPU-ready data

use crate::geom::{IPolyMesh, PolyMeshSample, ICurves, CurvesSample, ISubD, IPoints, PointsSample, ICamera, ILight};
use crate::material::{IMaterial, get_material_assignment};
use super::smooth_normals::SmoothNormalData;
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
        if self.is_valid() {
            (self.min + self.max) * 0.5
        } else {
            Vec3::ZERO
        }
    }
    
    pub fn radius(&self) -> f32 {
        if self.is_valid() {
            (self.max - self.min).length() * 0.5
        } else {
            0.0
        }
    }
    
    /// Check if bounds are valid (not empty, no NaN/Inf)
    pub fn is_valid(&self) -> bool {
        // Check min <= max for all axes
        let ordered = self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z;
        // Check no NaN or Inf values
        let finite = self.min.is_finite() && self.max.is_finite();
        // Check not empty (initialized from empty())
        let not_empty = self.min.x < f32::MAX && self.max.x > f32::MIN;
        ordered && finite && not_empty
    }
}

/// Converted mesh data ready for GPU
pub struct ConvertedMesh {
    pub path: String,  // full object path
    pub vertices: Arc<Vec<Vertex>>,  // Arc for cheap cloning from cache
    pub indices: Arc<Vec<u32>>,
    pub transform: Mat4,
    pub bounds: Bounds,
    // Material properties (from assigned material if any)
    pub base_color: Option<Vec3>,
    pub metallic: Option<f32>,
    pub roughness: Option<f32>,
    // Data for dynamic smooth normal recalculation
    pub smooth_data: Option<SmoothNormalData>,
}

/// Converted curves data ready for GPU (as line strips)
pub struct ConvertedCurves {
    pub path: String,
    pub vertices: Arc<Vec<Vertex>>,
    pub indices: Arc<Vec<u32>>,
    pub transform: Mat4,
}

/// Converted points data ready for GPU
pub struct ConvertedPoints {
    pub path: String,
    pub positions: Arc<Vec<[f32; 3]>>,
    #[allow(dead_code)]
    pub widths: Arc<Vec<f32>>,  // radius per point
    pub transform: Mat4,
    pub bounds: Bounds,
}

/// Scene camera from Alembic file
#[derive(Clone, Debug)]
pub struct SceneCamera {
    pub name: String,
    pub transform: Mat4,
    /// Focal length in mm
    pub focal_length: f32,
    /// Horizontal aperture in cm
    #[allow(dead_code)]
    pub h_aperture: f32,
    /// Vertical aperture in cm
    pub v_aperture: f32,
    /// Near clip
    pub near: f32,
    /// Far clip
    pub far: f32,
}

impl SceneCamera {
    /// Compute vertical FOV in radians
    pub fn fov_y(&self) -> f32 {
        // aperture in cm, focal length in mm -> convert to same units
        // fov = 2 * atan(aperture / (2 * focal_length))
        2.0 * (self.v_aperture / (2.0 * self.focal_length / 10.0)).atan()
    }
    
    /// Compute aspect ratio
    #[allow(dead_code)]
    pub fn aspect(&self) -> f32 {
        self.h_aperture / self.v_aperture
    }
}

/// Scene light from Alembic file
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SceneLight {
    pub name: String,
    pub transform: Mat4,
    /// Position in world space (extracted from transform)
    pub position: Vec3,
    /// Direction (Z-axis of transform, normalized)
    pub direction: Vec3,
    /// Color (default white, could be read from user properties)
    pub color: Vec3,
    /// Intensity (derived from camera params or default)
    pub intensity: f32,
}

impl SceneLight {
    /// Create from transform matrix
    pub fn from_transform(name: String, transform: Mat4) -> Self {
        let position = Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);
        // Light direction is negative Z (looking down -Z in local space)
        let dir = transform.transform_vector3(-Vec3::Z).normalize_or_zero();
        Self {
            name,
            transform,
            position,
            direction: dir,
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }
}

/// Scene material from Alembic file
#[derive(Clone, Debug)]
pub struct SceneMaterial {
    #[allow(dead_code)]
    pub name: String,
    pub path: String,
    /// Base color (extracted from shader params if available)
    pub base_color: Option<Vec3>,
    /// Metallic (0.0 = dielectric, 1.0 = metal)
    pub metallic: Option<f32>,
    /// Roughness (0.0 = mirror, 1.0 = diffuse)
    pub roughness: Option<f32>,
    /// Path to parent material (for inheritance)
    pub inherits_path: Option<String>,
    /// Targets (e.g., "arnold", "renderman")
    #[allow(dead_code)]
    pub targets: Vec<String>,
}

/// Material assignment on geometry
#[derive(Clone, Debug)]
pub struct MaterialAssignment {
    pub object_path: String,
    pub material_path: String,
}

/// Convert CurvesSample to line strips for GPU
pub fn convert_curves(sample: &CurvesSample, path: &str, transform: Mat4) -> Option<ConvertedCurves> {
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
        path: path.to_string(),
        vertices: Arc::new(vertices),
        indices: Arc::new(indices),
        transform,
    })
}

/// Convert PolyMeshSample to triangulated GPU mesh
pub fn convert_polymesh(sample: &PolyMeshSample, transform: Mat4) -> Option<ConvertedMesh> {
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
    // For smooth normals: position and face normal per vertex
    let mut smooth_positions = Vec::with_capacity(tri_count * 3);
    let mut smooth_face_normals = Vec::with_capacity(tri_count * 3);
    
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
            
            // Compute geometric face normal for this triangle
            let edge1 = p1 - p0;
            let edge2 = p2 - p0;
            let geo_face_normal = edge1.cross(edge2).normalize_or_zero();
            
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
            
            // Store data for smooth normals recalculation
            smooth_positions.push(p0);
            smooth_positions.push(p1);
            smooth_positions.push(p2);
            smooth_face_normals.push(geo_face_normal);
            smooth_face_normals.push(geo_face_normal);
            smooth_face_normals.push(geo_face_normal);
            
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
    
    // Build smooth normal data for dynamic recalculation
    let smooth_data = SmoothNormalData::from_vertices(&smooth_positions, &smooth_face_normals);
    
    Some(ConvertedMesh {
        path: String::new(),  // set by caller
        vertices: Arc::new(vertices),
        indices: Arc::new(indices),
        transform,
        bounds,
        base_color: None,  // set by apply_materials
        metallic: None,
        roughness: None,
        smooth_data: Some(smooth_data),
    })
}

/// Convert PointsSample to GPU points
pub fn convert_points(sample: &PointsSample, path: &str, transform: Mat4) -> Option<ConvertedPoints> {
    if !sample.is_valid() {
        return None;
    }
    
    let positions: Vec<[f32; 3]> = sample.positions.iter()
        .map(|p| [p.x, p.y, p.z])
        .collect();
    
    // Use widths if available, otherwise default radius
    let widths = if sample.has_widths() {
        sample.widths.clone()
    } else {
        vec![0.02; sample.positions.len()]  // default 2cm radius
    };
    
    // Compute bounds
    let mut bounds = Bounds::empty();
    for pos in &sample.positions {
        let world_pos = transform.transform_point3(*pos);
        bounds.expand(world_pos);
    }
    
    Some(ConvertedPoints {
        path: path.to_string(),
        positions: Arc::new(positions),
        widths: Arc::new(widths),
        transform,
        bounds,
    })
}

/// Collected scene data
pub struct CollectedScene {
    pub meshes: Vec<ConvertedMesh>,
    pub curves: Vec<ConvertedCurves>,
    pub points: Vec<ConvertedPoints>,
    pub cameras: Vec<SceneCamera>,
    pub lights: Vec<SceneLight>,
    pub is_static: bool,
    #[allow(dead_code)]  // used internally for color resolution
    pub materials: Vec<SceneMaterial>,
    #[allow(dead_code)]  // used internally for color resolution
    pub material_assignments: Vec<MaterialAssignment>,
}

/// Cached mesh data for constant geometry
#[derive(Clone)]
pub(crate) struct CachedMesh {
    vertices: Arc<Vec<Vertex>>,  // Arc for zero-copy sharing
    indices: Arc<Vec<u32>>,
    local_bounds: Bounds,
}

/// Cached curves data for constant geometry
#[derive(Clone)]
pub(crate) struct CachedCurves {
    vertices: Arc<Vec<Vertex>>,
    indices: Arc<Vec<u32>>,
}

/// Cached points data for constant geometry
#[derive(Clone)]
pub(crate) struct CachedPoints {
    positions: Arc<Vec<[f32; 3]>>,
    widths: Arc<Vec<f32>>,
    local_bounds: Bounds,
}

/// Thread-safe caches for constant geometry
pub struct MeshCacheData {
    meshes: HashMap<String, CachedMesh>,
    curves: HashMap<String, CachedCurves>,
    points: HashMap<String, CachedPoints>,
}

/// Thread-safe cache handle
pub type MeshCache = Arc<Mutex<MeshCacheData>>;

/// Create a new empty mesh cache
pub fn new_mesh_cache() -> MeshCache {
    Arc::new(Mutex::new(MeshCacheData {
        meshes: HashMap::new(),
        curves: HashMap::new(),
        points: HashMap::new(),
    }))
}


/// Pending mesh conversion task
struct MeshTask {
    path: String,
    sample: PolyMeshSample,
    transform: Mat4,
    is_constant: bool,
}

/// Cached mesh result (from cache hit)
struct CachedResult {
    path: String,
    vertices: Arc<Vec<Vertex>>,
    indices: Arc<Vec<u32>>,
    transform: Mat4,
    local_bounds: Bounds,
}

struct CachedCurvesResult {
    path: String,
    vertices: Arc<Vec<Vertex>>,
    indices: Arc<Vec<u32>>,
    transform: Mat4,
}

struct CachedPointsResult {
    path: String,
    positions: Arc<Vec<[f32; 3]>>,
    widths: Arc<Vec<f32>>,
    transform: Mat4,
    local_bounds: Bounds,
}

/// Recursively collect all geometry with optional caching for constant meshes
pub fn collect_scene_cached(archive: &crate::abc::IArchive, sample_index: usize, cache: Option<&MeshCache>) -> CollectedScene {
    let mut mesh_tasks = Vec::new();
    let mut cached_results = Vec::new();
    let mut cached_curve_results = Vec::new();
    let mut cached_point_results = Vec::new();
    let mut curves = Vec::new();
    let mut points = Vec::new();
    let mut cameras = Vec::new();
    let mut lights = Vec::new();
    let mut materials = Vec::new();
    let mut material_assignments = Vec::new();
    let mut has_animation = false;
    let root = archive.getTop();

    // Phase 1: Collect all mesh samples, curves, points, cameras, lights, materials (sequential file reads)
    collect_samples_recursive(
        &root,
        Mat4::IDENTITY,
        sample_index,
        &mut mesh_tasks,
        &mut cached_results,
        &mut cached_curve_results,
        &mut cached_point_results,
        &mut curves,
        &mut points,
        &mut cameras,
        &mut lights,
        &mut materials,
        &mut material_assignments,
        &mut has_animation,
        cache,
    );
    
    // Phase 2: Convert meshes in parallel (CPU-bound)
    let converted: Vec<ConvertedMesh> = mesh_tasks
        .into_par_iter()
        .filter_map(|task| {
            convert_polymesh(&task.sample, task.transform).map(|mut converted| {
                // Set path from task
                converted.path = task.path.clone();
                
                // Cache constant meshes
                if task.is_constant {
                    if let Some(cache) = cache {
                        let mut local_bounds = Bounds::empty();
                        for pos in &task.sample.positions {
                            local_bounds.expand(*pos);
                        }
                        // Use path as cache key for uniqueness
                        cache.lock().meshes.insert(task.path.clone(), CachedMesh {
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
            path: cached.path,
            vertices: cached.vertices,
            indices: cached.indices,
            transform: cached.transform,
            bounds,
            base_color: None,  // set by apply_materials
            metallic: None,
            roughness: None,
            smooth_data: None,  // cached meshes don't store smooth data
        });
    }
    
    meshes.extend(converted);

    // Add cached curves
    for cached in cached_curve_results {
        curves.push(ConvertedCurves {
            path: cached.path,
            vertices: cached.vertices,
            indices: cached.indices,
            transform: cached.transform,
        });
    }

    // Add cached points
    for cached in cached_point_results {
        let bounds = transform_bounds(&cached.local_bounds, cached.transform);
        points.push(ConvertedPoints {
            path: cached.path,
            positions: cached.positions,
            widths: cached.widths,
            transform: cached.transform,
            bounds,
        });
    }
    
    // Resolve material inheritance FIRST (copy values from parent materials)
    // Must happen before building mat_props lookup, otherwise inherited values won't propagate
    resolve_material_inheritance(&mut materials);
    
    // Build lookup: material_path -> material properties (after inheritance resolved)
    let mat_props: std::collections::HashMap<&str, &SceneMaterial> = materials.iter()
        .map(|m| (m.path.as_str(), m))
        .collect();
    
    // Build lookup: object_path -> material_path
    let obj_to_mat: std::collections::HashMap<&str, &str> = material_assignments.iter()
        .map(|a| (a.object_path.as_str(), a.material_path.as_str()))
        .collect();
    
    // Apply material properties to meshes
    for mesh in &mut meshes {
        if let Some(&mat_path) = obj_to_mat.get(mesh.path.as_str()) {
            if let Some(&mat) = mat_props.get(mat_path) {
                mesh.base_color = mat.base_color;
                mesh.metallic = mat.metallic;
                mesh.roughness = mat.roughness;
            }
        }
    }
    
    CollectedScene {
        meshes,
        curves,
        points,
        cameras,
        lights,
        is_static: !has_animation,
        materials,
        material_assignments,
    }
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

fn bounds_from_vec3(positions: &[Vec3]) -> Bounds {
    let mut bounds = Bounds::empty();
    for pos in positions {
        bounds.expand(*pos);
    }
    bounds
}

/// Phase 1: Collect all mesh samples (sequential reads from file)
#[allow(clippy::too_many_arguments)]
fn collect_samples_recursive(
    obj: &crate::abc::IObject,
    parent_transform: Mat4,
    sample_index: usize,
    mesh_tasks: &mut Vec<MeshTask>,
    cached_results: &mut Vec<CachedResult>,
    cached_curve_results: &mut Vec<CachedCurvesResult>,
    cached_point_results: &mut Vec<CachedPointsResult>,
    curves: &mut Vec<ConvertedCurves>,
    points: &mut Vec<ConvertedPoints>,
    cameras: &mut Vec<SceneCamera>,
    lights: &mut Vec<SceneLight>,
    materials: &mut Vec<SceneMaterial>,
    material_assignments: &mut Vec<MaterialAssignment>,
    has_animation: &mut bool,
    cache: Option<&MeshCache>,
) {
    // Check if this object is an Xform
    let (local_transform, inherits) = if let Some(xform) = crate::geom::IXform::new(obj) {
        let num_samples = xform.getNumSamples();
        let sample_idx = if num_samples > 0 {
            // Clamp to last sample to mirror SampleSelector behavior.
            sample_index.min(num_samples - 1)
        } else {
            0
        };
        if num_samples > 1 && !xform.isConstant() {
            *has_animation = true;
        }
        if num_samples > 0 {
            if let Ok(sample) = xform.getSample(sample_idx) {
                (sample.matrix(), sample.inherits)
            } else {
                (Mat4::IDENTITY, true)
            }
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
        let mesh_path = polymesh.getFullName().to_string();
        let is_constant = polymesh.isConstant();
        let num_samples = polymesh.getNumSamples();
        if num_samples > 1 && !is_constant {
            *has_animation = true;
        }
        let sample_idx = if num_samples > 0 {
            // Clamp to last sample to mirror SampleSelector behavior.
            sample_index.min(num_samples - 1)
        } else {
            0
        };
        
        // Try cache first for constant meshes
        // IMPORTANT: Use mesh_path as key, not mesh_name - different objects may have same name
        // but different world-space positions (e.g., brake_discShape in multiple wheels)
        let cached = if is_constant {
            cache.and_then(|c| c.lock().meshes.get(&mesh_path).cloned())
        } else {
            None
        };
        
        if let Some(cached_mesh) = cached {
            cached_results.push(CachedResult {
                path: mesh_path,
                vertices: cached_mesh.vertices,
                indices: cached_mesh.indices,
                transform: world_transform,
                local_bounds: cached_mesh.local_bounds,
            });
        } else if num_samples > 0 {
            if let Ok(sample) = polymesh.getSample(sample_idx) {
                mesh_tasks.push(MeshTask {
                    path: mesh_path,
                    sample,
                    transform: world_transform,
                    is_constant,
                });
            }
        }
    }
    
    // Check if this object is a SubD (treat as polymesh)
    if let Some(subd) = ISubD::new(obj) {
        let mesh_path = subd.getFullName().to_string();
        let is_constant = subd.isConstant();
        let num_samples = subd.getNumSamples();
        if num_samples > 1 && !is_constant {
            *has_animation = true;
        }
        let sample_idx = if num_samples > 0 {
            // Clamp to last sample to mirror SampleSelector behavior.
            sample_index.min(num_samples - 1)
        } else {
            0
        };
        
        // Use mesh_path as key for SubD too
        let cached = if is_constant {
            cache.and_then(|c| c.lock().meshes.get(&mesh_path).cloned())
        } else {
            None
        };
        
        if let Some(cached_mesh) = cached {
            cached_results.push(CachedResult {
                path: mesh_path,
                vertices: cached_mesh.vertices,
                indices: cached_mesh.indices,
                transform: world_transform,
                local_bounds: cached_mesh.local_bounds,
            });
        } else if num_samples > 0 {
            if let Ok(sample) = subd.getSample(sample_idx) {
                // Convert SubD sample to PolyMesh sample for the task.
                let poly_sample = PolyMeshSample {
                    positions: sample.positions,
                    face_counts: sample.face_counts,
                    face_indices: sample.face_indices,
                    velocities: sample.velocities,
                    uvs: sample.uvs,
                    normals: sample.normals,
                    normals_is_simple_array: false,
                    self_bounds: sample.self_bounds,
                };
                mesh_tasks.push(MeshTask {
                    path: mesh_path,
                    sample: poly_sample,
                    transform: world_transform,
                    is_constant,
                });
            }
        }
    }
    
    // Check if this object is Curves
    if let Some(icurves) = ICurves::new(obj) {
        let curve_path = icurves.getFullName().to_string();
        let is_constant = icurves.isConstant();
        let num_samples = icurves.getNumSamples();
        if num_samples > 1 && !is_constant {
            *has_animation = true;
        }
        let sample_idx = if num_samples > 0 {
            // Clamp to last sample to mirror SampleSelector behavior.
            sample_index.min(num_samples - 1)
        } else {
            0
        };
        let cached = if is_constant {
            cache.and_then(|c| c.lock().curves.get(&curve_path).cloned())
        } else {
            None
        };
        if let Some(cached_curves) = cached {
            cached_curve_results.push(CachedCurvesResult {
                path: curve_path,
                vertices: cached_curves.vertices,
                indices: cached_curves.indices,
                transform: world_transform,
            });
        } else if num_samples > 0 {
            if let Ok(sample) = icurves.getSample(sample_idx) {
                if let Some(converted) = convert_curves(&sample, icurves.getFullName(), world_transform) {
                    if is_constant {
                        if let Some(cache) = cache {
                            cache.lock().curves.insert(converted.path.clone(), CachedCurves {
                                vertices: Arc::clone(&converted.vertices),
                                indices: Arc::clone(&converted.indices),
                            });
                        }
                    }
                    curves.push(converted);
                }
            }
        }
    }
    
    // Check if this object is Points
    if let Some(ipoints) = IPoints::new(obj) {
        let points_path = ipoints.getFullName().to_string();
        let is_constant = ipoints.isConstant();
        let num_samples = ipoints.getNumSamples();
        if num_samples > 1 && !is_constant {
            *has_animation = true;
        }
        let sample_idx = if num_samples > 0 {
            // Clamp to last sample to mirror SampleSelector behavior.
            sample_index.min(num_samples - 1)
        } else {
            0
        };
        let cached = if is_constant {
            cache.and_then(|c| c.lock().points.get(&points_path).cloned())
        } else {
            None
        };
        if let Some(cached_points) = cached {
            cached_point_results.push(CachedPointsResult {
                path: points_path,
                positions: cached_points.positions,
                widths: cached_points.widths,
                transform: world_transform,
                local_bounds: cached_points.local_bounds,
            });
        } else if num_samples > 0 {
            if let Ok(sample) = ipoints.getSample(sample_idx) {
                if let Some(converted) = convert_points(&sample, ipoints.getFullName(), world_transform) {
                    if is_constant {
                        if let Some(cache) = cache {
                            let local_bounds = bounds_from_vec3(&sample.positions);
                            cache.lock().points.insert(converted.path.clone(), CachedPoints {
                                positions: Arc::clone(&converted.positions),
                                widths: Arc::clone(&converted.widths),
                                local_bounds,
                            });
                        }
                    }
                    points.push(converted);
                }
            }
        }
    }
    
    // Check if this object is a Camera
    if let Some(icamera) = ICamera::new(obj) {
        let num_samples = icamera.getNumSamples();
        if num_samples > 1 && !icamera.isConstant() {
            *has_animation = true;
        }
        let sample_idx = if num_samples > 0 {
            // Clamp to last sample to mirror SampleSelector behavior.
            sample_index.min(num_samples - 1)
        } else {
            0
        };
        if num_samples > 0 {
            if let Ok(sample) = icamera.getSample(sample_idx) {
                cameras.push(SceneCamera {
                    name: icamera.getName().to_string(),
                    transform: world_transform,
                    focal_length: sample.focal_length as f32,
                    h_aperture: sample.horizontal_aperture as f32,
                    v_aperture: sample.vertical_aperture as f32,
                    near: sample.near_clipping_plane as f32,
                    far: sample.far_clipping_plane as f32,
                });
            }
        }
    }

    // Check if this object is a Light
    if let Some(ilight) = ILight::new(obj) {
        let num_samples = ilight.getNumSamples();
        if num_samples > 1 && !ilight.isConstant() {
            *has_animation = true;
        }
        // Lights use transform for position/direction
        lights.push(SceneLight::from_transform(
            ilight.getName().to_string(),
            world_transform,
        ));
    }

    // Check if this object is a Material
    if let Some(imat) = IMaterial::new(obj) {
        let targets = imat.target_names();
        let flattened = imat.flatten();
        
        // Helper to extract param value from any target's surface shader
        let find_float = |names: &[&str]| -> Option<f32> {
            targets.iter().find_map(|target| {
                let network = flattened.networks.get(target)?;
                let surface = network.surface_shader()?;
                names.iter().find_map(|name| surface.param(name).and_then(|p| p.as_float()))
            })
        };
        let find_vec3 = |names: &[&str]| -> Option<Vec3> {
            targets.iter().find_map(|target| {
                let network = flattened.networks.get(target)?;
                let surface = network.surface_shader()?;
                names.iter().find_map(|name| surface.param(name).and_then(|p| p.as_vec3()))
            })
        };
        
        // Extract material params
        let base_color = find_vec3(&["base_color", "diffuse_color", "color"]);
        let metallic = find_float(&["metallic", "metalness", "metal"]);
        let roughness = find_float(&["roughness", "specular_roughness", "diffuse_roughness"]);
        
        materials.push(SceneMaterial {
            name: imat.getName().to_string(),
            path: imat.getFullName().to_string(),
            base_color,
            metallic,
            roughness,
            inherits_path: imat.inherits_path(),
            targets,
        });
    }

    // Check for material assignment on this object
    if let Some(mat_path) = get_material_assignment(obj) {
        material_assignments.push(MaterialAssignment {
            object_path: obj.getFullName().to_string(),
            material_path: mat_path,
        });
    }

    // Recurse into children
    for child in obj.getChildren() {
        collect_samples_recursive(
            &child,
            world_transform,
            sample_index,
            mesh_tasks,
            cached_results,
            cached_curve_results,
            cached_point_results,
            curves,
            points,
            cameras,
            lights,
            materials,
            material_assignments,
            has_animation,
            cache,
        );
    }
}

/// Get mesh statistics
pub struct MeshStats {
    pub mesh_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
}

/// Resolve material inheritance - fill in missing values from parent materials
#[allow(clippy::type_complexity)]  // HashMap with tuple is clear enough for local use
pub fn resolve_material_inheritance(materials: &mut [SceneMaterial]) {
    // Resolve in multiple passes (for chains)
    for _ in 0..10 {  // Max 10 levels of inheritance
        // Build lookup each pass (properties may have changed)
        let parent_data: HashMap<String, (Option<Vec3>, Option<f32>, Option<f32>)> = materials.iter()
            .map(|m| (m.path.clone(), (m.base_color, m.metallic, m.roughness)))
            .collect();
        
        let mut changes = false;
        
        for mat in materials.iter_mut() {
            if let Some(parent_path) = &mat.inherits_path {
                if let Some(&(parent_color, parent_metallic, parent_roughness)) = parent_data.get(parent_path) {
                    // Copy missing values from parent
                    if mat.base_color.is_none() && parent_color.is_some() {
                        mat.base_color = parent_color;
                        changes = true;
                    }
                    if mat.metallic.is_none() && parent_metallic.is_some() {
                        mat.metallic = parent_metallic;
                        changes = true;
                    }
                    if mat.roughness.is_none() && parent_roughness.is_some() {
                        mat.roughness = parent_roughness;
                        changes = true;
                    }
                }
            }
        }
        
        if !changes {
            break;
        }
    }
}

pub fn compute_stats(meshes: &[ConvertedMesh]) -> MeshStats {
    MeshStats {
        mesh_count: meshes.len(),
        vertex_count: meshes.iter().map(|m| m.vertices.len()).sum(),
        triangle_count: meshes.iter().map(|m| m.indices.len() / 3).sum(),
    }
}

/// Compute combined bounds of all meshes and points
pub fn compute_scene_bounds(meshes: &[ConvertedMesh], points: &[ConvertedPoints]) -> Bounds {
    let mut bounds = Bounds::empty();
    for mesh in meshes {
        bounds.merge(&mesh.bounds);
    }
    for pts in points {
        bounds.merge(&pts.bounds);
    }
    bounds
}
