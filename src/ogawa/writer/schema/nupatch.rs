//! NuPatch schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/ONuPatch.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/ONuPatch.h`

use crate::core::MetaData;
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};
use super::util::{bounds_meta, compute_bounds_vec3};

/// NuPatch sample data for output.
pub struct ONuPatchSample {
    pub positions: Vec<glam::Vec3>,
    pub num_u: i32,
    pub num_v: i32,
    pub u_order: i32,
    pub v_order: i32,
    pub u_knot: Vec<f32>,
    pub v_knot: Vec<f32>,
    pub position_weights: Option<Vec<f32>>,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub uvs: Option<Vec<glam::Vec2>>,
    pub normals: Option<Vec<glam::Vec3>>,
}

impl ONuPatchSample {
    /// Create new NuPatch sample.
    pub fn new(
        positions: Vec<glam::Vec3>,
        num_u: i32,
        num_v: i32,
        u_order: i32,
        v_order: i32,
        u_knot: Vec<f32>,
        v_knot: Vec<f32>,
    ) -> Self {
        Self {
            positions,
            num_u,
            num_v,
            u_order,
            v_order,
            u_knot,
            v_knot,
            position_weights: None,
            velocities: None,
            uvs: None,
            normals: None,
        }
    }
}

/// NuPatch schema writer.
pub struct ONuPatch {
    object: OObject,
    geom_compound: OProperty,
    time_sampling_index: u32,
}

impl ONuPatch {
    /// Create new NuPatch.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_NuPatch_v2");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_NuPatch_v2:.geom");
        object.meta_data = meta;

        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_NuPatch_v2");
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
    pub fn add_sample(&mut self, sample: &ONuPatchSample) {
        let bounds = compute_bounds_vec3(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(
            ".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6),
            bounds_meta(),
        );
        self_bnds_prop.data_write_order = 12;
        self_bnds_prop.add_scalar_pod(&bounds);

        let p_prop = self.get_or_create_array_with_meta(
            "P",
            DataType::new(PlainOldDataType::Float32, 3),
            Self::p_meta(),
        );
        p_prop.data_write_order = 0;
        p_prop.add_array_pod(&sample.positions);

        let nu_prop = self.geom_compound.get_or_create_scalar_child(
            "nu",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        nu_prop.data_write_order = 1;
        nu_prop.add_scalar_pod(&sample.num_u);

        let nv_prop = self.geom_compound.get_or_create_scalar_child(
            "nv",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        nv_prop.data_write_order = 2;
        nv_prop.add_scalar_pod(&sample.num_v);

        let uo_prop = self.geom_compound.get_or_create_scalar_child(
            "uOrder",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        uo_prop.data_write_order = 3;
        uo_prop.add_scalar_pod(&sample.u_order);

        let vo_prop = self.geom_compound.get_or_create_scalar_child(
            "vOrder",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        vo_prop.data_write_order = 4;
        vo_prop.add_scalar_pod(&sample.v_order);

        let uk_prop = self.geom_compound.get_or_create_array_child(
            "uKnot",
            DataType::new(PlainOldDataType::Float32, 1),
        );
        uk_prop.data_write_order = 5;
        uk_prop.add_array_pod(&sample.u_knot);

        let vk_prop = self.geom_compound.get_or_create_array_child(
            "vKnot",
            DataType::new(PlainOldDataType::Float32, 1),
        );
        vk_prop.data_write_order = 6;
        vk_prop.add_array_pod(&sample.v_knot);

        if let Some(ref weights) = sample.position_weights {
            let prop = self.geom_compound.get_or_create_array_child(
                "w",
                DataType::new(PlainOldDataType::Float32, 1),
            );
            prop.data_write_order = 8;
            prop.add_array_pod(weights);
        }

        if let Some(ref vels) = sample.velocities {
            let prop = self.geom_compound.get_or_create_array_child(
                ".velocities",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            prop.data_write_order = 9;
            prop.add_array_pod(vels);
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
            vals_prop.data_write_order = 10;
            vals_prop.add_array_pod(uvs);
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
            vals_prop.data_write_order = 11;
            vals_prop.add_array_pod(normals);
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
