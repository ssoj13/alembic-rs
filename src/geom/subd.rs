//! Subdivision surface schema implementation.
//!
//! Provides reading of subdivision surface data from Alembic files.

use crate::abc::IObject;
use crate::util::Result;

/// SubD schema identifier.
pub const SUBD_SCHEMA: &str = "AbcGeom_SubD_v1";

/// Subdivision scheme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubDScheme {
    /// Catmull-Clark subdivision.
    #[default]
    CatmullClark,
    /// Loop subdivision (for triangles).
    Loop,
    /// Bilinear subdivision.
    Bilinear,
}

impl SubDScheme {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s {
            "catmullClark" => SubDScheme::CatmullClark,
            "loop" => SubDScheme::Loop,
            "bilinear" => SubDScheme::Bilinear,
            _ => SubDScheme::CatmullClark,
        }
    }
}

/// SubD sample data.
#[derive(Clone, Debug, Default)]
pub struct SubDSample {
    /// Vertex positions.
    pub positions: Vec<glam::Vec3>,
    /// Face vertex counts.
    pub face_counts: Vec<i32>,
    /// Face vertex indices.
    pub face_indices: Vec<i32>,
    /// Subdivision scheme.
    pub scheme: SubDScheme,
    /// Crease vertex indices (pairs).
    pub crease_indices: Vec<i32>,
    /// Crease lengths.
    pub crease_lengths: Vec<i32>,
    /// Crease sharpness values.
    pub crease_sharpnesses: Vec<f32>,
    /// Corner vertex indices.
    pub corner_indices: Vec<i32>,
    /// Corner sharpness values.
    pub corner_sharpnesses: Vec<f32>,
    /// Hole face indices.
    pub holes: Vec<i32>,
    /// Face-varying interpolation boundary.
    pub fv_interp_boundary: i32,
    /// Face-varying propagate corners.
    pub fv_propagate_corners: i32,
    /// Interpolate boundary.
    pub interp_boundary: i32,
}

impl SubDSample {
    /// Create empty sample.
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
    
    /// Check if sample has crease data.
    pub fn has_creases(&self) -> bool {
        !self.crease_indices.is_empty()
    }
    
    /// Check if sample has corner data.
    pub fn has_corners(&self) -> bool {
        !self.corner_indices.is_empty()
    }
    
    /// Check if sample has holes.
    pub fn has_holes(&self) -> bool {
        !self.holes.is_empty()
    }
    
    /// Check if sample is valid.
    pub fn is_valid(&self) -> bool {
        !self.positions.is_empty() && !self.face_counts.is_empty()
    }
    
    /// Compute bounding box.
    pub fn compute_bounds(&self) -> (glam::Vec3, glam::Vec3) {
        if self.positions.is_empty() {
            return (glam::Vec3::ZERO, glam::Vec3::ZERO);
        }
        
        let mut min = glam::Vec3::splat(f32::MAX);
        let mut max = glam::Vec3::splat(f32::MIN);
        
        for p in &self.positions {
            min = min.min(*p);
            max = max.max(*p);
        }
        
        (min, max)
    }
}

/// Input SubD schema reader.
pub struct ISubD<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ISubD<'a> {
    /// Wrap an IObject as ISubD.
    /// Returns None if the object doesn't have the SubD schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(SUBD_SCHEMA) {
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
    
    /// Get property names.
    pub fn property_names(&self) -> Vec<String> {
        let props = self.object.properties();
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                return geom.property_names();
            }
        }
        Vec::new()
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 1 };
        let Some(geom) = geom_prop.as_compound() else { return 1 };
        let Some(p_prop) = geom.property_by_name("P") else { return 1 };
        let Some(array) = p_prop.as_array() else { return 1 };
        array.num_samples()
    }
    
    /// Check if SubD is constant.
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<SubDSample> {
        use crate::util::Error;
        
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        
        let mut sample = SubDSample::new();
        
        // Read P (positions)
        if let Some(p_prop) = geom.property_by_name("P") {
            if let Some(array) = p_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.positions = floats.chunks_exact(3)
                        .map(|c| glam::vec3(c[0], c[1], c[2]))
                        .collect();
                }
            }
        }
        
        // Read .faceCounts
        if let Some(fc_prop) = geom.property_by_name(".faceCounts") {
            if let Some(array) = fc_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.face_counts = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        // Read .faceIndices
        if let Some(fi_prop) = geom.property_by_name(".faceIndices") {
            if let Some(array) = fi_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.face_indices = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        // Read .creaseIndices
        if let Some(ci_prop) = geom.property_by_name(".creaseIndices") {
            if let Some(array) = ci_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.crease_indices = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        // Read .creaseLengths
        if let Some(cl_prop) = geom.property_by_name(".creaseLengths") {
            if let Some(array) = cl_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.crease_lengths = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        // Read .creaseSharpnesses
        if let Some(cs_prop) = geom.property_by_name(".creaseSharpnesses") {
            if let Some(array) = cs_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.crease_sharpnesses = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
                }
            }
        }
        
        // Read .cornerIndices
        if let Some(cri_prop) = geom.property_by_name(".cornerIndices") {
            if let Some(array) = cri_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.corner_indices = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        // Read .cornerSharpnesses
        if let Some(crs_prop) = geom.property_by_name(".cornerSharpnesses") {
            if let Some(array) = crs_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.corner_sharpnesses = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
                }
            }
        }
        
        // Read .holes
        if let Some(h_prop) = geom.property_by_name(".holes") {
            if let Some(array) = h_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.holes = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_subd_sample_empty() {
        let sample = SubDSample::new();
        assert_eq!(sample.num_vertices(), 0);
        assert!(!sample.is_valid());
    }
    
    #[test]
    fn test_subd_scheme() {
        assert_eq!(SubDScheme::from_str("catmullClark"), SubDScheme::CatmullClark);
        assert_eq!(SubDScheme::from_str("loop"), SubDScheme::Loop);
        assert_eq!(SubDScheme::from_str("bilinear"), SubDScheme::Bilinear);
        assert_eq!(SubDScheme::from_str("unknown"), SubDScheme::CatmullClark);
    }
    
    #[test]
    fn test_subd_sample_quad() {
        let mut sample = SubDSample::new();
        sample.positions = vec![
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(1.0, 0.0, 0.0),
            glam::vec3(1.0, 1.0, 0.0),
            glam::vec3(0.0, 1.0, 0.0),
        ];
        sample.face_counts = vec![4];
        sample.face_indices = vec![0, 1, 2, 3];
        
        assert_eq!(sample.num_vertices(), 4);
        assert_eq!(sample.num_faces(), 1);
        assert!(sample.is_valid());
    }
}
