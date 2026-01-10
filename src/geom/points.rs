//! Points schema implementation.
//!
//! Provides reading of point cloud / particle data from Alembic files.

use crate::abc::IObject;
use crate::geom::util as geom_util;
use crate::util::{Result, BBox3d};
use crate::core::TopologyVariance;

/// Points schema identifier.
pub const POINTS_SCHEMA: &str = "AbcGeom_Points_v1";

/// Points sample data.
#[derive(Clone, Debug, Default)]
pub struct PointsSample {
    /// Point positions.
    pub positions: Vec<glam::Vec3>,
    /// Point IDs (unique identifiers).
    pub ids: Vec<u64>,
    /// Optional velocities.
    pub velocities: Vec<glam::Vec3>,
    /// Optional widths (radius per point).
    pub widths: Vec<f32>,
    /// Self bounds - bounding box of this geometry (optional).
    pub self_bounds: Option<BBox3d>,
}

impl PointsSample {
    /// Create empty sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get number of points.
    pub fn num_points(&self) -> usize {
        self.positions.len()
    }
    
    /// Check if sample has velocity data.
    pub fn has_velocities(&self) -> bool {
        !self.velocities.is_empty()
    }
    
    /// Check if sample has width data.
    pub fn has_widths(&self) -> bool {
        !self.widths.is_empty()
    }
    
    /// Check if sample has ID data.
    pub fn has_ids(&self) -> bool {
        !self.ids.is_empty()
    }
    
    /// Check if sample has self bounds.
    pub fn has_self_bounds(&self) -> bool {
        self.self_bounds.is_some()
    }
    
    /// Check if sample is valid.
    pub fn is_valid(&self) -> bool {
        !self.positions.is_empty()
    }
    
    /// Compute bounding box.
    pub fn compute_bounds(&self) -> (glam::Vec3, glam::Vec3) {
        geom_util::compute_bounds_vec3(&self.positions)
    }
}

/// Input Points schema reader.
pub struct IPoints<'a> {
    object: &'a IObject<'a>,
}

impl<'a> IPoints<'a> {
    /// Wrap an IObject as IPoints.
    /// Returns None if the object doesn't have the Points schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(POINTS_SCHEMA) {
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
    
    /// Get property names from .geom compound.
    pub fn property_names(&self) -> Vec<String> {
        geom_util::geom_property_names(self.object)
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        geom_util::num_samples_from_positions(self.object)
    }
    
    /// Check if points are constant.
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Check if points have arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        geom_util::has_arb_geom_params(self.object)
    }
    
    /// Get names of arbitrary geometry parameters.
    pub fn arb_geom_param_names(&self) -> Vec<String> {
        geom_util::arb_geom_param_names(self.object)
    }
    
    /// Check if points have user properties.
    pub fn has_user_properties(&self) -> bool {
        geom_util::has_user_properties(self.object)
    }
    
    /// Get names of user properties.
    pub fn user_property_names(&self) -> Vec<String> {
        geom_util::user_property_names(self.object)
    }
    
    /// Check if points have self bounds property.
    pub fn has_self_bounds(&self) -> bool {
        geom_util::has_self_bounds(self.object)
    }
    
    /// Get topology variance.
    /// 
    /// Points are typically heterogeneous since particle count can change.
    pub fn topology_variance(&self) -> TopologyVariance {
        if self.num_samples() <= 1 {
            TopologyVariance::Static
        } else {
            // Points typically have changing topology (particle birth/death)
            TopologyVariance::Heterogeneous
        }
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<PointsSample> {
        use crate::util::Error;
        
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        let g = geom.as_reader();
        
        let mut sample = PointsSample::new();
        
        // Read core geometry using helpers
        if let Some(pos) = geom_util::read_vec3_array(g, "P", index) {
            sample.positions = pos;
        }
        if let Some(ids) = geom_util::read_u64_array(g, "id", index) {
            sample.ids = ids;
        }
        
        // Read velocity (try both "velocity" and ".velocities")
        sample.velocities = geom_util::read_vec3_array(g, "velocity", index)
            .or_else(|| geom_util::read_vec3_array(g, ".velocities", index))
            .unwrap_or_default();
        
        // Read width and bounds
        if let Some(w) = geom_util::read_f32_array(g, "width", index) {
            sample.widths = w;
        }
        sample.self_bounds = geom_util::read_self_bounds(g, index);
        
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_points_sample_empty() {
        let sample = PointsSample::new();
        assert_eq!(sample.num_points(), 0);
        assert!(!sample.is_valid());
    }
    
    #[test]
    fn test_points_sample_basic() {
        let mut sample = PointsSample::new();
        sample.positions = vec![
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(1.0, 2.0, 3.0),
            glam::vec3(-1.0, -2.0, -3.0),
        ];
        sample.ids = vec![100, 101, 102];
        
        assert_eq!(sample.num_points(), 3);
        assert!(sample.is_valid());
        assert!(sample.has_ids());
        assert!(!sample.has_velocities());
        
        let (min, max) = sample.compute_bounds();
        assert_eq!(min, glam::vec3(-1.0, -2.0, -3.0));
        assert_eq!(max, glam::vec3(1.0, 2.0, 3.0));
    }
}
