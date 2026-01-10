//! PolyMesh (polygon mesh) schema implementation.
//!
//! Provides reading of polygon mesh data from Alembic files.

use crate::abc::IObject;
use crate::core::TopologyVariance;
use crate::geom::faceset::FACESET_SCHEMA;
use crate::geom::util as geom_util;
use crate::util::{Result, BBox3d};

/// PolyMesh schema identifier.
pub const POLYMESH_SCHEMA: &str = "AbcGeom_PolyMesh_v1";

/// Polygon mesh sample data.
#[derive(Clone, Debug, Default)]
pub struct PolyMeshSample {
    /// Vertex positions (P).
    pub positions: Vec<glam::Vec3>,
    /// Face vertex counts - number of vertices per face.
    pub face_counts: Vec<i32>,
    /// Face vertex indices - indices into positions array.
    pub face_indices: Vec<i32>,
    /// Vertex velocities (optional).
    pub velocities: Option<Vec<glam::Vec3>>,
    /// UV coordinates (optional).
    pub uvs: Option<Vec<glam::Vec2>>,
    /// Normals (optional).
    pub normals: Option<Vec<glam::Vec3>>,
    /// Self bounds - bounding box of this geometry (optional).
    pub self_bounds: Option<BBox3d>,
}

impl PolyMeshSample {
    /// Create an empty sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.positions.len()
    }
    
    /// Get number of faces.
    pub fn num_faces(&self) -> usize {
        self.face_counts.len()
    }
    
    /// Get total number of face-vertex indices.
    pub fn num_indices(&self) -> usize {
        self.face_indices.len()
    }
    
    /// Check if mesh has UVs.
    pub fn has_uvs(&self) -> bool {
        self.uvs.is_some()
    }
    
    /// Check if mesh has normals.
    pub fn has_normals(&self) -> bool {
        self.normals.is_some()
    }
    
    /// Check if mesh has velocities.
    pub fn has_velocities(&self) -> bool {
        self.velocities.is_some()
    }
    
    /// Check if mesh has self bounds.
    pub fn has_self_bounds(&self) -> bool {
        self.self_bounds.is_some()
    }
    
    /// Check if this is a valid mesh (has positions and face data).
    pub fn is_valid(&self) -> bool {
        !self.positions.is_empty() && 
        !self.face_counts.is_empty() && 
        !self.face_indices.is_empty()
    }
    
    /// Calculate face normals.
    pub fn compute_face_normals(&self) -> Vec<glam::Vec3> {
        let mut normals = Vec::with_capacity(self.face_counts.len());
        let mut idx = 0usize;
        
        for &count in &self.face_counts {
            if count >= 3 {
                let i0 = self.face_indices[idx] as usize;
                let i1 = self.face_indices[idx + 1] as usize;
                let i2 = self.face_indices[idx + 2] as usize;
                
                if i0 < self.positions.len() && i1 < self.positions.len() && i2 < self.positions.len() {
                    let v0 = self.positions[i0];
                    let v1 = self.positions[i1];
                    let v2 = self.positions[i2];
                    
                    let edge1 = v1 - v0;
                    let edge2 = v2 - v0;
                    let normal = edge1.cross(edge2).normalize_or_zero();
                    normals.push(normal);
                } else {
                    normals.push(glam::Vec3::Y);
                }
            } else {
                normals.push(glam::Vec3::Y);
            }
            idx += count as usize;
        }
        
        normals
    }
    
    /// Calculate bounding box.
    pub fn compute_bounds(&self) -> (glam::Vec3, glam::Vec3) {
        geom_util::compute_bounds_vec3(&self.positions)
    }
}

/// Input PolyMesh schema reader.
pub struct IPolyMesh<'a> {
    object: &'a IObject<'a>,
}

