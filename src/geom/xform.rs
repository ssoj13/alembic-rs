//! Xform (transform) schema implementation.
//!
//! Provides reading of transform data from Alembic files.

use crate::abc::IObject;
use crate::util::Result;

/// Xform schema identifier.
pub const XFORM_SCHEMA: &str = "AbcGeom_Xform_v3";

/// Transform operation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XformOpType {
    Scale,
    Translate,
    RotateX,
    RotateY,
    RotateZ,
    Rotate,    // axis + angle
    Matrix,
}

/// A single transform operation.
#[derive(Clone, Debug)]
pub struct XformOp {
    pub op_type: XformOpType,
    pub values: Vec<f64>,
}

impl XformOp {
    /// Create a scale operation.
    pub fn scale(x: f64, y: f64, z: f64) -> Self {
        Self { op_type: XformOpType::Scale, values: vec![x, y, z] }
    }
    
    /// Create a translate operation.
    pub fn translate(x: f64, y: f64, z: f64) -> Self {
        Self { op_type: XformOpType::Translate, values: vec![x, y, z] }
    }
    
    /// Create a rotation around X axis (angle in degrees).
    pub fn rotate_x(angle: f64) -> Self {
        Self { op_type: XformOpType::RotateX, values: vec![angle] }
    }
    
    /// Create a rotation around Y axis (angle in degrees).
    pub fn rotate_y(angle: f64) -> Self {
        Self { op_type: XformOpType::RotateY, values: vec![angle] }
    }
    
    /// Create a rotation around Z axis (angle in degrees).
    pub fn rotate_z(angle: f64) -> Self {
        Self { op_type: XformOpType::RotateZ, values: vec![angle] }
    }
    
    /// Create a 4x4 matrix operation.
    pub fn matrix(m: [f64; 16]) -> Self {
        Self { op_type: XformOpType::Matrix, values: m.to_vec() }
    }
}

/// Transform sample with decomposed operations or matrix.
#[derive(Clone, Debug)]
pub struct XformSample {
    /// Transform operations in order.
    pub ops: Vec<XformOp>,
    /// Whether this xform inherits from parent.
    pub inherits: bool,
}

impl Default for XformSample {
    fn default() -> Self {
        Self { ops: Vec::new(), inherits: true }
    }
}

impl XformSample {
    /// Create identity xform.
    pub fn identity() -> Self {
        Self::default()
    }
    
    /// Compute the final 4x4 transformation matrix.
    pub fn matrix(&self) -> glam::Mat4 {
        let mut result = glam::Mat4::IDENTITY;
        
        for op in &self.ops {
            let m = match op.op_type {
                XformOpType::Scale => {
                    let (x, y, z) = (op.values[0] as f32, op.values[1] as f32, op.values[2] as f32);
                    glam::Mat4::from_scale(glam::vec3(x, y, z))
                }
                XformOpType::Translate => {
                    let (x, y, z) = (op.values[0] as f32, op.values[1] as f32, op.values[2] as f32);
                    glam::Mat4::from_translation(glam::vec3(x, y, z))
                }
                XformOpType::RotateX => {
                    let angle = (op.values[0] as f32).to_radians();
                    glam::Mat4::from_rotation_x(angle)
                }
                XformOpType::RotateY => {
                    let angle = (op.values[0] as f32).to_radians();
                    glam::Mat4::from_rotation_y(angle)
                }
                XformOpType::RotateZ => {
                    let angle = (op.values[0] as f32).to_radians();
                    glam::Mat4::from_rotation_z(angle)
                }
                XformOpType::Rotate => {
                    // axis (x, y, z) + angle (degrees)
                    let axis = glam::vec3(
                        op.values[0] as f32,
                        op.values[1] as f32,
                        op.values[2] as f32,
                    ).normalize_or_zero();
                    let angle = (op.values[3] as f32).to_radians();
                    if axis.length_squared() > 0.0001 {
                        glam::Mat4::from_axis_angle(axis, angle)
                    } else {
                        glam::Mat4::IDENTITY
                    }
                }
                XformOpType::Matrix => {
                    // Alembic stores row-major, glam uses column-major
                    // Need to transpose: read as rows, store as columns
                    let v: Vec<f32> = op.values.iter().map(|&x| x as f32).collect();
                    glam::Mat4::from_cols(
                        glam::vec4(v[0], v[4], v[8], v[12]),   // col 0 from row 0s
                        glam::vec4(v[1], v[5], v[9], v[13]),   // col 1 from row 1s
                        glam::vec4(v[2], v[6], v[10], v[14]),  // col 2 from row 2s
                        glam::vec4(v[3], v[7], v[11], v[15]),  // col 3 from row 3s
                    )
                }
            };
            // Alembic uses left-multiply for row-vectors: ret = m * ret
            // For glam column-vectors, equivalent is right-multiply: result = result * m
            result = result * m;
        }
        
        result
    }
    
