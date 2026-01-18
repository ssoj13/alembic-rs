//! FaceSet schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcGeom/OFaceSet.cpp`
//! - `_ref/alembic/lib/Alembic/AbcGeom/OFaceSet.h`

use crate::core::MetaData;
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::OProperty;

/// FaceSet sample data for output.
pub struct OFaceSetSample {
    pub faces: Vec<i32>,
}

impl OFaceSetSample {
    /// Create new FaceSet sample.
    pub fn new(faces: Vec<i32>) -> Self {
        Self { faces }
    }
}

/// FaceSet schema writer.
pub struct OFaceSet {
    object: OObject,
    geom_compound: OProperty,
    time_sampling_index: u32,
}

impl OFaceSet {
    /// Create new FaceSet.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_FaceSet_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_FaceSet_v1:.faceset");
        object.meta_data = meta;

        let mut faceset_meta = MetaData::new();
        faceset_meta.set("schema", "AbcGeom_FaceSet_v1");
        faceset_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut faceset = OProperty::compound(".faceset");
        faceset.meta_data = faceset_meta;

        Self { object, geom_compound: faceset, time_sampling_index: 0 }
    }

    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }

    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OFaceSetSample) {
        let faces_prop = self.geom_compound.get_or_create_array_child(
            ".faces",
            DataType::new(PlainOldDataType::Int32, 1),
        );
        faces_prop.add_array_pod(&sample.faces);
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }
}
