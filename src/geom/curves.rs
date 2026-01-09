//! Curves schema implementation.
//!
//! Provides reading of curve data (NURBS, Bezier, linear) from Alembic files.

use crate::abc::IObject;
use crate::util::Result;

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
}

/// Curves sample data.
#[derive(Clone, Debug, Default)]
pub struct CurvesSample {
    /// Curve positions (all curves concatenated).
    pub positions: Vec<glam::Vec3>,
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

/// Input Curves schema reader.
pub struct ICurves<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ICurves<'a> {
    /// Wrap an IObject as ICurves.
    /// Returns None if the object doesn't have the Curves schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(CURVES_SCHEMA) {
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
    
    /// Check if curves are constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<CurvesSample> {
        use crate::util::Error;
        
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        
        let mut sample = CurvesSample::new();
        
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
        
        // Read nVertices (vertex count per curve) - required
        if let Some(nv_prop) = geom.property_by_name("nVertices") {
            if let Some(array) = nv_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.num_vertices = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
                }
            }
        }
        
        // Read curveBasisAndType (combined type/basis info)
        if let Some(cbt_prop) = geom.property_by_name("curveBasisAndType") {
            if let Some(scalar) = cbt_prop.as_scalar() {
                let mut buf = [0u8; 4];
                if scalar.read_sample(0, &mut buf).is_ok() {
                    sample.curve_type = CurveType::from_u8(buf[0]);
                    sample.wrap = CurvePeriodicity::from_u8(buf[1]);
                    sample.basis = BasisType::from_u8(buf[2]);
                }
            }
        }
        
        // Read widths if present
        if let Some(w_prop) = geom.property_by_name("width") {
            if let Some(array) = w_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.widths = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
                }
            }
        }
        
        // Read UVs if present
        if let Some(uv_prop) = geom.property_by_name("uv") {
            if let Some(array) = uv_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.uvs = floats.chunks_exact(2)
                        .map(|c| glam::vec2(c[0], c[1]))
                        .collect();
                }
            }
        }
        
        // Read normals if present
        if let Some(n_prop) = geom.property_by_name("N") {
            if let Some(array) = n_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.normals = floats.chunks_exact(3)
                        .map(|c| glam::vec3(c[0], c[1], c[2]))
                        .collect();
                }
            }
        }
        
        // Read knots if present (for NURBS)
        if let Some(k_prop) = geom.property_by_name("knots") {
            if let Some(array) = k_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.knots = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
                }
            }
        }
        
        // Read orders if present (for NURBS)
        if let Some(o_prop) = geom.property_by_name("orders") {
            if let Some(array) = o_prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    sample.orders = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
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
