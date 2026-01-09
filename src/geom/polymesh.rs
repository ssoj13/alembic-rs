//! PolyMesh (polygon mesh) schema implementation.
//!
//! Provides reading of polygon mesh data from Alembic files.

use crate::abc::IObject;
use crate::util::Result;

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
    /// Bounding box (optional).
    pub bounds: Option<glam::Vec3A>,
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
        if self.positions.is_empty() {
            return (glam::Vec3::ZERO, glam::Vec3::ZERO);
        }
        
        let mut min = self.positions[0];
        let mut max = self.positions[0];
        
        for &p in &self.positions[1..] {
            min = min.min(p);
            max = max.max(p);
        }
        
        (min, max)
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
        // Read from P property through the chain
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 1 };
        let Some(geom) = geom_prop.as_compound() else { return 1 };
        let Some(p_prop) = geom.property_by_name("P") else { return 1 };
        let Some(array_reader) = p_prop.as_array() else { return 1 };
        array_reader.num_samples()
    }
    
    /// Check if this mesh is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Get property names available on this mesh.
    pub fn property_names(&self) -> Vec<String> {
        self.object.properties().property_names()
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
        
        // Read P (positions)
        if let Some(p_prop) = geom.property_by_name("P") {
            if let Some(array_reader) = p_prop.as_array() {
                let data = array_reader.read_sample_vec(index)?;
                let floats: &[f32] = bytemuck::cast_slice(&data);
                sample.positions = floats.chunks_exact(3)
                    .map(|c| glam::vec3(c[0], c[1], c[2]))
                    .collect();
            }
        }
        
        // Read .faceCounts
        if let Some(fc_prop) = geom.property_by_name(".faceCounts") {
            if let Some(array_reader) = fc_prop.as_array() {
                let data = array_reader.read_sample_vec(index)?;
                sample.face_counts = bytemuck::cast_slice(&data).to_vec();
            }
        }
        
        // Read .faceIndices
        if let Some(fi_prop) = geom.property_by_name(".faceIndices") {
            if let Some(array_reader) = fi_prop.as_array() {
                let data = array_reader.read_sample_vec(index)?;
                sample.face_indices = bytemuck::cast_slice(&data).to_vec();
            }
        }
        
        // Read N (normals) if present
        if let Some(n_prop) = geom.property_by_name("N") {
            if let Some(array_reader) = n_prop.as_array() {
                if let Ok(data) = array_reader.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.normals = Some(
                        floats.chunks_exact(3)
                            .map(|c| glam::vec3(c[0], c[1], c[2]))
                            .collect()
                    );
                }
            }
        }
        
        // Read uv if present
        if let Some(uv_prop) = geom.property_by_name("uv") {
            if let Some(array_reader) = uv_prop.as_array() {
                if let Ok(data) = array_reader.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.uvs = Some(
                        floats.chunks_exact(2)
                            .map(|c| glam::vec2(c[0], c[1]))
                            .collect()
                    );
                }
            }
        }
        
        Ok(sample)
    }
}

/// Topology type for mesh changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshTopologyVariance {
    /// Topology is constant (vertex count and connectivity don't change).
    Constant,
    /// Topology can change between samples.
    Varying,
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