    /// Get translation component.
    pub fn translation(&self) -> glam::Vec3 {
        for op in &self.ops {
            if op.op_type == XformOpType::Translate {
                return glam::vec3(
                    op.values[0] as f32,
                    op.values[1] as f32,
                    op.values[2] as f32,
                );
            }
        }
        // Fall back to extracting from matrix
        let m = self.matrix();
        m.w_axis.truncate()
    }
    
    /// Get scale component.
    pub fn scale(&self) -> glam::Vec3 {
        for op in &self.ops {
            if op.op_type == XformOpType::Scale {
                return glam::vec3(
                    op.values[0] as f32,
                    op.values[1] as f32,
                    op.values[2] as f32,
                );
            }
        }
        glam::Vec3::ONE
    }
}

/// Input Xform schema reader.
pub struct IXform<'a> {
    object: &'a IObject<'a>,
}

impl<'a> IXform<'a> {
    /// Wrap an IObject as an IXform.
    /// Returns None if the object doesn't have the Xform schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(XFORM_SCHEMA) {
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
    
    /// Check if this is an inheriting xform.
    pub fn is_inheriting(&self) -> bool {
        // Read .inherits property if present
        let props = self.object.properties();
        if let Some(geom_prop) = props.property_by_name(".xform") {
            if let Some(geom) = geom_prop.as_compound() {
                if let Some(inh_prop) = geom.property_by_name(".inherits") {
                    if let Some(scalar) = inh_prop.as_scalar() {
                        let mut buf = [0u8; 1];
                        if scalar.read_sample(0, &mut buf).is_ok() {
                            return buf[0] != 0;
                        }
                    }
                }
            }
        }
        true // Default to inheriting
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        // Read from .vals property
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".xform") else { return 1 };
        let Some(geom) = geom_prop.as_compound() else { return 1 };
        let Some(vals_prop) = geom.property_by_name(".vals") else { return 1 };
        let Some(array_reader) = vals_prop.as_array() else { return 1 };
        array_reader.num_samples()
    }
    
    /// Check if this xform is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Check if this xform is constant AND identity transform.
    pub fn is_constant_identity(&self) -> bool {
        if !self.is_constant() {
            return false;
        }
        if let Ok(sample) = self.get_sample(0) {
            // Check if ops are empty (identity) or matrix is identity
            sample.ops.is_empty() || sample.matrix() == glam::Mat4::IDENTITY
        } else {
            true // No samples = identity
        }
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<XformSample> {
        use crate::util::Error;
        
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".xform")
            .ok_or_else(|| Error::invalid("No .xform property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".xform is not compound"))?;
        
        let mut sample = XformSample::default();
        
        // Read .inherits (static scalar)
        if let Some(inh_prop) = geom.property_by_name(".inherits") {
            if let Some(scalar) = inh_prop.as_scalar() {
                let mut buf = [0u8; 1];
                if scalar.read_sample(0, &mut buf).is_ok() {
                    sample.inherits = buf[0] != 0;
                }
            }
        }
        
        // Read .ops (static scalar with uint8 operation codes)
        // Per IXform.cpp: numOps = ops->getHeader().getDataType().getExtent()
        let ops_data = if let Some(ops_prop) = geom.property_by_name(".ops") {
            if let Some(scalar) = ops_prop.as_scalar() {
                let num_ops = scalar.header().data_type.extent as usize;
                let mut buf = vec![0u8; num_ops];
                if scalar.read_sample(0, &mut buf).is_ok() {
                    Some(buf)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Read .vals (doubles - can be scalar or array)
        // Per IXform.cpp: useArrayProp determines which method
        let vals_data = if let Some(vals_prop) = geom.property_by_name(".vals") {
            if vals_prop.is_scalar() {
                if let Some(scalar) = vals_prop.as_scalar() {
                    // Per IXform.cpp: dataVec.resize( extent )
                    let num_vals = scalar.header().data_type.extent as usize;
                    let byte_count = num_vals * 8; // f64 = 8 bytes
                    let mut buf = vec![0u8; byte_count];
                    if scalar.read_sample(index, &mut buf).is_ok() {
                        Some(buf)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if let Some(array_reader) = vals_prop.as_array() {
                // Array property - size comes from the sample itself
                array_reader.read_sample_vec(index).ok()
            } else {
                None
            }
        } else {
            None
        };
        
        // Parse ops and vals
        if let (Some(ops), Some(vals)) = (ops_data, vals_data) {
            let doubles: &[f64] = bytemuck::try_cast_slice(&vals).unwrap_or(&[]);
            
            let mut val_idx = 0;
            
            for &op_code in &ops {
                let (op_type, num_vals) = decode_xform_op(op_code);
                if val_idx + num_vals > doubles.len() {
                    break;
                }
                
                let values: Vec<f64> = doubles[val_idx..val_idx + num_vals].to_vec();
                val_idx += num_vals;
                
                if let Some(op_type) = op_type {
                    sample.ops.push(XformOp { op_type, values });
                }
            }
        }
        
        Ok(sample)
    }
    
    /// Check if this xform has child bounds property.
    pub fn has_child_bounds(&self) -> bool {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".xform") else {
            return false;
        };
        let Some(geom) = geom_prop.as_compound() else {
            return false;
        };
        geom.has_property(".childBnds")
    }
    
    /// Read child bounds at the given sample index.
    /// Returns the bounding box of all children.
    pub fn child_bounds(&self, index: usize) -> Option<crate::util::BBox3d> {
        let props = self.object.properties();
        let geom_prop = props.property_by_name(".xform")?;
        let geom = geom_prop.as_compound()?;
        let bnds_prop = geom.property_by_name(".childBnds")?;
        let scalar = bnds_prop.as_scalar()?;
        
        let mut buf = [0u8; 48]; // 6 x f64
        if scalar.read_sample(index, &mut buf).is_ok() {
            let doubles: &[f64] = bytemuck::try_cast_slice(&buf).ok()?;
            if doubles.len() >= 6 {
                return Some(crate::util::BBox3d::new(
                    glam::dvec3(doubles[0], doubles[1], doubles[2]),
                    glam::dvec3(doubles[3], doubles[4], doubles[5]),
                ));
            }
        }
        None
    }
    
    /// Get the number of child bounds samples.
    pub fn child_bounds_num_samples(&self) -> usize {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".xform") else { return 0 };
        let Some(geom) = geom_prop.as_compound() else { return 0 };
        let Some(bnds_prop) = geom.property_by_name(".childBnds") else { return 0 };
        let Some(scalar) = bnds_prop.as_scalar() else { return 0 };
        scalar.num_samples()
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".xform") else { return 0 };
        let Some(geom) = geom_prop.as_compound() else { return 0 };
        let Some(bnds_prop) = geom.property_by_name(".childBnds") else { return 0 };
        bnds_prop.header().time_sampling_index
    }
    
    /// Check if this xform has arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".xform") else {
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
        let Some(geom_prop) = props.property_by_name(".xform") else {
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
    
    /// Check if this xform has user properties.
    pub fn has_user_properties(&self) -> bool {
        let props = self.object.properties();
        let Some(geom_prop) = props.property_by_name(".xform") else {
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
        let Some(geom_prop) = props.property_by_name(".xform") else {
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
}

/// Decode xform operation code to type and number of values.
/// Per XformOp.cpp: getOpEncoding() returns (m_type << 4) | (m_hint & 0xF)
fn decode_xform_op(code: u8) -> (Option<XformOpType>, usize) {
    // Upper nibble = op type, lower nibble = hint
    let op_type = code >> 4;
    
    // Foundation.h enum XformOperationType
    match op_type {
        0 => (Some(XformOpType::Scale), 3),      // kScaleOperation
        1 => (Some(XformOpType::Translate), 3),  // kTranslateOperation
        2 => (Some(XformOpType::Rotate), 4),     // kRotateOperation (axis + angle)
        3 => (Some(XformOpType::Matrix), 16),    // kMatrixOperation
        4 => (Some(XformOpType::RotateX), 1),    // kRotateXOperation  
        5 => (Some(XformOpType::RotateY), 1),    // kRotateYOperation
        6 => (Some(XformOpType::RotateZ), 1),    // kRotateZOperation
        _ => (None, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_xform_sample_identity() {
        let sample = XformSample::identity();
        let m = sample.matrix();
        assert_eq!(m, glam::Mat4::IDENTITY);
        assert!(sample.ops.is_empty());
    }
    
    #[test]
    fn test_xform_sample_is_identity() {
        // Empty ops = identity
        let identity = XformSample::identity();
        assert!(identity.ops.is_empty());
        assert_eq!(identity.matrix(), glam::Mat4::IDENTITY);
        
        // Explicit identity scale
        let mut scale_one = XformSample::default();
        scale_one.ops.push(XformOp::scale(1.0, 1.0, 1.0));
        assert_eq!(scale_one.matrix(), glam::Mat4::IDENTITY);
        
        // Non-identity
        let mut translated = XformSample::default();
        translated.ops.push(XformOp::translate(1.0, 0.0, 0.0));
        assert_ne!(translated.matrix(), glam::Mat4::IDENTITY);
    }
    
    #[test]
    fn test_xform_ops() {
        let mut sample = XformSample::default();
        sample.ops.push(XformOp::translate(1.0, 2.0, 3.0));
        sample.ops.push(XformOp::scale(2.0, 2.0, 2.0));
        
        let t = sample.translation();
        assert_eq!(t, glam::vec3(1.0, 2.0, 3.0));
        
        let s = sample.scale();
        assert_eq!(s, glam::vec3(2.0, 2.0, 2.0));
    }
    
    #[test]
    fn test_xform_rotation() {
        let mut sample = XformSample::default();
        sample.ops.push(XformOp::rotate_z(90.0));
        
        let m = sample.matrix();
        // 90 degree Z rotation should swap X and Y
        let v = m.transform_vector3(glam::vec3(1.0, 0.0, 0.0));
        assert!((v.x).abs() < 0.0001);
        assert!((v.y - 1.0).abs() < 0.0001);
    }
}
