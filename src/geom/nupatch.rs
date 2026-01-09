//! NURBS Patch schema implementation.
//!
//! Provides reading of NURBS surface data from Alembic files.

use crate::abc::IObject;
use crate::util::{Result, Error, BBox3d};

/// NuPatch schema identifier.
pub const NUPATCH_SCHEMA: &str = "AbcGeom_NuPatch_v2";

/// NURBS patch sample data.
#[derive(Clone, Debug, Default)]
pub struct NuPatchSample {
    /// Control vertex positions.
    pub positions: Vec<glam::Vec3>,
    /// Velocities (optional).
    pub velocities: Option<Vec<glam::Vec3>>,
    /// Number of CVs in U direction.
    pub num_u: i32,
    /// Number of CVs in V direction.
    pub num_v: i32,
    /// Order in U direction.
    pub u_order: i32,
    /// Order in V direction.
    pub v_order: i32,
    /// U knot vector.
    pub u_knots: Vec<f32>,
    /// V knot vector.
    pub v_knots: Vec<f32>,
    /// Position weights (optional, 1.0 if not present).
    pub position_weights: Option<Vec<f32>>,
    /// Normals (optional).
    pub normals: Option<Vec<glam::Vec3>>,
    /// UVs (optional).
    pub uvs: Option<Vec<glam::Vec2>>,
    /// Self bounds (optional).
    pub self_bounds: Option<BBox3d>,
    /// Trim curve data (optional).
    pub trim_curve: Option<TrimCurveData>,
}

/// Trim curve data for NURBS patches.
#[derive(Clone, Debug, Default)]
pub struct TrimCurveData {
    /// Number of loops.
    pub num_loops: i32,
    /// Number of curves per loop.
    pub num_curves: Vec<i32>,
    /// Number of vertices per curve.
    pub num_vertices: Vec<i32>,
    /// Orders of trim curves.
    pub orders: Vec<i32>,
    /// Knot vectors.
    pub knots: Vec<f32>,
    /// Min parameter values.
    pub mins: Vec<f32>,
    /// Max parameter values.
    pub maxes: Vec<f32>,
    /// U coordinates.
    pub u: Vec<f32>,
    /// V coordinates.
    pub v: Vec<f32>,
    /// W (weight) coordinates.
    pub w: Vec<f32>,
}

impl NuPatchSample {
    /// Create an empty sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if sample has valid data.
    pub fn is_valid(&self) -> bool {
        !self.positions.is_empty() && 
        self.num_u > 0 && 
        self.num_v > 0 &&
        self.u_order > 0 && 
        self.v_order > 0 &&
        !self.u_knots.is_empty() && 
        !self.v_knots.is_empty()
    }
    
    /// Get total number of control vertices.
    pub fn num_cvs(&self) -> usize {
        self.positions.len()
    }
    
    /// Get expected number of CVs from dimensions.
    pub fn expected_cvs(&self) -> usize {
        (self.num_u * self.num_v) as usize
    }
    
    /// Check if this is a rational NURBS (has position weights).
    pub fn is_rational(&self) -> bool {
        self.position_weights.is_some()
    }
    
    /// Check if this patch has trim curves.
    pub fn has_trim_curve(&self) -> bool {
        self.trim_curve.as_ref().map(|t| t.num_loops > 0).unwrap_or(false)
    }
    
    /// Check if this patch has velocities.
    pub fn has_velocities(&self) -> bool {
        self.velocities.is_some()
    }
    
    /// Check if this patch has normals.
    pub fn has_normals(&self) -> bool {
        self.normals.is_some()
    }
    
    /// Check if this patch has UVs.
    pub fn has_uvs(&self) -> bool {
        self.uvs.is_some()
    }
    
    /// Get degree in U direction (order - 1).
    pub fn u_degree(&self) -> i32 {
        self.u_order.saturating_sub(1)
    }
    
    /// Get degree in V direction (order - 1).
    pub fn v_degree(&self) -> i32 {
        self.v_order.saturating_sub(1)
    }
    
    /// Calculate bounding box from positions.
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

/// Input NURBS Patch schema reader.
pub struct INuPatch<'a> {
    object: &'a IObject<'a>,
}

