//! Curves schema implementation.
//!
//! Provides reading of curve data (NURBS, Bezier, linear) from Alembic files.

use crate::abc::IObject;
use crate::core::TopologyVariance;
use crate::geom::util as geom_util;
use crate::util::{Result, BBox3d};

/// Curves schema identifier.
pub const CURVES_SCHEMA: &str = "AbcGeom_Curve_v2";

/// Curve type enumeration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurveType {
    /// Cubic curves (Bezier/NURBS)
    #[default]
    Cubic,
    /// Linear curves (polylines)
    Linear,
    /// Bezier curves with explicit order
    Bezier,
    /// B-spline curves
    Bspline,
    /// Catmull-Rom splines
    CatmullRom,
    /// Hermite curves
    Hermite,
}

impl CurveType {
    /// Parse from Alembic u8 value.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => CurveType::Cubic,
            1 => CurveType::Linear,
            2 => CurveType::Bezier,
            3 => CurveType::Bspline,
            4 => CurveType::CatmullRom,
            5 => CurveType::Hermite,
            _ => CurveType::Cubic,
        }
    }

    /// Convert to Alembic u8 value.
    pub fn to_u8(self) -> u8 {
        match self {
            CurveType::Cubic => 0,
            CurveType::Linear => 1,
            CurveType::Bezier => 2,
            CurveType::Bspline => 3,
            CurveType::CatmullRom => 4,
            CurveType::Hermite => 5,
        }
    }
}

impl std::fmt::Display for CurveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_u8())
    }
}

/// Curve periodicity (wrap mode).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurvePeriodicity {
    /// Non-periodic (open) curves
    #[default]
    NonPeriodic,
    /// Periodic (closed) curves
    Periodic,
}

impl CurvePeriodicity {
    /// Parse from Alembic u8 value.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => CurvePeriodicity::NonPeriodic,
            1 => CurvePeriodicity::Periodic,
            _ => CurvePeriodicity::NonPeriodic,
        }
    }

    /// Convert to Alembic u8 value.
    pub fn to_u8(self) -> u8 {
        match self {
            CurvePeriodicity::NonPeriodic => 0,
            CurvePeriodicity::Periodic => 1,
        }
    }
}

impl std::fmt::Display for CurvePeriodicity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_u8())
    }
}

/// Basis type for curves (interpolation method).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BasisType {
    /// No specific basis
    #[default]
    NoBasis,
    /// Bezier basis
    Bezier,
    /// B-spline basis
    Bspline,
    /// Catmull-Rom basis
    CatmullRom,
    /// Hermite basis
    Hermite,
    /// Power basis
    Power,
}

impl BasisType {
    /// Parse from Alembic u8 value.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => BasisType::NoBasis,
            1 => BasisType::Bezier,
            2 => BasisType::Bspline,
            3 => BasisType::CatmullRom,
            4 => BasisType::Hermite,
            5 => BasisType::Power,
            _ => BasisType::NoBasis,
        }
    }

    /// Convert to Alembic u8 value.
    pub fn to_u8(self) -> u8 {
        match self {
            BasisType::NoBasis => 0,
            BasisType::Bezier => 1,
            BasisType::Bspline => 2,
            BasisType::CatmullRom => 3,
            BasisType::Hermite => 4,
            BasisType::Power => 5,
        }
    }
}

impl std::fmt::Display for BasisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_u8())
    }
}

/// Curves sample data.
#[derive(Clone, Debug, Default)]
pub struct CurvesSample {
    /// Curve positions (all curves concatenated).
    pub positions: Vec<glam::Vec3>,
    /// Vertex velocities (optional, for motion blur).
    pub velocities: Option<Vec<glam::Vec3>>,
    /// Number of vertices per curve.
    pub num_vertices: Vec<i32>,
    /// Curve type.
    pub curve_type: CurveType,
    /// Periodicity (wrap mode).
    pub wrap: CurvePeriodicity,
    /// Basis type.
    pub basis: BasisType,
    /// Optional widths per vertex.
    pub widths: Vec<f32>,
    /// Optional UVs per vertex.
    pub uvs: Vec<glam::Vec2>,
    /// Optional normals per vertex.
    pub normals: Vec<glam::Vec3>,
    /// Optional knots for NURBS curves.
    pub knots: Vec<f32>,
    /// Optional orders for NURBS curves.
    pub orders: Vec<i32>,
    /// Self bounds (axis-aligned bounding box).
    pub self_bounds: Option<BBox3d>,
}

