//! Abstract traits for Alembic archives, objects, and properties.
//!
//! These traits define the interface between the low-level Ogawa layer
//! and the high-level Abc API.

use crate::core::{ObjectHeader, PropertyHeader, TimeSampling};
use crate::util::Result;

// ============================================================================
// Archive Traits
// ============================================================================

/// Reader interface for an Alembic archive.
pub trait ArchiveReader: Send + Sync {
    /// Get the archive name/path.
    fn name(&self) -> &str;

    /// Get the number of time samplings in the archive.
    fn num_time_samplings(&self) -> usize;

    /// Get a time sampling by index.
    fn time_sampling(&self, index: usize) -> Option<&TimeSampling>;

    /// Get the root object.
    fn root(&self) -> &dyn ObjectReader;

    /// Find an object by full path.
    fn find_object(&self, path: &str) -> Option<Box<dyn ObjectReader + '_>> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Some(Box::new(ObjectReaderRef(self.root())));
        }

        let current: &dyn ObjectReader = self.root();
        for part in parts {
            let child = current.child_by_name(part)?;
            // This is a simplification - real impl needs lifetime management
            return Some(child);
        }
        None
    }
}

/// Writer interface for an Alembic archive.
pub trait ArchiveWriter: Send {
    /// Get the archive name/path.
    fn name(&self) -> &str;

    /// Add a time sampling and return its index.
    fn add_time_sampling(&mut self, ts: TimeSampling) -> u32;

    /// Get the number of time samplings.
    fn num_time_samplings(&self) -> usize;

    /// Get a time sampling by index.
    fn time_sampling(&self, index: usize) -> Option<&TimeSampling>;

    /// Get the root object for writing.
    fn root_mut(&mut self) -> &mut dyn ObjectWriter;

    /// Flush and finalize the archive.
    fn close(self: Box<Self>) -> Result<()>;
}

// ============================================================================
// Object Traits
// ============================================================================

/// Reader interface for an object in the hierarchy.
pub trait ObjectReader: Send + Sync {
    /// Get the object header.
    fn header(&self) -> &ObjectHeader;

    /// Get the parent object (None for root).
    fn parent(&self) -> Option<&dyn ObjectReader>;

    /// Get the number of child objects.
    fn num_children(&self) -> usize;

    /// Get a child by index.
    fn child(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>>;

    /// Get a child by name.
    fn child_by_name(&self, name: &str) -> Option<Box<dyn ObjectReader + '_>>;

    /// Get the properties compound.
    fn properties(&self) -> &dyn CompoundPropertyReader;

    /// Get object name (convenience).
    fn name(&self) -> &str {
        &self.header().name
    }

    /// Get full path (convenience).
    fn full_name(&self) -> &str {
        &self.header().full_name
    }

    /// Check if object matches a schema.
    fn matches_schema(&self, schema: &str) -> bool {
        self.header().meta_data.matches_schema(schema)
    }
}

/// Writer interface for an object.
pub trait ObjectWriter: Send {
    /// Get the object header.
    fn header(&self) -> &ObjectHeader;

    /// Add a child object.
    fn add_child(&mut self, header: ObjectHeader) -> Result<&mut dyn ObjectWriter>;

    /// Get the properties compound for writing.
    fn properties_mut(&mut self) -> &mut dyn CompoundPropertyWriter;

    /// Get object name.
    fn name(&self) -> &str {
        &self.header().name
    }
}

// ============================================================================
// Property Traits
// ============================================================================

/// Base reader interface for any property.
pub trait PropertyReader: Send + Sync {
    /// Get the property header.
    fn header(&self) -> &PropertyHeader;

    /// Check if this is a scalar property.
    fn is_scalar(&self) -> bool {
        self.header().is_scalar()
    }

    /// Check if this is an array property.
    fn is_array(&self) -> bool {
        self.header().is_array()
    }

    /// Check if this is a compound property.
    fn is_compound(&self) -> bool {
        self.header().is_compound()
    }

    /// Get property name.
    fn name(&self) -> &str {
        &self.header().name
    }

    /// Try to cast to scalar property reader.
    fn as_scalar(&self) -> Option<&dyn ScalarPropertyReader> {
        None
    }

    /// Try to cast to array property reader.
    fn as_array(&self) -> Option<&dyn ArrayPropertyReader> {
        None
    }

    /// Try to cast to compound property reader.
    fn as_compound(&self) -> Option<&dyn CompoundPropertyReader> {
        None
    }
}

/// Reader for scalar properties (single value per sample).
pub trait ScalarPropertyReader: PropertyReader {
    /// Get the number of samples.
    fn num_samples(&self) -> usize;

    /// Check if this property is constant (all samples identical).
    fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }

    /// Read a sample into the provided buffer.
    /// Buffer must be large enough for header().data_type.num_bytes().
    fn read_sample(&self, index: usize, out: &mut [u8]) -> Result<()>;

