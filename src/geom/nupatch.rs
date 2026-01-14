//! NURBS Patch schema implementation.
//!
//! Provides reading of NURBS surface data from Alembic files.

use crate::abc::IObject;
use crate::geom::util as geom_util;
use crate::util::{Result, Error, BBox3d};
use crate::core::TopologyVariance;

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

/// A single trim curve within a trim loop.
#[derive(Clone, Debug)]
pub struct TrimCurve {
    /// Order of this curve.
    pub order: i32,
    /// Number of vertices (control points).
    pub num_vertices: i32,
    /// Knot vector for this curve.
    pub knots: Vec<f32>,
    /// Minimum parameter value.
    pub min: f32,
    /// Maximum parameter value.
    pub max: f32,
    /// U coordinates of control points.
    pub u: Vec<f32>,
    /// V coordinates of control points.
    pub v: Vec<f32>,
    /// W (weight) coordinates of control points.
    pub w: Vec<f32>,
}

impl TrimCurveData {
    /// Get the number of trim loops.
    pub fn num_loops(&self) -> usize {
        self.num_loops as usize
    }
    
    /// Get number of curves in a specific loop.
    pub fn num_curves_in_loop(&self, loop_idx: usize) -> usize {
        self.num_curves.get(loop_idx).copied().unwrap_or(0) as usize
    }
    
    /// Get total number of trim curves across all loops.
    pub fn total_curves(&self) -> usize {
        self.num_curves.iter().map(|&n| n as usize).sum()
    }
    
    /// Check if this trim data is valid.
    pub fn is_valid(&self) -> bool {
        self.num_loops > 0 && !self.num_curves.is_empty()
    }
    
    /// Get a specific trim curve by global index.
    /// 
    /// Returns None if index is out of range.
    pub fn curve(&self, curve_idx: usize) -> Option<TrimCurve> {
        let total = self.total_curves();
        if curve_idx >= total {
            return None;
        }
        
        // Calculate offsets
        let order = *self.orders.get(curve_idx)?;
        let num_verts = *self.num_vertices.get(curve_idx)? as usize;
        let min = *self.mins.get(curve_idx)?;
        let max = *self.maxes.get(curve_idx)?;
        
        // Calculate knot offset (sum of knots for previous curves)
        let mut knot_offset = 0usize;
        for i in 0..curve_idx {
            let n = self.num_vertices.get(i).copied().unwrap_or(0) as usize;
            let o = self.orders.get(i).copied().unwrap_or(0) as usize;
            knot_offset += n + o;
        }
        let num_knots = num_verts + order as usize;
        let knots = self.knots.get(knot_offset..knot_offset + num_knots)?.to_vec();
        
        // Calculate vertex offset
        let mut vert_offset = 0usize;
        for i in 0..curve_idx {
            vert_offset += self.num_vertices.get(i).copied().unwrap_or(0) as usize;
        }
        
        let u = self.u.get(vert_offset..vert_offset + num_verts)?.to_vec();
        let v = self.v.get(vert_offset..vert_offset + num_verts)?.to_vec();
        let w = self.w.get(vert_offset..vert_offset + num_verts)?.to_vec();
        
        Some(TrimCurve {
            order,
            num_vertices: num_verts as i32,
            knots,
            min,
            max,
            u,
            v,
            w,
        })
    }
    
    /// Get curve at (loop_idx, curve_in_loop_idx).
    pub fn curve_in_loop(&self, loop_idx: usize, curve_in_loop: usize) -> Option<TrimCurve> {
        if loop_idx >= self.num_loops() {
            return None;
        }
        
        // Calculate global curve index
        let mut global_idx = 0usize;
        for i in 0..loop_idx {
            global_idx += self.num_curves_in_loop(i);
        }
        global_idx += curve_in_loop;
        
        self.curve(global_idx)
    }
    
    /// Iterate over all trim curves.
    pub fn curves(&self) -> impl Iterator<Item = TrimCurve> + '_ {
        (0..self.total_curves()).filter_map(|i| self.curve(i))
    }
    
    /// Iterate over curves in a specific loop.
    pub fn curves_in_loop(&self, loop_idx: usize) -> impl Iterator<Item = TrimCurve> + '_ {
        let num_curves = self.num_curves_in_loop(loop_idx);
        (0..num_curves).filter_map(move |i| self.curve_in_loop(loop_idx, i))
    }
}

impl TrimCurve {
    /// Get degree of the curve (order - 1).
    pub fn degree(&self) -> i32 {
        self.order - 1
    }
    
    /// Get UV control points as Vec2 array.
    pub fn control_points(&self) -> Vec<glam::Vec2> {
        self.u.iter().zip(self.v.iter())
            .map(|(&u, &v)| glam::vec2(u, v))
            .collect()
    }
    
