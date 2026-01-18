//! Ogawa object writer types.
//!
//! Reference: `_ref/alembic/lib/Alembic/AbcCoreOgawa/OwData.cpp`.

use crate::core::MetaData;
use crate::util::DataType;

use super::property::OProperty;

/// Object for writing to archive.
#[derive(Clone)]
pub struct OObject {
    /// Object name.
    pub name: String,
    /// Object metadata.
    pub meta_data: MetaData,
    /// Child objects.
    pub children: Vec<OObject>,
    /// Properties.
    pub properties: Vec<OProperty>,
}

impl OObject {
    /// Create a new object.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            meta_data: MetaData::new(),
            children: Vec::new(),
            properties: Vec::new(),
        }
    }

    /// Set metadata.
    pub fn with_meta_data(mut self, md: MetaData) -> Self {
        self.meta_data = md;
        self
    }

    /// Add a child object.
    pub fn add_child(&mut self, child: OObject) -> &mut OObject {
        self.children.push(child);
        self.children.last_mut().unwrap()
    }

    /// Add a property.
    pub fn add_property(&mut self, prop: OProperty) -> &mut OProperty {
        self.properties.push(prop);
        self.properties.last_mut().unwrap()
    }

    /// Create and add a scalar property.
    pub fn add_scalar(&mut self, name: &str, data_type: DataType) -> &mut OProperty {
        let prop = OProperty::scalar(name, data_type);
        self.add_property(prop)
    }

    /// Create and add an array property.
    pub fn add_array(&mut self, name: &str, data_type: DataType) -> &mut OProperty {
        let prop = OProperty::array(name, data_type);
        self.add_property(prop)
    }
}
