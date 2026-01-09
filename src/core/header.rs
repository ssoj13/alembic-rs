//! Headers for objects and properties.
//!
//! Headers contain metadata about objects and properties in the archive.

use crate::util::DataType;
use super::MetaData;

/// Header information for an object in the hierarchy.
#[derive(Clone, Debug)]
pub struct ObjectHeader {
    /// Name of this object (not full path).
    pub name: String,
    /// Full path from root (e.g., "/root/parent/child").
    pub full_name: String,
    /// Metadata containing schema info, etc.
    pub meta_data: MetaData,
}

impl ObjectHeader {
    /// Create a new object header.
    pub fn new(name: impl Into<String>, full_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            full_name: full_name.into(),
            meta_data: MetaData::new(),
        }
    }

    /// Create with metadata.
    pub fn with_meta_data(
        name: impl Into<String>,
        full_name: impl Into<String>,
        meta_data: MetaData,
    ) -> Self {
        Self {
            name: name.into(),
            full_name: full_name.into(),
            meta_data,
        }
    }

    /// Get the schema name from metadata.
    pub fn schema(&self) -> Option<&str> {
        self.meta_data.schema()
    }
}

impl Default for ObjectHeader {
    fn default() -> Self {
        Self {
            name: String::new(),
            full_name: String::new(),
            meta_data: MetaData::new(),
        }
    }
}

/// Header information for a property.
#[derive(Clone, Debug)]
pub struct PropertyHeader {
    /// Name of this property.
    pub name: String,
    /// Property type.
    pub property_type: PropertyType,
    /// Data type (POD + extent).
    pub data_type: DataType,
    /// Time sampling index (0 = identity/static).
    pub time_sampling_index: u32,
    /// Metadata.
    pub meta_data: MetaData,
}

impl PropertyHeader {
    /// Create a scalar property header.
    pub fn scalar(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            property_type: PropertyType::Scalar,
            data_type,
            time_sampling_index: 0,
            meta_data: MetaData::new(),
        }
    }

    /// Create an array property header.
    pub fn array(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            property_type: PropertyType::Array,
            data_type,
            time_sampling_index: 0,
            meta_data: MetaData::new(),
        }
    }

    /// Create a compound property header.
    pub fn compound(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            property_type: PropertyType::Compound,
            data_type: DataType::UNKNOWN,
            time_sampling_index: 0,
            meta_data: MetaData::new(),
        }
    }

    /// Set time sampling index.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }

    /// Set metadata.
    pub fn with_meta_data(mut self, meta_data: MetaData) -> Self {
        self.meta_data = meta_data;
        self
    }

    /// Check if this is a scalar property.
    pub fn is_scalar(&self) -> bool {
        self.property_type == PropertyType::Scalar
    }

    /// Check if this is an array property.
    pub fn is_array(&self) -> bool {
        self.property_type == PropertyType::Array
    }

    /// Check if this is a compound property.
    pub fn is_compound(&self) -> bool {
        self.property_type == PropertyType::Compound
    }

    /// Get the interpretation from metadata (e.g., "point", "vector", "normal").
    pub fn interpretation(&self) -> Option<&str> {
        self.meta_data.interpretation()
    }
}

/// Type of property.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyType {
    /// Single value per sample.
    Scalar,
    /// Array of values per sample.
    Array,
    /// Container for other properties.
    Compound,
}

impl Default for PropertyType {
    fn default() -> Self {
        Self::Scalar
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::DataType;

    #[test]
    fn test_object_header() {
        let header = ObjectHeader::new("mesh", "/root/mesh");
        assert_eq!(header.name, "mesh");
        assert_eq!(header.full_name, "/root/mesh");
    }

    #[test]
    fn test_property_header_scalar() {
        let header = PropertyHeader::scalar("P", DataType::VEC3F);
        assert!(header.is_scalar());
        assert!(!header.is_array());
        assert_eq!(header.data_type, DataType::VEC3F);
    }

    #[test]
    fn test_property_header_array() {
        let header = PropertyHeader::array("vertices", DataType::VEC3F)
            .with_time_sampling(1);
        assert!(header.is_array());
        assert_eq!(header.time_sampling_index, 1);
    }
}