    /// Get weighted control points as Vec3 (u, v, w) array.
    pub fn weighted_control_points(&self) -> Vec<glam::Vec3> {
        self.u.iter().zip(self.v.iter()).zip(self.w.iter())
            .map(|((&u, &v), &w)| glam::vec3(u, v, w))
            .collect()
    }
    
    /// Check if this is a rational curve (weights != 1).
    pub fn is_rational(&self) -> bool {
        self.w.iter().any(|&w| (w - 1.0).abs() > 1e-6)
    }
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
        geom_util::num_samples_from_positions(self.object)
    }
    
    /// Check if this patch is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Get time sampling index from positions property.
    pub fn time_sampling_index(&self) -> u32 {
        geom_util::positions_time_sampling_index(self.object)
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        geom_util::child_bounds_time_sampling_index(self.object)
    }
    
    /// Check if patch has self bounds property.
    pub fn has_self_bounds(&self) -> bool {
        geom_util::has_self_bounds(self.object)
    }
    
    /// Get topology variance.
    /// 
    /// NuPatch is typically homogeneous (CVs move but topology is fixed).
    pub fn topology_variance(&self) -> TopologyVariance {
        if self.num_samples() <= 1 {
            TopologyVariance::Static
        } else {
            // NURBS patches typically have fixed topology
            TopologyVariance::Homogeneous
        }
    }
    
    /// Check if patch has arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        geom_util::has_arb_geom_params(self.object)
    }
    
    /// Check if patch has user properties.
    pub fn has_user_properties(&self) -> bool {
        geom_util::has_user_properties(self.object)
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<NuPatchSample> {
        let mut sample = NuPatchSample::new();
        
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        let g = geom.as_reader();
        
        // Read core geometry using helpers
        if let Some(pos) = geom_util::read_vec3_array(g, "P", index) {
            sample.positions = pos;
        }
        
        // Read NURBS parameters (scalars)
        if let Some(nu) = geom_util::read_i32_scalar(g, "nu", index) {
            sample.num_u = nu;
        }
        if let Some(nv) = geom_util::read_i32_scalar(g, "nv", index) {
            sample.num_v = nv;
        }
        if let Some(uo) = geom_util::read_i32_scalar(g, "uOrder", index) {
            sample.u_order = uo;
        }
        if let Some(vo) = geom_util::read_i32_scalar(g, "vOrder", index) {
            sample.v_order = vo;
        }
        
        // Read knots
        if let Some(uk) = geom_util::read_f32_array(g, "uKnot", index) {
            sample.u_knots = uk;
        }
        if let Some(vk) = geom_util::read_f32_array(g, "vKnot", index) {
            sample.v_knots = vk;
        }
        
        // Read optional attributes
        sample.position_weights = geom_util::read_f32_array(g, "Pw", index);
        sample.velocities = geom_util::read_vec3_array(g, ".velocities", index);
        sample.normals = geom_util::read_vec3_array(g, "N", index);
        sample.uvs = geom_util::read_vec2_array(g, "uv", index);
        
        // Read trim curve data if present
        sample.trim_curve = self.read_trim_curve(&geom, index)?;
        
        // Read bounds
        sample.self_bounds = geom_util::read_self_bounds(g, index);
        
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
                    trim.num_curves = bytemuck::try_cast_slice::<_, i32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_n
        if let Some(prop) = geom.property_by_name("trim_n") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.num_vertices = bytemuck::try_cast_slice::<_, i32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_order
        if let Some(prop) = geom.property_by_name("trim_order") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.orders = bytemuck::try_cast_slice::<_, i32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_knot
        if let Some(prop) = geom.property_by_name("trim_knot") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.knots = bytemuck::try_cast_slice::<_, f32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_min
        if let Some(prop) = geom.property_by_name("trim_min") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.mins = bytemuck::try_cast_slice::<_, f32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_max
        if let Some(prop) = geom.property_by_name("trim_max") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.maxes = bytemuck::try_cast_slice::<_, f32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_u
        if let Some(prop) = geom.property_by_name("trim_u") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.u = bytemuck::try_cast_slice::<_, f32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_v
        if let Some(prop) = geom.property_by_name("trim_v") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.v = bytemuck::try_cast_slice::<_, f32>(&data).map(|s| s.to_vec()).unwrap_or_default();
                }
            }
        }
        
        // Read trim_w
        if let Some(prop) = geom.property_by_name("trim_w") {
            if let Some(array) = prop.as_array() {
                if let Ok(data) = array.read_sample_vec(index) {
                    trim.w = bytemuck::try_cast_slice::<_, f32>(&data).map(|s| s.to_vec()).unwrap_or_default();
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