    /// Read sample as typed value.
    fn read_sample_typed<T: bytemuck::Pod + Default>(&self, index: usize) -> Result<T>
    where
        Self: Sized,
    {
        let mut value = T::default();
        let bytes = bytemuck::bytes_of_mut(&mut value);
        self.read_sample(index, bytes)?;
        Ok(value)
    }
}

/// Reader for array properties (array of values per sample).
pub trait ArrayPropertyReader: PropertyReader {
    /// Get the number of samples.
    fn num_samples(&self) -> usize;

    /// Check if this property is constant.
    fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }

    /// Get the size (in elements) of a sample.
    fn sample_len(&self, index: usize) -> Result<usize>;

    /// Read a sample into the provided buffer.
    fn read_sample(&self, index: usize, out: &mut [u8]) -> Result<usize>;

    /// Read sample as Vec of bytes.
    fn read_sample_vec(&self, index: usize) -> Result<Vec<u8>>;

    /// Read sample as typed Vec.
    fn read_sample_typed<T: bytemuck::Pod + Clone>(&self, index: usize) -> Result<Vec<T>>
    where
        Self: Sized,
    {
        let data = self.read_sample_vec(index)?;
        let slice: &[T] = bytemuck::cast_slice(&data);
        Ok(slice.to_vec())
    }
}

/// Reader for compound properties (container of sub-properties).
pub trait CompoundPropertyReader: PropertyReader {
    /// Get the number of sub-properties.
    fn num_properties(&self) -> usize;

    /// Get a property by index.
    fn property(&self, index: usize) -> Option<Box<dyn PropertyReader + '_>>;

    /// Get a property by name.
    fn property_by_name(&self, name: &str) -> Option<Box<dyn PropertyReader + '_>>;

    /// Check if a property exists.
    fn has_property(&self, name: &str) -> bool {
        self.property_by_name(name).is_some()
    }

    /// Iterate over property names.
    fn property_names(&self) -> Vec<String> {
        (0..self.num_properties())
            .filter_map(|i| self.property(i).map(|p| p.name().to_string()))
            .collect()
    }
}

// ============================================================================
// Property Writer Traits
// ============================================================================

/// Base writer interface for properties.
pub trait PropertyWriter: Send {
    /// Get the property header.
    fn header(&self) -> &PropertyHeader;
}

/// Writer for scalar properties.
pub trait ScalarPropertyWriter: PropertyWriter {
    /// Write a sample from raw bytes.
    fn write_sample(&mut self, data: &[u8]) -> Result<()>;

    /// Write a sample from typed value.
    fn write_sample_typed<T: bytemuck::Pod>(&mut self, value: &T) -> Result<()>
    where
        Self: Sized,
    {
        self.write_sample(bytemuck::bytes_of(value))
    }
}

/// Writer for array properties.
pub trait ArrayPropertyWriter: PropertyWriter {
    /// Write a sample from raw bytes.
    fn write_sample(&mut self, data: &[u8]) -> Result<()>;

    /// Write a sample from typed slice.
    fn write_sample_typed<T: bytemuck::Pod>(&mut self, values: &[T]) -> Result<()>
    where
        Self: Sized,
    {
        self.write_sample(bytemuck::cast_slice(values))
    }
}

/// Writer for compound properties.
pub trait CompoundPropertyWriter: PropertyWriter {
    /// Add a scalar property.
    fn add_scalar(&mut self, header: PropertyHeader) -> Result<Box<dyn ScalarPropertyWriter + '_>>;

    /// Add an array property.
    fn add_array(&mut self, header: PropertyHeader) -> Result<Box<dyn ArrayPropertyWriter + '_>>;

    /// Add a compound property.
    fn add_compound(&mut self, header: PropertyHeader) -> Result<Box<dyn CompoundPropertyWriter + '_>>;

    /// Get an existing property by name.
    fn property_mut(&mut self, name: &str) -> Option<&mut dyn PropertyWriter>;
}

// ============================================================================
// Helper Wrapper
// ============================================================================

/// Wrapper to box a reference to ObjectReader.
struct ObjectReaderRef<'a>(&'a dyn ObjectReader);

impl<'a> ObjectReader for ObjectReaderRef<'a> {
    fn header(&self) -> &ObjectHeader {
        self.0.header()
    }

    fn parent(&self) -> Option<&dyn ObjectReader> {
        self.0.parent()
    }

    fn num_children(&self) -> usize {
        self.0.num_children()
    }

    fn child(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>> {
        self.0.child(index)
    }

    fn child_by_name(&self, name: &str) -> Option<Box<dyn ObjectReader + '_>> {
        self.0.child_by_name(name)
    }

    fn properties(&self) -> &dyn CompoundPropertyReader {
        self.0.properties()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_type_checks() {
        let header = PropertyHeader::scalar("test", crate::util::DataType::FLOAT32);
        assert!(header.is_scalar());
        assert!(!header.is_array());
        assert!(!header.is_compound());
    }
}
