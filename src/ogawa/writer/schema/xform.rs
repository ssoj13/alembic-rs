//! Xform schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OXform.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/XformOp.cpp`

use crate::core::MetaData;
use crate::geom::{XformOp, XformOpType};
use crate::util::{BBox3d, DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};

/// Xform sample data.
pub struct OXformSample {
    /// Transform operations (decomposed or single matrix).
    pub ops: Vec<XformOp>,
    pub inherits: bool,
}

impl OXformSample {
    /// Create identity sample.
    pub fn identity() -> Self {
        Self { ops: Vec::new(), inherits: true }
    }

    /// Create from matrix (single Matrix operation).
    pub fn from_matrix(matrix: glam::Mat4, inherits: bool) -> Self {
        let mat_f64: [f64; 16] = [
            matrix.x_axis.x as f64,
            matrix.x_axis.y as f64,
            matrix.x_axis.z as f64,
            matrix.x_axis.w as f64,
            matrix.y_axis.x as f64,
            matrix.y_axis.y as f64,
            matrix.y_axis.z as f64,
            matrix.y_axis.w as f64,
            matrix.z_axis.x as f64,
            matrix.z_axis.y as f64,
            matrix.z_axis.z as f64,
            matrix.z_axis.w as f64,
            matrix.w_axis.x as f64,
            matrix.w_axis.y as f64,
            matrix.w_axis.z as f64,
            matrix.w_axis.w as f64,
        ];
        Self { ops: vec![XformOp::matrix(mat_f64)], inherits }
    }

    /// Create from XformSample ops (preserves decomposed operations).
    pub fn from_ops(ops: Vec<XformOp>, inherits: bool) -> Self {
        Self { ops, inherits }
    }
}

/// Xform schema writer.
pub struct OXform {
    object: OObject,
    samples: Vec<OXformSample>,
    time_sampling_index: u32,
    /// Child bounds samples (bounding box of all children).
    child_bounds: Vec<BBox3d>,
    /// Time sampling index for child bounds (may differ from transform ts).
    child_bounds_ts_index: u32,
}

impl OXform {
    /// Create new Xform.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Xform_v3");
        meta.set("schemaObjTitle", "AbcGeom_Xform_v3:.xform");
        object.meta_data = meta;

        Self {
            object,
            samples: Vec::new(),
            time_sampling_index: 0,
            child_bounds: Vec::new(),
            child_bounds_ts_index: 0,
        }
    }

    /// Set time sampling index for animated properties.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }

    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }

    /// Add a sample.
    pub fn add_sample(&mut self, sample: OXformSample) {
        self.samples.push(sample);
    }

    /// Add child bounds sample (bounding box of all children).
    pub fn add_child_bounds(&mut self, bounds: BBox3d) {
        self.child_bounds.push(bounds);
    }

    /// Set child bounds time sampling index.
    pub fn set_child_bounds_time_sampling(&mut self, index: u32) {
        self.child_bounds_ts_index = index;
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.samples.is_empty() {
            let mut geom = OProperty::compound(".xform");
            let mut geom_meta = MetaData::new();
            geom_meta.set("schema", "AbcGeom_Xform_v3");
            geom.meta_data = geom_meta;

            let is_not_identity = self.samples.iter().any(|s| !s.ops.is_empty());

            if is_not_identity {
                let first_sample = &self.samples[0];
                let num_ops = first_sample.ops.len();
                let total_vals: usize = first_sample.ops.iter().map(|op| op.values.len()).sum();

                let mut inherits = OProperty::scalar(
                    ".inherits",
                    DataType::new(PlainOldDataType::Boolean, 1),
                );
                inherits.time_sampling_index = self.time_sampling_index;
                for sample in &self.samples {
                    inherits.add_scalar_pod(&(sample.inherits as u8));
                }

                let mut ops = OProperty::scalar(
                    ".ops",
                    DataType::new(PlainOldDataType::Uint8, num_ops as u8),
                );
                let op_codes: Vec<u8> = first_sample.ops.iter().map(|op| encode_xform_op(op.op_type)).collect();
                ops.add_scalar_sample(&op_codes);

                let mut vals = OProperty::scalar(
                    ".vals",
                    DataType::new(PlainOldDataType::Float64, total_vals as u8),
                );
                vals.time_sampling_index = self.time_sampling_index;
                for sample in &self.samples {
                    let all_vals: Vec<f64> = sample.ops.iter().flat_map(|op| op.values.iter().copied()).collect();
                    vals.add_scalar_sample(bytemuck::cast_slice(&all_vals));
                }

                let mut not_id = OProperty::scalar(
                    "isNotConstantIdentity",
                    DataType::new(PlainOldDataType::Boolean, 1),
                );
                not_id.add_scalar_pod(&1u8);

                if let OPropertyData::Compound(children) = &mut geom.data {
                    children.push(inherits);
                    children.push(ops);
                    children.push(vals);
                    children.push(not_id);
                }
            }

            if !self.child_bounds.is_empty() {
                let mut bnds = OProperty::scalar(
                    ".childBnds",
                    DataType::new(PlainOldDataType::Float64, 6),
                );
                bnds.time_sampling_index = self.child_bounds_ts_index;
                let mut bnds_meta = MetaData::new();
                bnds_meta.set("interpretation", "box");
                bnds.meta_data = bnds_meta;
                for bounds in &self.child_bounds {
                    let data: [f64; 6] = [
                        bounds.min.x,
                        bounds.min.y,
                        bounds.min.z,
                        bounds.max.x,
                        bounds.max.y,
                        bounds.max.z,
                    ];
                    bnds.add_scalar_sample(bytemuck::cast_slice(&data));
                }
                if let OPropertyData::Compound(children) = &mut geom.data {
                    children.push(bnds);
                }
            }

            self.object.properties.push(geom);
        }

        self.object
    }

    /// Add child xform.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

/// Encode xform operation type to byte.
/// Per XformOp.cpp: getOpEncoding() returns (m_type << 4) | (m_hint & 0xF).
fn encode_xform_op(op_type: XformOpType) -> u8 {
    let type_code = match op_type {
        XformOpType::Scale => 0,
        XformOpType::Translate => 1,
        XformOpType::Rotate => 2,
        XformOpType::Matrix => 3,
        XformOpType::RotateX => 4,
        XformOpType::RotateY => 5,
        XformOpType::RotateZ => 6,
    };
    type_code << 4
}