impl<'a> INuPatch<'a> {
    /// Wrap an IObject as an INuPatch.
    /// Returns None if the object doesn't have the NuPatch schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(NUPATCH_SCHEMA) {
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
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 1 };
        let Some(geom) = geom_prop.as_compound() else { return 1 };
        let Some(p_prop) = geom.property_by_name("P") else { return 1 };
        let Some(array_reader) = p_prop.as_array() else { return 1 };
        array_reader.num_samples()
    }
    
    /// Check if this patch is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<NuPatchSample> {
        let mut sample = NuPatchSample::new();
        
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
        
        // Read nu (numU)
        if let Some(nu_prop) = geom.property_by_name("nu") {
            if let Some(scalar) = nu_prop.as_scalar() {
                let mut buf = [0u8; 4];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    sample.num_u = i32::from_le_bytes(buf);
                }
            }
        }
        
        // Read nv (numV)
        if let Some(nv_prop) = geom.property_by_name("nv") {
            if let Some(scalar) = nv_prop.as_scalar() {
                let mut buf = [0u8; 4];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    sample.num_v = i32::from_le_bytes(buf);
                }
            }
        }
        
        // Read uOrder
        if let Some(order_prop) = geom.property_by_name("uOrder") {
            if let Some(scalar) = order_prop.as_scalar() {
                let mut buf = [0u8; 4];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    sample.u_order = i32::from_le_bytes(buf);
                }
            }
        }
        
        // Read vOrder
        if let Some(order_prop) = geom.property_by_name("vOrder") {
            if let Some(scalar) = order_prop.as_scalar() {
                let mut buf = [0u8; 4];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    sample.v_order = i32::from_le_bytes(buf);
                }
            }
        }
        
        // Read uKnot
        if let Some(knot_prop) = geom.property_by_name("uKnot") {
            if let Some(array_reader) = knot_prop.as_array() {
                let data = array_reader.read_sample_vec(index)?;
                sample.u_knots = bytemuck::cast_slice(&data).to_vec();
            }
        }
        
        // Read vKnot
        if let Some(knot_prop) = geom.property_by_name("vKnot") {
            if let Some(array_reader) = knot_prop.as_array() {
                let data = array_reader.read_sample_vec(index)?;
                sample.v_knots = bytemuck::cast_slice(&data).to_vec();
            }
        }
        
        // Read Pw (position weights) if present
        if let Some(pw_prop) = geom.property_by_name("Pw") {
            if let Some(array_reader) = pw_prop.as_array() {
                if let Ok(data) = array_reader.read_sample_vec(index) {
                    sample.position_weights = Some(bytemuck::cast_slice(&data).to_vec());
                }
            }
        }
        
        // Read velocities if present
        if let Some(v_prop) = geom.property_by_name(".velocities") {
            if let Some(array_reader) = v_prop.as_array() {
                if let Ok(data) = array_reader.read_sample_vec(index) {
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    sample.velocities = Some(
                        floats.chunks_exact(3)
                            .map(|c| glam::vec3(c[0], c[1], c[2]))
                            .collect()
                    );
                }
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
        
        // Read trim curve data if present
        sample.trim_curve = self.read_trim_curve(&geom, index)?;
        
        // Read .selfBnds if present
        if let Some(bnds_prop) = geom.property_by_name(".selfBnds") {
            if let Some(scalar) = bnds_prop.as_scalar() {
                let mut buf = [0u8; 48];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    let values: &[f64] = bytemuck::cast_slice(&buf);
                    if values.len() >= 6 {
                        sample.self_bounds = Some(BBox3d::new(
                            glam::dvec3(values[0], values[1], values[2]),
                            glam::dvec3(values[3], values[4], values[5]),
                        ));
                    }
                }
            }
        }
        
        Ok(sample)
    }
    
    /// Read trim curve data from properties.
    fn read_trim_curve(&self, geom: &crate::abc::ICompoundProperty<'_>, index: usize) -> Result<Option<TrimCurveData>> {
        let mut trim = TrimCurveData::default();
        
        // Read trim_nloops
        if let Some(prop) = geom.property_by_name("trim_nloops") {
            if let Some(scalar) = prop.as_scalar() {
                let mut buf = [0u8; 4];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    trim.num_loops = i32::from_le_bytes(buf);
                }
            }
        }
        
        // If no loops, no trim curve
        if trim.num_loops == 0 {
            return Ok(None);
        }
        
        // Read trim_ncurves
        if let Some(prop) = geom.property_by_name("trim_ncurves") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.num_curves = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_n
        if let Some(prop) = geom.property_by_name("trim_n") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.num_vertices = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_order
        if let Some(prop) = geom.property_by_name("trim_order") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.orders = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_knot
        if let Some(prop) = geom.property_by_name("trim_knot") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.knots = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_min
        if let Some(prop) = geom.property_by_name("trim_min") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.mins = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_max
        if let Some(prop) = geom.property_by_name("trim_max") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.maxes = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_u
        if let Some(prop) = geom.property_by_name("trim_u") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.u = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_v
        if let Some(prop) = geom.property_by_name("trim_v") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.v = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        // Read trim_w
        if let Some(prop) = geom.property_by_name("trim_w") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.w = bytemuck::cast_slice(&data).to_vec();
                }
            }
        }
        
        Ok(Some(trim))
    }
    
    /// Check if this patch has trim curves.
    pub fn has_trim_curve(&self) -> bool {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return false };
        let Some(geom) = geom_prop.as_compound() else { return false };
        geom.has_property("trim_nloops")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nupatch_sample_empty() {
        let sample = NuPatchSample::new();
        assert!(!sample.is_valid());
        assert_eq!(sample.num_cvs(), 0);
    }
    
    #[test]
    fn test_nupatch_sample_basic() {
        let mut sample = NuPatchSample::new();
        sample.positions = vec![
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(1.0, 0.0, 0.0),
            glam::vec3(0.0, 1.0, 0.0),
            glam::vec3(1.0, 1.0, 0.0),
        ];
        sample.num_u = 2;
        sample.num_v = 2;
        sample.u_order = 2;
        sample.v_order = 2;
        sample.u_knots = vec![0.0, 0.0, 1.0, 1.0];
        sample.v_knots = vec![0.0, 0.0, 1.0, 1.0];
        
        assert!(sample.is_valid());
        assert_eq!(sample.num_cvs(), 4);
        assert_eq!(sample.expected_cvs(), 4);
        assert!(!sample.is_rational());
        assert!(!sample.has_trim_curve());
        assert_eq!(sample.u_degree(), 1);
        assert_eq!(sample.v_degree(), 1);
    }
    
    #[test]
    fn test_nupatch_rational() {
        let mut sample = NuPatchSample::new();
        sample.positions = vec![glam::vec3(0.0, 0.0, 0.0)];
        sample.position_weights = Some(vec![1.0]);
        
        assert!(sample.is_rational());
    }
}
