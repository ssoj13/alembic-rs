//! Points schema implementation.
//!
//! Provides reading of point cloud / particle data from Alembic files.

use crate::abc::IObject;
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
    
    /// Check if points are constant.
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Check if points have arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return false;
        };
        let Some(geom) = geom_prop.as_compound() else {
            return false;
        };
        geom.has_property(".arbGeomParams")
    }
    
    /// Get names of arbitrary geometry parameters.
    pub fn arb_geom_param_names(&self) -> Vec<String> {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return Vec::new();
        };
        let Some(geom) = geom_prop.as_compound() else {
            return Vec::new();
        };
        let Some(arb_prop) = geom.property_by_name(".arbGeomParams") else {
            return Vec::new();
        };
        let Some(arb) = arb_prop.as_compound() else {
            return Vec::new();
        };
        arb.property_names()
    }
    
    /// Check if points have user properties.
    pub fn has_user_properties(&self) -> bool {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return false;
        };
        let Some(geom) = geom_prop.as_compound() else {
            return false;
        };
        geom.has_property(".userProperties")
    }
    
    /// Get names of user properties.
    pub fn user_property_names(&self) -> Vec<String> {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else {
            return Vec::new();
        };
        let Some(geom) = geom_prop.as_compound() else {
            return Vec::new();
        };
        let Some(user_prop) = geom.property_by_name(".userProperties") else {
            return Vec::new();
        };
        let Some(user) = user_prop.as_compound() else {
            return Vec::new();
        };
        user.property_names()
    }
    
    /// Check if points have self bounds property.
    pub fn has_self_bounds(&self) -> bool {
        let props = self.object.properties();
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                return geom.has_property(".selfBnds");
            }
        }
        false
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
        
        let mut sample = PointsSample::new();
        
        // Read P (positions) - required
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
        
        // Read id (particle IDs)
        if let Some(id_prop) = geom.property_by_name("id") {
            if let Some(array) = id_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.ids = bytemuck::cast_slice::<u8, u64>(&data).to_vec();
                }
            }
        }
        
        // Read velocity (try both "velocity" and ".velocities")
        let v_prop = geom.property_by_name("velocity")
            .or_else(|| geom.property_by_name(".velocities"));
        if let Some(v_prop) = v_prop {
            if let Some(array) = v_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.velocities = floats.chunks_exact(3)
                        .map(|c| glam::vec3(c[0], c[1], c[2]))
                        .collect();
                }
            }
        }
        
        // Read width
        if let Some(w_prop) = geom.property_by_name("width") {
            if let Some(array) = w_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.widths = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
                }
            }
        }
        
        // Read .selfBnds if present
        if let Some(bnds_prop) = geom.property_by_name(".selfBnds") {
            if let Some(scalar) = bnds_prop.as_scalar() {
                let mut buf = [0u8; 48]; // 6 x f64
                if scalar.read_sample(index, &mut buf).is_ok() {
                    let doubles: &[f64] = bytemuck::cast_slice(&buf);
                    if doubles.len() >= 6 {
                        sample.self_bounds = Some(BBox3d::new(
                            glam::dvec3(doubles[0], doubles[1], doubles[2]),
                            glam::dvec3(doubles[3], doubles[4], doubles[5]),
                        ));
                    }
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
