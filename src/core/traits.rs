//! Abstract traits for Alembic archives, objects, and properties.
//!
//! These traits define the interface between the low-level Ogawa layer
//! and the high-level Abc API.

use crate::core::{ObjectHeader, PropertyHeader, TimeSampling, MetaData, SampleDigest};
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
    
    /// Get the archive version.
    /// Returns the Alembic library version this archive was written with.
    fn archive_version(&self) -> i32 {
        0 // Default for older archives
    }
    
    /// Get the maximum number of samples for a given time sampling index.
    /// Returns None if the index is invalid or the info isn't available.
    fn max_num_samples_for_time_sampling(&self, _index: usize) -> Option<usize> {
        None // Default - not available
    }
    
    /// Get the archive-level metadata.
    /// This includes info like application name, writer library, etc.
    fn archive_metadata(&self) -> &MetaData;

    /// Find an object by full path.
    /// Note: Due to lifetime constraints, this only supports single-level paths.
    /// For nested paths, use recursive child lookups.
    fn find_object(&self, path: &str) -> Option<Box<dyn ObjectReader + '_>> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Some(Box::new(ObjectReaderRef(self.root())));
        }

        // Due to Rust lifetime constraints with trait objects, we can only
        // navigate one level at a time. Return the first child if exists.
        if let Some(first_part) = parts.first() {
            self.root().child_by_name(first_part)
        } else {
            None
        }
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
    /// 
    /// **Note:** Current implementations return None due to Rust ownership
    /// constraints. Use `full_name()` to get the object path and navigate
    /// via the archive if parent access is needed.
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
    
    /// Get metadata (convenience).
    fn meta_data(&self) -> &MetaData {
        &self.header().meta_data
    }
    
    /// Get header of child object at index without creating full reader.
    fn child_header(&self, _index: usize) -> Option<&ObjectHeader> {
        None  // Default: not implemented
    }
    
    /// Get header of child object by name without creating full reader.
    fn child_header_by_name(&self, _name: &str) -> Option<&ObjectHeader> {
        None  // Default: not implemented
    }
    
    // ========================================================================
    // Instance support
    // ========================================================================
    
    /// Check if this object is an instance root (directly instances another object).
    fn is_instance_root(&self) -> bool {
        false
    }
    
    /// Check if this object has been reached via an instance path.
    fn is_instance_descendant(&self) -> bool {
        false
    }
    
    /// If this object is an instance, returns the source path. Empty string otherwise.
    fn instance_source_path(&self) -> &str {
        ""
    }
    
    /// Check if child at index is an instance.
    fn is_child_instance(&self, _index: usize) -> bool {
        false
    }
    
    /// Check if child with given name is an instance.
    fn is_child_instance_by_name(&self, _name: &str) -> bool {
        false
    }
    
    // ========================================================================
    // Hash support
    // ========================================================================
    
    /// Get aggregated properties hash if available.
    /// Returns 16-byte digest.
    fn properties_hash(&self) -> Option<[u8; 16]> {
        None
    }
    
    /// Get aggregated children hash if available.
    /// Returns 16-byte digest.
    fn children_hash(&self) -> Option<[u8; 16]> {
        None
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
    
    /// Read sample into a Vec (for variable-length data like strings).
    /// Default implementation uses fixed 4KB buffer; override for larger data.
    fn read_sample_vec(&self, index: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 4096];
        self.read_sample(index, &mut buf)?;
        // Trim trailing zeros for string data
        if let Some(end) = buf.iter().position(|&b| b == 0) {
            buf.truncate(end);
        }
        Ok(buf)
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
    
    /// Get the key (digest) of a sample for deduplication.
    /// 
    /// Returns the 16-byte MD5 digest stored with the sample data.
    /// This can be used to detect duplicate samples without reading the full data.
    fn sample_key(&self, index: usize) -> Result<SampleDigest>;
    
    /// Get the dimensions of a sample.
    /// 
    /// For 1D arrays returns `[num_elements]`.
    /// For 2D arrays returns `[rows, cols]`, etc.
    fn sample_dimensions(&self, index: usize) -> Result<Vec<usize>>;

    /// Read sample as typed Vec.
    fn read_sample_typed<T: bytemuck::Pod + Clone>(&self, index: usize) -> Result<Vec<T>>
    where
        Self: Sized,
    {
        let data = self.read_sample_vec(index)?;
        let slice: &[T] = bytemuck::try_cast_slice(&data).map_err(|_| crate::util::Error::invalid("cast error"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as f32 array.
    fn read_f32_array(&self, index: usize) -> Result<Vec<f32>> {
        let data = self.read_sample_vec(index)?;
        let slice: &[f32] = bytemuck::try_cast_slice(&data)
            .map_err(|_| crate::util::Error::invalid("cannot cast to f32"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as i32 array.
    fn read_i32_array(&self, index: usize) -> Result<Vec<i32>> {
        let data = self.read_sample_vec(index)?;
        let slice: &[i32] = bytemuck::try_cast_slice(&data)
            .map_err(|_| crate::util::Error::invalid("cannot cast to i32"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as string array (null-terminated concatenated strings).
    fn read_string_array(&self, index: usize) -> Result<Vec<String>> {
        let data = self.read_sample_vec(index)?;
        let mut strings = Vec::new();
        let mut start = 0;
        for (i, &byte) in data.iter().enumerate() {
            if byte == 0 {
                if i > start {
                    if let Ok(s) = String::from_utf8(data[start..i].to_vec()) {
                        strings.push(s);
                    }
                }
                start = i + 1;
            }
        }
        // Handle last string if no trailing null
        if start < data.len() {
            if let Ok(s) = String::from_utf8(data[start..].to_vec()) {
                if !s.is_empty() {
                    strings.push(s);
                }
            }
        }
        Ok(strings)
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