impl CurvesSample {
    /// Create empty sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get the number of curves.
    pub fn num_curves(&self) -> usize {
        self.num_vertices.len()
    }
    
    /// Get total vertex count.
    pub fn total_vertices(&self) -> usize {
        self.positions.len()
    }
    
    /// Check if sample has width data.
    pub fn has_widths(&self) -> bool {
        !self.widths.is_empty()
    }
    
    /// Check if sample has UV data.
    pub fn has_uvs(&self) -> bool {
        !self.uvs.is_empty()
    }
    
    /// Check if sample has normal data.
    pub fn has_normals(&self) -> bool {
        !self.normals.is_empty()
    }
    
    /// Check if sample has velocities.
    pub fn has_velocities(&self) -> bool {
        self.velocities.is_some()
    }
    
    /// Check if sample has self bounds.
    pub fn has_self_bounds(&self) -> bool {
        self.self_bounds.is_some()
    }
    
    /// Check if sample is valid (has data).
    pub fn is_valid(&self) -> bool {
        !self.positions.is_empty() && !self.num_vertices.is_empty()
    }
    
    /// Get positions for a specific curve by index.
    pub fn curve_positions(&self, curve_idx: usize) -> Option<&[glam::Vec3]> {
        if curve_idx >= self.num_vertices.len() {
            return None;
        }
        
        let start: usize = self.num_vertices[..curve_idx]
            .iter()
            .map(|&n| n as usize)
            .sum();
        let count = self.num_vertices[curve_idx] as usize;
        
        if start + count <= self.positions.len() {
            Some(&self.positions[start..start + count])
        } else {
            None
        }
    }
    
    /// Compute bounding box of all curves.
    pub fn compute_bounds(&self) -> (glam::Vec3, glam::Vec3) {
        geom_util::compute_bounds_vec3(&self.positions)
    }
}

