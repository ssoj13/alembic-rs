//! Points schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OPoints.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/OPoints.h`

use crate::core::MetaData;
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};

/// Points sample data for output.
pub struct OPointsSample {
    pub positions: Vec<glam::Vec3>,
    pub ids: Vec<i64>,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub widths: Option<Vec<f32>>,
}

impl OPointsSample {
    /// Create new points sample.
    pub fn new(positions: Vec<glam::Vec3>, ids: Vec<i64>) -> Self {
        Self { positions, ids, velocities: None, widths: None }
    }
}

/// Points schema writer.
pub struct OPoints {
    object: OObject,
    geom_compound: OProperty,
    time_sampling_index: u32,
}

impl OPoints {
    /// Create new Points.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Points_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_Points_v1:.geom");
        object.meta_data = meta;

        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_Points_v1");
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
    pub fn add_sample(&mut self, sample: &OPointsSample) {
        let p_prop = self.get_or_create_array_with_meta(
            "P",
            DataType::new(PlainOldDataType::Float32, 3),
            Self::p_meta(),
        );
        p_prop.add_array_pod(&sample.positions);

        let id_prop = self.geom_compound.get_or_create_array_child(
            "id",
            DataType::new(PlainOldDataType::Int64, 1),
        );
        id_prop.add_array_pod(&sample.ids);

        if let Some(ref vels) = sample.velocities {
            let prop = self.geom_compound.get_or_create_array_child(
                ".velocities",
                DataType::new(PlainOldDataType::Float32, 3),
            );
            prop.add_array_pod(vels);
        }

        if let Some(ref widths) = sample.widths {
            let prop = self.geom_compound.get_or_create_array_child(
                ".widths",
                DataType::new(PlainOldDataType::Float32, 1),
            );
            prop.add_array_pod(widths);
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
