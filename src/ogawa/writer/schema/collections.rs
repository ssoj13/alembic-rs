//! Collections schema writer.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCollection/OCollections.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCollection/OCollections.h`

use std::collections::HashMap;

use crate::core::MetaData;
use crate::util::{DataType, PlainOldDataType};

use super::super::object::OObject;
use super::super::property::{OProperty, OPropertyData};
use super::super::write_util::encode_string_array;

/// Collections sample data for output.
pub struct OCollectionsSample {
    pub collections: HashMap<String, Vec<String>>,
}

impl OCollectionsSample {
    /// Create empty collections sample.
    pub fn new() -> Self {
        Self { collections: HashMap::new() }
    }

    /// Add a collection.
    pub fn add_collection(&mut self, name: &str, paths: Vec<String>) {
        self.collections.insert(name.to_string(), paths);
    }
}

impl Default for OCollectionsSample {
    fn default() -> Self {
        Self::new()
    }
}

/// Collections schema writer.
pub struct OCollections {
    object: OObject,
    sample: OCollectionsSample,
}

impl OCollections {
    /// Create new Collections.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcCollection_Collection_v1");
        object.meta_data = meta;

        Self { object, sample: OCollectionsSample::new() }
    }

    /// Add a collection.
    pub fn add_collection(&mut self, name: &str, paths: Vec<String>) {
        self.sample.add_collection(name, paths);
    }

    /// Build the object.
    pub fn build(mut self) -> OObject {
        let mut coll = OProperty::compound(".collections");

        for (name, paths) in &self.sample.collections {
            let paths_data = encode_string_array(paths);

            let mut prop = OProperty::array(name, DataType::new(PlainOldDataType::String, 1));
            prop.add_array_sample(&paths_data, &[paths.len()]);

            if let OPropertyData::Compound(children) = &mut coll.data {
                children.push(prop);
            }
        }

        self.object.properties.push(coll);
        self.object
    }
}
