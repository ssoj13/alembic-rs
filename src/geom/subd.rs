//! Subdivision surface schema implementation.
//!
//! Provides reading of subdivision surface data from Alembic files.

use crate::abc::IObject;
use crate::core::TopologyVariance;
use crate::geom::faceset::{IFaceSet, FACESET_SCHEMA};
use crate::geom::util as geom_util;
use crate::util::{Result, BBox3d};

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
    /// Parse subdivision scheme from string.
    /// Accepts both C++ canonical form ("catmull-clark") and legacy ("catmullClark").
    pub fn parse(s: &str) -> Self {
        match s {
            "catmull-clark" | "catmullClark" => SubDScheme::CatmullClark,
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
    /// Vertex velocities (optional, for motion blur).
    pub velocities: Option<Vec<glam::Vec3>>,
    /// UV coordinates (optional).
    pub uvs: Option<Vec<glam::Vec2>>,
    /// UV indices for indexed UVs (optional).
    pub uv_indices: Option<Vec<i32>>,
    /// Normals (optional).
    pub normals: Option<Vec<glam::Vec3>>,
    /// Normal indices for indexed normals (optional).
    pub normal_indices: Option<Vec<i32>>,
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
    /// Self bounds - bounding box of this geometry (optional).
    pub self_bounds: Option<BBox3d>,
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
    
    /// Check if sample has velocities.
    pub fn has_velocities(&self) -> bool {
        self.velocities.is_some()
    }
    
    /// Check if sample has self bounds.
    pub fn has_self_bounds(&self) -> bool {
        self.self_bounds.is_some()
    }
    
    /// Check if sample is valid.
    pub fn is_valid(&self) -> bool {
        !self.positions.is_empty() && !self.face_counts.is_empty()
    }
    
    /// Compute bounding box.
    pub fn compute_bounds(&self) -> (glam::Vec3, glam::Vec3) {
        geom_util::compute_bounds_vec3(&self.positions)
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
        if object.matchesSchema(SUBD_SCHEMA) {
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
    pub fn getName(&self) -> &str {
        self.object.getName()
    }
    
    /// Get the full path.
    pub fn getFullName(&self) -> &str {
        self.object.getFullName()
    }
    
    /// Get property names.
    pub fn getPropertyNames(&self) -> Vec<String> {
        geom_util::geom_property_names(self.object)
    }
    
    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        geom_util::num_samples_from_positions(self.object)
    }
    
    /// Check if SubD is constant.
    pub fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Get time sampling index from positions property.
    pub fn getTimeSamplingIndex(&self) -> u32 {
        geom_util::positions_time_sampling_index(self.object)
    }
    
    /// Get the topology variance for this subdivision surface.
    /// 
    /// Returns:
    /// - Static: Only one sample exists
    /// - Homogeneous: Topology is constant, only positions change
    /// - Heterogeneous: Topology can change between samples
    pub fn topology_variance(&self) -> TopologyVariance {
        let props = self.object.getProperties();
        let Some(geom_prop) = props.getPropertyByName(".geom") else {
            return TopologyVariance::Static;
        };
        let Some(geom) = geom_prop.asCompound() else {
            return TopologyVariance::Static;
        };
        
        // Get sample counts for positions and topology
        let p_samples = if let Some(p) = geom.getPropertyByName("P") {
            p.asArray().map(|a| a.getNumSamples()).unwrap_or(1)
        } else { 1 };
        
        let fc_samples = if let Some(fc) = geom.getPropertyByName(".faceCounts") {
            fc.asArray().map(|a| a.getNumSamples()).unwrap_or(1)
        } else { 1 };
        
        let fi_samples = if let Some(fi) = geom.getPropertyByName(".faceIndices") {
            fi.asArray().map(|a| a.getNumSamples()).unwrap_or(1)
        } else { 1 };
        
        // Determine variance
        let max_samples = p_samples.max(fc_samples).max(fi_samples);
        
        if max_samples <= 1 {
            TopologyVariance::Static
        } else if fc_samples <= 1 && fi_samples <= 1 {
            TopologyVariance::Homogeneous
        } else {
            TopologyVariance::Heterogeneous
        }
    }
    
    /// Get the names of all FaceSets on this SubD.
    /// 
    /// FaceSets are child objects with the FaceSet schema.
    pub fn face_set_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for i in 0..self.object.getNumChildren() {
            if let Some(header) = self.object.getChildHeader(i) {
                if header.meta_data.get("schema").map(|s| s == FACESET_SCHEMA).unwrap_or(false) {
                    names.push(header.name.clone());
                }
            }
        }
        names
    }
    
    /// Check if this SubD has a FaceSet with the given name.
    pub fn has_face_set(&self, name: &str) -> bool {
        if let Some(child) = self.object.getChildByName(name) {
            child.matchesSchema(FACESET_SCHEMA)
        } else {
            false
        }
    }
    
    /// Get a FaceSet by name.
    /// 
    /// Returns None if the FaceSet doesn't exist or doesn't have the FaceSet schema.
    pub fn face_set(&self, name: &str) -> Option<IFaceSet<'_>> {
        let child = self.object.getChildByName(name)?;
        IFaceSet::from_owned(child)
    }
    
    /// Get number of FaceSets on this SubD.
    pub fn num_face_sets(&self) -> usize {
        self.face_set_names().len()
    }
    
    /// Check if this SubD has arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        geom_util::has_arb_geom_params(self.object)
    }
    
    /// Get names of arbitrary geometry parameters.
    pub fn arb_geom_param_names(&self) -> Vec<String> {
        geom_util::arb_geom_param_names(self.object)
    }
    
    /// Check if this SubD has user properties.
    pub fn has_user_properties(&self) -> bool {
        geom_util::has_user_properties(self.object)
    }
    
    /// Get names of user properties.
    pub fn user_property_names(&self) -> Vec<String> {
        geom_util::user_property_names(self.object)
    }
    
    /// Check if this SubD has child bounds property.
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
    pub fn getSample(&self, index: usize) -> Result<SubDSample> {
        use crate::util::Error;
        
        let props = self.object.getProperties();
        let geom_prop = props.getPropertyByName(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.asCompound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        let g = geom.as_reader();
        
        let mut sample = SubDSample::new();
        
        // Read core geometry using helpers
        if let Some(pos) = geom_util::read_vec3_array(g, "P", index) {
            sample.positions = pos;
        }
        sample.velocities = geom_util::read_vec3_array(g, ".velocities", index);
        
        // Read indexed UVs
        if let Some((uvs, uv_idx)) = geom_util::read_indexed_vec2(g, "uv", index) {
            sample.uvs = Some(uvs);
            sample.uv_indices = uv_idx;
        }
        
        // Read indexed normals
        if let Some((normals, normal_idx)) = geom_util::read_indexed_vec3(g, "N", index) {
            sample.normals = Some(normals);
            sample.normal_indices = normal_idx;
        }
        
        // Read face data
        if let Some(fc) = geom_util::read_i32_array(g, ".faceCounts", index) {
            sample.face_counts = fc;
        }
        if let Some(fi) = geom_util::read_i32_array(g, ".faceIndices", index) {
            sample.face_indices = fi;
        }
        
        // Read crease data
        if let Some(ci) = geom_util::read_i32_array(g, ".creaseIndices", index) {
            sample.crease_indices = ci;
        }
        if let Some(cl) = geom_util::read_i32_array(g, ".creaseLengths", index) {
            sample.crease_lengths = cl;
        }
        if let Some(cs) = geom_util::read_f32_array(g, ".creaseSharpnesses", index) {
            sample.crease_sharpnesses = cs;
        }
        
        // Read corner data
        if let Some(cri) = geom_util::read_i32_array(g, ".cornerIndices", index) {
            sample.corner_indices = cri;
        }
        if let Some(crs) = geom_util::read_f32_array(g, ".cornerSharpnesses", index) {
            sample.corner_sharpnesses = crs;
        }
        
        // Read holes and bounds
        if let Some(h) = geom_util::read_i32_array(g, ".holes", index) {
            sample.holes = h;
        }
        sample.self_bounds = geom_util::read_self_bounds(g, index);
        
        Ok(sample)
    }
    
    /// Check if this subd has UVs.
    pub fn has_uvs(&self) -> bool {
        let props = self.object.getProperties();
        let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
        let Some(geom) = geom_prop.asCompound() else { return false };
        geom.hasProperty("uv")
    }
    
    /// Check if this subd has normals.
    pub fn has_normals(&self) -> bool {
        let props = self.object.getProperties();
        let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
        let Some(geom) = geom_prop.asCompound() else { return false };
        geom.hasProperty("N")
    }
    
    /// Get expanded UVs at the given sample index.
    /// 
    /// If UVs are indexed, this expands them to per-face-vertex values.
    pub fn get_uvs(&self, index: usize) -> Option<Vec<glam::Vec2>> {
        let props = self.object.getProperties();
        let geom_prop = props.getPropertyByName(".geom")?;
        let geom = geom_prop.asCompound()?;
        let uv_prop = geom.getPropertyByName("uv")?;
        
        if let Some(compound) = uv_prop.asCompound() {
            // Indexed UVs
            let vals_prop = compound.getPropertyByName(".vals")?;
            let array = vals_prop.asArray()?;
            let data = array.getSampleVec(index).ok()?;
            let floats: &[f32] = bytemuck::try_cast_slice(&data).ok()?;
            
            if let Some(idx_prop) = compound.getPropertyByName(".indices") {
                if let Some(idx_array) = idx_prop.asArray() {
                    if let Ok(idx_data) = idx_array.getSampleVec(index) {
                        let indices: &[u32] = bytemuck::try_cast_slice(&idx_data).ok()?;
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
            
            Some(floats.chunks_exact(2)
                .map(|c| glam::vec2(c[0], c[1]))
                .collect())
        } else if let Some(array) = uv_prop.asArray() {
            let data = array.getSampleVec(index).ok()?;
            let floats: &[f32] = bytemuck::try_cast_slice(&data).ok()?;
            Some(floats.chunks_exact(2)
                .map(|c| glam::vec2(c[0], c[1]))
                .collect())
        } else {
            None
        }
    }
    
    /// Get expanded normals at the given sample index.
    pub fn get_normals(&self, index: usize) -> Option<Vec<glam::Vec3>> {
        let props = self.object.getProperties();
        let geom_prop = props.getPropertyByName(".geom")?;
        let geom = geom_prop.asCompound()?;
        let n_prop = geom.getPropertyByName("N")?;
        
        if let Some(compound) = n_prop.asCompound() {
            let vals_prop = compound.getPropertyByName(".vals")?;
            let array = vals_prop.asArray()?;
            let data = array.getSampleVec(index).ok()?;
            let floats: &[f32] = bytemuck::try_cast_slice(&data).ok()?;
            
            if let Some(idx_prop) = compound.getPropertyByName(".indices") {
                if let Some(idx_array) = idx_prop.asArray() {
                    if let Ok(idx_data) = idx_array.getSampleVec(index) {
                        let indices: &[u32] = bytemuck::try_cast_slice(&idx_data).ok()?;
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
            
            Some(floats.chunks_exact(3)
                .map(|c| glam::vec3(c[0], c[1], c[2]))
                .collect())
        } else if let Some(array) = n_prop.asArray() {
            let data = array.getSampleVec(index).ok()?;
            let floats: &[f32] = bytemuck::try_cast_slice(&data).ok()?;
            Some(floats.chunks_exact(3)
                .map(|c| glam::vec3(c[0], c[1], c[2]))
                .collect())
        } else {
            None
        }
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
        assert_eq!(SubDScheme::parse("catmull-clark"), SubDScheme::CatmullClark);
        assert_eq!(SubDScheme::parse("catmullClark"), SubDScheme::CatmullClark);
        assert_eq!(SubDScheme::parse("loop"), SubDScheme::Loop);
        assert_eq!(SubDScheme::parse("bilinear"), SubDScheme::Bilinear);
        assert_eq!(SubDScheme::parse("unknown"), SubDScheme::CatmullClark);
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
