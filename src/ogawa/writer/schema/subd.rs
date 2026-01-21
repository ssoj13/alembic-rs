//! SubD schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OSubD.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/OSubD.h`

use crate::core::MetaData;
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};
use super::util::{bounds_meta, compute_bounds_vec3};

/// SubD sample data.
pub struct OSubDSample {
    pub positions: Vec<glam::Vec3>,
    pub face_counts: Vec<i32>,
    pub face_indices: Vec<i32>,
    pub subdivision_scheme: String,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub uvs: Option<Vec<glam::Vec2>>,
    pub uv_indices: Option<Vec<i32>>,
    pub crease_indices: Option<Vec<i32>>,
    pub crease_lengths: Option<Vec<i32>>,
    pub crease_sharpnesses: Option<Vec<f32>>,
    pub corner_indices: Option<Vec<i32>>,
    pub corner_sharpnesses: Option<Vec<f32>>,
    pub holes: Option<Vec<i32>>,
    pub normals: Option<Vec<glam::Vec3>>,
    pub normal_indices: Option<Vec<i32>>,
}

impl OSubDSample {
    /// Create new SubD sample with required geometry data.
    pub fn new(
        positions: Vec<glam::Vec3>,
        face_counts: Vec<i32>,
        face_indices: Vec<i32>,
    ) -> Self {
        Self {
            positions,
            face_counts,
            face_indices,
            subdivision_scheme: "catmullClark".to_string(),
            velocities: None,
            uvs: None,
            uv_indices: None,
            crease_indices: None,
            crease_lengths: None,
            crease_sharpnesses: None,
            corner_indices: None,
            corner_sharpnesses: None,
            holes: None,
            normals: None,
            normal_indices: None,
        }
    }
    
    /// Set subdivision scheme (builder pattern).
    /// Common values: "catmullClark", "loop", "bilinear"
    pub fn with_scheme(mut self, scheme: impl Into<String>) -> Self {
        self.subdivision_scheme = scheme.into();
        self
    }
}

/// SubD schema writer.
pub struct OSubD {
    object: OObject,
    geom_compound: OProperty,
    time_sampling_index: u32,
}

impl OSubD {
    /// Create new SubD.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_SubD_v2");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_SubD_v2:.geom");
        object.meta_data = meta;

        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_SubD_v2");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;

        Self { object, geom_compound: geom, time_sampling_index: 0 }
    }

    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }

    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OSubDSample) {
        let bounds = compute_bounds_vec3(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(
            ".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6),
            bounds_meta(),
        );
        self_bnds_prop.data_write_order = 4;
        self_bnds_prop.add_scalar_pod(&bounds);

        let p_prop = self.get_or_create_array_with_meta(
            "P",
            DataType::new(PlainOldDataType::Float32, 3),
            Self::p_meta(),
        );
        p_prop.data_write_order = 0;
        p_prop.add_array_pod(&sample.positions);

        let fi_prop = self.geom_compound.get_or_create_array_child(
            ".faceIndices",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        fi_prop.data_write_order = 1;
        fi_prop.add_array_pod(&sample.face_indices);

        let fc_prop = self.geom_compound.get_or_create_array_child(
            ".faceCounts",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        fc_prop.data_write_order = 2;
        fc_prop.add_array_pod(&sample.face_counts);

        let scheme_prop = self.geom_compound.get_or_create_scalar_child(
            ".scheme",
            DataType::new(PlainOldDataType::String, 1),
        );
        scheme_prop.data_write_order = 10;
        scheme_prop.add_scalar_string(&sample.subdivision_scheme);

        if let Some(ref vels) = sample.velocities {
            let v_prop = self.geom_compound.get_or_create_array_child(
                ".velocities",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            v_prop.data_write_order = 3;
            v_prop.add_array_pod(vels);
        }

        if let Some(ref indices) = sample.crease_indices {
            let prop = self.geom_compound.get_or_create_array_child(
                ".creaseIndices",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            prop.data_write_order = 11;
            prop.add_array_pod(indices);
        }
        if let Some(ref lengths) = sample.crease_lengths {
            let prop = self.geom_compound.get_or_create_array_child(
                ".creaseLengths",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            prop.data_write_order = 12;
            prop.add_array_pod(lengths);
        }
        if let Some(ref sharpnesses) = sample.crease_sharpnesses {
            let prop = self.geom_compound.get_or_create_array_child(
                ".creaseSharpnesses",
                DataType::new(PlainOldDataType::Float32, 1),
            );
            prop.data_write_order = 13;
            prop.add_array_pod(sharpnesses);
        }

        if let Some(ref indices) = sample.corner_indices {
            let prop = self.geom_compound.get_or_create_array_child(
                ".cornerIndices",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            prop.data_write_order = 14;
            prop.add_array_pod(indices);
        }
        if let Some(ref sharpnesses) = sample.corner_sharpnesses {
            let prop = self.geom_compound.get_or_create_array_child(
                ".cornerSharpnesses",
                DataType::new(PlainOldDataType::Float32, 1),
            );
            prop.data_write_order = 15;
            prop.add_array_pod(sharpnesses);
        }

        if let Some(ref holes) = sample.holes {
            let prop = self.geom_compound.get_or_create_array_child(
                ".holes",
                DataType::new(PlainOldDataType::Int32, 1),
            );
            prop.data_write_order = 16;
            prop.add_array_pod(holes);
        }

        if let Some(ref uvs) = sample.uvs {
            let uv_compound = self.geom_compound.get_or_create_compound_child("uv");
            uv_compound.meta_data.set("isGeomParam", "true");
            uv_compound.meta_data.set("podName", "float32_t");
            uv_compound.meta_data.set("podExtent", "2");
            uv_compound.meta_data.set("geoScope", "fvr");
            let vals_prop = uv_compound.get_or_create_array_child(
                ".vals",
                DataType::new(PlainOldDataType::Float32, 2),
            );
            vals_prop.time_sampling_index = self.time_sampling_index;
            vals_prop.data_write_order = 5;
            vals_prop.add_array_pod(uvs);

            if let Some(ref uvi) = sample.uv_indices {
                let idx_prop = uv_compound.get_or_create_array_child(
                    ".indices",
                    DataType::new(PlainOldDataType::Int32, 1),
                );
                idx_prop.time_sampling_index = self.time_sampling_index;
                idx_prop.data_write_order = 6;
                idx_prop.add_array_pod(uvi);
            }
        }

        if let Some(ref normals) = sample.normals {
            let n_compound = self.geom_compound.get_or_create_compound_child("N");
            n_compound.meta_data.set("isGeomParam", "true");
            n_compound.meta_data.set("podName", "float32_t");
            n_compound.meta_data.set("podExtent", "3");
            n_compound.meta_data.set("geoScope", "fvr");
            let vals_prop = n_compound.get_or_create_array_child(
                ".vals",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            vals_prop.time_sampling_index = self.time_sampling_index;
            vals_prop.data_write_order = 7;
            vals_prop.add_array_pod(normals);

            if let Some(ref ni) = sample.normal_indices {
                let idx_prop = n_compound.get_or_create_array_child(
                    ".indices",
                    DataType::new(PlainOldDataType::Int32, 1),
                );
                idx_prop.time_sampling_index = self.time_sampling_index;
                idx_prop.data_write_order = 8;
                idx_prop.add_array_pod(ni);
            }
        }
    }

    fn p_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("geoScope", "vtx");
        meta.set("interpretation", "point");
        meta
    }

    fn get_or_create_array_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }

    fn get_or_create_scalar_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }

    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}