/// Input Curves schema reader.
pub struct ICurves<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ICurves<'a> {
    /// Wrap an IObject as ICurves.
    /// Returns None if the object doesn't have the Curves schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matchesSchema(CURVES_SCHEMA) {
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
    
    /// Get property names from .geom compound.
    pub fn getPropertyNames(&self) -> Vec<String> {
        geom_util::geom_property_names(self.object)
    }
    
    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        geom_util::num_samples_from_positions(self.object)
    }
    
    /// Check if curves are constant (single sample).
    pub fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Get time sampling index from positions property.
    pub fn getTimeSamplingIndex(&self) -> u32 {
        geom_util::positions_time_sampling_index(self.object)
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        geom_util::child_bounds_time_sampling_index(self.object)
    }
    
    /// Get the topology variance for these curves.
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
        
        let nv_samples = if let Some(nv) = geom.getPropertyByName("nVertices") {
            nv.asArray().map(|a| a.getNumSamples()).unwrap_or(1)
        } else { 1 };
        
        // Determine variance
        let max_samples = p_samples.max(nv_samples);
        
        if max_samples <= 1 {
            TopologyVariance::Static
        } else if nv_samples <= 1 {
            TopologyVariance::Homogeneous
        } else {
            TopologyVariance::Heterogeneous
        }
    }
    
    /// Check if curves have arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        geom_util::has_arb_geom_params(self.object)
    }
    
    /// Get names of arbitrary geometry parameters.
    pub fn arb_geom_param_names(&self) -> Vec<String> {
        geom_util::arb_geom_param_names(self.object)
    }
    
    /// Check if curves have user properties.
    pub fn has_user_properties(&self) -> bool {
        geom_util::has_user_properties(self.object)
    }
    
    /// Get names of user properties.
    pub fn user_property_names(&self) -> Vec<String> {
        geom_util::user_property_names(self.object)
    }
    
    /// Read a sample at the given index.
    pub fn getSample(&self, index: usize) -> Result<CurvesSample> {
        use crate::util::Error;
        
        let props = self.object.getProperties();
        let geom_prop = props.getPropertyByName(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.asCompound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        let g = geom.as_reader();
        
        let mut sample = CurvesSample::new();
        
        // Read core geometry using helpers
        if let Some(pos) = geom_util::read_vec3_array(g, "P", index) {
            sample.positions = pos;
        }
        sample.velocities = geom_util::read_vec3_array(g, ".velocities", index);
        if let Some(nv) = geom_util::read_i32_array(g, "nVertices", index) {
            sample.num_vertices = nv;
        }
        
        // Read curveBasisAndType (combined type/basis info) - special handling
        if let Some(cbt_prop) = geom.getPropertyByName("curveBasisAndType") {
            if let Some(scalar) = cbt_prop.asScalar() {
                let mut buf = [0u8; 4];
                if scalar.getSample(0, &mut buf).is_ok() {
                    sample.curve_type = CurveType::from_u8(buf[0]);
                    sample.wrap = CurvePeriodicity::from_u8(buf[1]);
                    sample.basis = BasisType::from_u8(buf[2]);
                }
            }
        }
        
        // Read optional attributes
        if let Some(w) = geom_util::read_f32_array(g, "width", index) {
            sample.widths = w;
        }
        if let Some(uvs) = geom_util::read_vec2_array(g, "uv", index) {
            sample.uvs = uvs;
        }
        if let Some(n) = geom_util::read_vec3_array(g, "N", index) {
            sample.normals = n;
        }
        
        // NURBS data
        if let Some(k) = geom_util::read_f32_array(g, "knots", index) {
            sample.knots = k;
        }
        if let Some(o) = geom_util::read_i32_array(g, "orders", index) {
            sample.orders = o;
        }
        sample.self_bounds = geom_util::read_self_bounds(g, index);
        
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_curves_sample_empty() {
        let sample = CurvesSample::new();
        assert_eq!(sample.num_curves(), 0);
        assert_eq!(sample.total_vertices(), 0);
        assert!(!sample.is_valid());
    }
    
    #[test]
    fn test_curves_sample_basic() {
        let mut sample = CurvesSample::new();
        sample.positions = vec![
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(1.0, 0.0, 0.0),
            glam::vec3(1.0, 1.0, 0.0),
            glam::vec3(0.0, 1.0, 0.0),
            glam::vec3(2.0, 0.0, 0.0),
            glam::vec3(3.0, 1.0, 0.0),
        ];
        sample.num_vertices = vec![4, 2]; // First curve has 4 verts, second has 2
        
        assert_eq!(sample.num_curves(), 2);
        assert_eq!(sample.total_vertices(), 6);
        assert!(sample.is_valid());
        
        // Get first curve
        let curve0 = sample.curve_positions(0).unwrap();
        assert_eq!(curve0.len(), 4);
        
        // Get second curve
        let curve1 = sample.curve_positions(1).unwrap();
        assert_eq!(curve1.len(), 2);
        
        // Check bounds
        let (min, max) = sample.compute_bounds();
        assert_eq!(min, glam::vec3(0.0, 0.0, 0.0));
        assert_eq!(max, glam::vec3(3.0, 1.0, 0.0));
    }
    
    #[test]
    fn test_curve_type_parsing() {
        assert_eq!(CurveType::from_u8(0), CurveType::Cubic);
        assert_eq!(CurveType::from_u8(1), CurveType::Linear);
        assert_eq!(CurveType::from_u8(99), CurveType::Cubic); // Unknown defaults to Cubic
    }
    
    #[test]
    fn test_curve_periodicity() {
        assert_eq!(CurvePeriodicity::from_u8(0), CurvePeriodicity::NonPeriodic);
        assert_eq!(CurvePeriodicity::from_u8(1), CurvePeriodicity::Periodic);
    }
}