impl<'a> IPolyMesh<'a> {
    /// Wrap an IObject as an IPolyMesh.
    /// Returns None if the object doesn't have the PolyMesh schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(POLYMESH_SCHEMA) {
            Some(Self { object })
        } else {
            None
        }
    }
    
    /// Get the underlying object.
    pub fn object(&self) -> &IObject<'a> {
        self.object
    }
    
    /// Get the object name.
    pub fn name(&self) -> &str {
        self.object.name()
    }
    
    /// Get the full path.
    pub fn full_name(&self) -> &str {
        self.object.full_name()
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        geom_util::num_samples_from_positions(self.object)
    }
    
    /// Check if this mesh is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Get the topology variance for this mesh.
    /// 
    /// Returns:
    /// - Static: Only one sample exists
    /// - Homogeneous: Topology is constant, only positions change
    /// - Heterogeneous: Topology can change between samples
    pub fn topology_variance(&self) -> TopologyVariance {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return TopologyVariance::Static;
        };
        let Some(geom) = geom_prop.as_compound() else {
            return TopologyVariance::Static;
        };
        
        // Get sample counts for positions and topology
        let p_samples = if let Some(p) = geom.property_by_name("P") {
            p.as_array().map(|a| a.num_samples()).unwrap_or(1)
        } else { 1 };
        
        let fc_samples = if let Some(fc) = geom.property_by_name(".faceCounts") {
            fc.as_array().map(|a| a.num_samples()).unwrap_or(1)
        } else { 1 };
        
        let fi_samples = if let Some(fi) = geom.property_by_name(".faceIndices") {
            fi.as_array().map(|a| a.num_samples()).unwrap_or(1)
        } else { 1 };
        
        // Determine variance
        let max_samples = p_samples.max(fc_samples).max(fi_samples);
        
        if max_samples <= 1 {
            TopologyVariance::Static
        } else if fc_samples <= 1 && fi_samples <= 1 {
            // Topology constant, only positions animated
            TopologyVariance::Homogeneous
        } else {
            // Topology can change
            TopologyVariance::Heterogeneous
        }
    }
    
    /// Get property names available on this mesh.
    pub fn property_names(&self) -> Vec<String> {
        self.object.properties().property_names()
    }
    
    /// Get the names of all FaceSets on this mesh.
    /// 
    /// FaceSets are child objects with the FaceSet schema.
    pub fn face_set_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for i in 0..self.object.num_children() {
            if let Some(header) = self.object.child_header(i) {
                if header.meta_data.get("schema").map(|s| s == FACESET_SCHEMA).unwrap_or(false) {
                    names.push(header.name.clone());
                }
            }
        }
        names
    }
    
    /// Check if this mesh has a FaceSet with the given name.
    pub fn has_face_set(&self, name: &str) -> bool {
        if let Some(child) = self.object.child_by_name(name) {
            child.matches_schema(FACESET_SCHEMA)
        } else {
            false
        }
    }
    
    /// Get a FaceSet sample by name.
    /// 
    /// This reads the FaceSet data directly without returning an IFaceSet wrapper.
    /// Use this when you need the face indices for a specific sample.
    pub fn get_face_set_sample(&self, name: &str, index: usize) -> Option<super::faceset::FaceSetSample> {
        let child = self.object.child_by_name(name)?;
        if !child.matches_schema(FACESET_SCHEMA) {
            return None;
        }
        
        let mut sample = super::faceset::FaceSetSample::new();
        
        let props = child.properties();
        let geom_prop = props.property_by_name(".geom")?;
        let geom = geom_prop.as_compound()?;
        
        // Read .faces
        if let Some(faces_prop) = geom.property_by_name(".faces") {
            if let Some(array) = faces_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.faces = super::safe_cast_vec(&data);
                }
            }
        }
        
        // Read .selfBnds if present
        if let Some(bnds_prop) = geom.property_by_name(".selfBnds") {
            if let Some(scalar) = bnds_prop.as_scalar() {
                let mut buf = [0u8; 48];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    if let Some(values) = super::safe_cast_slice::<f64>(&buf) {
                        if values.len() >= 6 {
                            sample.self_bounds = Some(BBox3d::new(
                                glam::dvec3(values[0], values[1], values[2]),
                                glam::dvec3(values[3], values[4], values[5]),
                            ));
                        }
                    }
                }
            }
        }
        
        Some(sample)
    }
    
    /// Get the exclusivity setting for a FaceSet.
    pub fn face_set_exclusivity(&self, name: &str) -> Option<super::faceset::FaceSetExclusivity> {
        let child = self.object.child_by_name(name)?;
        if !child.matches_schema(FACESET_SCHEMA) {
            return None;
        }
        
        let header = child.header();
        Some(if let Some(excl_str) = header.meta_data.get(super::faceset::FACE_EXCLUSIVITY_KEY) {
            super::faceset::FaceSetExclusivity::parse(excl_str)
        } else {
            super::faceset::FaceSetExclusivity::NonExclusive
        })
    }
    
    /// Get number of samples in a FaceSet.
    pub fn face_set_num_samples(&self, name: &str) -> usize {
        let Some(child) = self.object.child_by_name(name) else { return 0 };
        if !child.matches_schema(FACESET_SCHEMA) {
            return 0;
        }
        
        let props = child.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 1 };
        let Some(geom) = geom_prop.as_compound() else { return 1 };
        let Some(faces_prop) = geom.property_by_name(".faces") else { return 1 };
        let Some(array) = faces_prop.as_array() else { return 1 };
        array.num_samples()
    }
    
    /// Get number of FaceSets on this mesh.
    pub fn num_face_sets(&self) -> usize {
        self.face_set_names().len()
    }
    
    /// Check if this mesh has arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        geom_util::has_arb_geom_params(self.object)
    }
    
    /// Get names of arbitrary geometry parameters.
    pub fn arb_geom_param_names(&self) -> Vec<String> {
        geom_util::arb_geom_param_names(self.object)
    }
    
    /// Check if this mesh has user properties.
    pub fn has_user_properties(&self) -> bool {
        geom_util::has_user_properties(self.object)
    }
    
    /// Get names of user properties.
    pub fn user_property_names(&self) -> Vec<String> {
        geom_util::user_property_names(self.object)
    }
    
    /// Check if this mesh has child bounds property.
    pub fn has_child_bounds(&self) -> bool {
        geom_util::has_child_bounds(self.object)
    }
    
    /// Get child bounds at the given sample index.
    pub fn child_bounds(&self, index: usize) -> Option<BBox3d> {
        geom_util::read_child_bounds(self.object, index)
    }
    
    /// Get the number of child bounds samples.
    pub fn child_bounds_num_samples(&self) -> usize {
        geom_util::child_bounds_num_samples(self.object)
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        geom_util::child_bounds_time_sampling_index(self.object)
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<PolyMeshSample> {
        use crate::util::Error;
        
        let mut sample = PolyMeshSample::new();
        
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        let g = geom.as_reader();
        
        // Read core geometry data using helpers
        if let Some(pos) = geom_util::read_vec3_array(g, "P", index) {
            sample.positions = pos;
        }
        sample.velocities = geom_util::read_vec3_array(g, ".velocities", index);
        if let Some(fc) = geom_util::read_i32_array(g, ".faceCounts", index) {
            sample.face_counts = fc;
        }
        if let Some(fi) = geom_util::read_i32_array(g, ".faceIndices", index) {
            sample.face_indices = fi;
        }
        
        // Read optional attributes
        sample.normals = geom_util::read_vec3_array(g, "N", index);
        sample.uvs = geom_util::read_vec2_array(g, "uv", index);
        sample.self_bounds = geom_util::read_self_bounds(g, index);
        
        Ok(sample)
    }
    
    /// Check if this mesh has UVs.
    pub fn has_uvs(&self) -> bool {
        geom_util::has_geom_property(self.object, "uv")
    }
    
    /// Check if this mesh has normals.
    pub fn has_normals(&self) -> bool {
        geom_util::has_geom_property(self.object, "N")
    }
    
    /// Get expanded UVs at the given sample index.
    /// 
    /// If UVs are indexed, this expands them to per-face-vertex values.
    pub fn get_uvs(&self, index: usize) -> Option<Vec<glam::Vec2>> {
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")?;
        let geom = geom_prop.as_compound()?;
        let uv_prop = geom.property_by_name("uv")?;
        
        if let Some(compound) = uv_prop.as_compound() {
            // Indexed UVs - read .vals and .indices
            let vals_prop = compound.property_by_name(".vals")?;
            let array = vals_prop.as_array()?;
            let data = array.read_sample_vec(index).ok()?;
            let floats: &[f32] = bytemuck::cast_slice(&data);
            
            // Check for indices
            if let Some(idx_prop) = compound.property_by_name(".indices") {
                if let Some(idx_array) = idx_prop.as_array() {
                    if let Ok(idx_data) = idx_array.read_sample_vec(index) {
                        let indices: &[u32] = bytemuck::cast_slice(&idx_data);
                        return Some(indices.iter()
                            .map(|&i| {
                                let base = (i as usize) * 2;
                                if base + 1 < floats.len() {
                                    glam::vec2(floats[base], floats[base + 1])
                                } else {
                                    glam::Vec2::ZERO
                                }
                            })
                            .collect());
                    }
                }
            }
            
            // No indices - direct values
            Some(floats.chunks_exact(2)
                .map(|c| glam::vec2(c[0], c[1]))
                .collect())
        } else if let Some(array) = uv_prop.as_array() {
            // Non-indexed UVs
            let data = array.read_sample_vec(index).ok()?;
            let floats: &[f32] = bytemuck::cast_slice(&data);
            Some(floats.chunks_exact(2)
                .map(|c| glam::vec2(c[0], c[1]))
                .collect())
        } else {
            None
        }
    }
    
    /// Get expanded normals at the given sample index.
    /// 
    /// If normals are indexed, this expands them to per-face-vertex values.
    pub fn get_normals(&self, index: usize) -> Option<Vec<glam::Vec3>> {
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")?;
        let geom = geom_prop.as_compound()?;
        let n_prop = geom.property_by_name("N")?;
        
        if let Some(compound) = n_prop.as_compound() {
            // Indexed normals - read .vals and .indices
            let vals_prop = compound.property_by_name(".vals")?;
            let array = vals_prop.as_array()?;
            let data = array.read_sample_vec(index).ok()?;
            let floats: &[f32] = bytemuck::cast_slice(&data);
            
            // Check for indices
            if let Some(idx_prop) = compound.property_by_name(".indices") {
                if let Some(idx_array) = idx_prop.as_array() {
                    if let Ok(idx_data) = idx_array.read_sample_vec(index) {
                        let indices: &[u32] = bytemuck::cast_slice(&idx_data);
                        return Some(indices.iter()
                            .map(|&i| {
                                let base = (i as usize) * 3;
                                if base + 2 < floats.len() {
                                    glam::vec3(floats[base], floats[base + 1], floats[base + 2])
                                } else {
                                    glam::Vec3::Y
                                }
                            })
                            .collect());
                    }
                }
            }
            
            // No indices - direct values
            Some(floats.chunks_exact(3)
                .map(|c| glam::vec3(c[0], c[1], c[2]))
                .collect())
        } else if let Some(array) = n_prop.as_array() {
            // Non-indexed normals
            let data = array.read_sample_vec(index).ok()?;
            let floats: &[f32] = bytemuck::cast_slice(&data);
            Some(floats.chunks_exact(3)
                .map(|c| glam::vec3(c[0], c[1], c[2]))
                .collect())
        } else {
            None
        }
    }
    
    /// Get UV scope (how UVs are mapped to geometry).
    pub fn uvs_scope(&self) -> crate::core::GeometryScope {
        use crate::core::GeometryScope;
        
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return GeometryScope::Unknown;
        };
        let Some(geom) = geom_prop.as_compound() else {
            return GeometryScope::Unknown;
        };
        let Some(uv_prop) = geom.property_by_name("uv") else {
            return GeometryScope::Unknown;
        };
        
        if let Some(scope_str) = uv_prop.header().meta_data.get("geoScope") {
            GeometryScope::parse(scope_str)
        } else {
            GeometryScope::FaceVarying // Default for UVs
        }
    }
    
    /// Get normals scope (how normals are mapped to geometry).
    pub fn normals_scope(&self) -> crate::core::GeometryScope {
        use crate::core::GeometryScope;
        
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return GeometryScope::Unknown;
        };
        let Some(geom) = geom_prop.as_compound() else {
            return GeometryScope::Unknown;
        };
        let Some(n_prop) = geom.property_by_name("N") else {
            return GeometryScope::Unknown;
        };
        
        if let Some(scope_str) = n_prop.header().meta_data.get("geoScope") {
            GeometryScope::parse(scope_str)
        } else {
            GeometryScope::FaceVarying // Default for normals
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_polymesh_sample_empty() {
        let sample = PolyMeshSample::new();
        assert_eq!(sample.num_vertices(), 0);
        assert_eq!(sample.num_faces(), 0);
        assert!(!sample.is_valid());
    }
    
    #[test]
    fn test_polymesh_sample_triangle() {
        let mut sample = PolyMeshSample::new();
        sample.positions = vec![
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(1.0, 0.0, 0.0),
            glam::vec3(0.0, 1.0, 0.0),
        ];
        sample.face_counts = vec![3];
        sample.face_indices = vec![0, 1, 2];
        
        assert_eq!(sample.num_vertices(), 3);
        assert_eq!(sample.num_faces(), 1);
        assert!(sample.is_valid());
        
        let normals = sample.compute_face_normals();
        assert_eq!(normals.len(), 1);
        // Normal should point in Z direction for XY plane triangle
        assert!((normals[0].z - 1.0).abs() < 0.001 || (normals[0].z + 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_polymesh_bounds() {
        let mut sample = PolyMeshSample::new();
        sample.positions = vec![
            glam::vec3(-1.0, -2.0, -3.0),
            glam::vec3(1.0, 2.0, 3.0),
            glam::vec3(0.0, 0.0, 0.0),
        ];
        
        let (min, max) = sample.compute_bounds();
        assert_eq!(min, glam::vec3(-1.0, -2.0, -3.0));
        assert_eq!(max, glam::vec3(1.0, 2.0, 3.0));
    }
}
