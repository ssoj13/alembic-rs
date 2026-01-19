//! Abstract traits for Alembic archives, objects, and properties.
//!
//! These traits define the interface between the low-level Ogawa layer
//! and the high-level Abc API.
//!
//! # API Naming Convention
//!
//! Method names follow the original Alembic C++ API naming convention (camelCase)
//! for full compatibility with the reference implementation. See:
//! - `_ref/alembic/lib/Alembic/AbcCoreAbstract/` for core abstract interfaces
//! - `_ref/alembic/lib/Alembic/Abc/` for high-level API

use crate::core::{ObjectHeader, PropertyHeader, TimeSampling, MetaData, SampleDigest};
use crate::util::Result;

// ============================================================================
// Archive Traits
// ============================================================================

/// Reader interface for an Alembic archive.
///
/// Corresponds to `Alembic::AbcCoreAbstract::ArchiveReader` in the reference.
pub trait ArchiveReader: Send + Sync {
    /// Get the archive name/path.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveReader::getName()`
    fn getName(&self) -> &str;

    /// Get the number of time samplings in the archive.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveReader::getNumTimeSamplings()`
    fn getNumTimeSamplings(&self) -> usize;

    /// Get a time sampling by index.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveReader::getTimeSampling()`
    fn getTimeSampling(&self, index: usize) -> Option<&TimeSampling>;

    /// Get the root object.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveReader::getTop()`
    fn getTop(&self) -> &dyn ObjectReader;
    
    /// Get the archive version.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveReader::getArchiveVersion()`
    fn getArchiveVersion(&self) -> i32 {
        0
    }
    
    /// Get the maximum number of samples for a given time sampling index.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveReader::getMaxNumSamplesForTimeSamplingIndex()`
    fn getMaxNumSamplesForTimeSamplingIndex(&self, _index: usize) -> Option<usize> {
        None
    }
    
    /// Get the archive-level metadata.
    ///
    /// Note: Extended method for archive metadata access.
    fn getArchiveMetaData(&self) -> &MetaData;
    
    /// Get the indexed metadata table (for binary-compatible copying).
    ///
    /// Returns the metadata entries used for indexed metadata serialization.
    fn getIndexedMetaData(&self) -> &[MetaData] {
        &[]
    }

    /// Find an object by full path.
    fn findObject(&self, path: &str) -> Option<Box<dyn ObjectReader + '_>> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Some(Box::new(ObjectReaderRef(self.getTop())));
        }
        if let Some(first_part) = parts.first() {
            self.getTop().getChild(first_part)
        } else {
            None
        }
    }
}

/// Writer interface for an Alembic archive.
///
/// Corresponds to `Alembic::AbcCoreAbstract::ArchiveWriter` in the reference.
pub trait ArchiveWriter: Send {
    /// Get the archive name/path.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveWriter::getName()`
    fn getName(&self) -> &str;

    /// Add a time sampling and return its index.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveWriter::addTimeSampling()`
    fn addTimeSampling(&mut self, ts: TimeSampling) -> u32;

    /// Get the number of time samplings.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveWriter::getNumTimeSamplings()`
    fn getNumTimeSamplings(&self) -> usize;

    /// Get a time sampling by index.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveWriter::getTimeSampling()`
    fn getTimeSampling(&self, index: usize) -> Option<&TimeSampling>;

    /// Get the root object for writing.
    ///
    /// Reference: `AbcCoreAbstract::ArchiveWriter::getTop()`
    fn getTop(&mut self) -> &mut dyn ObjectWriter;

    /// Flush and finalize the archive.
    fn close(self: Box<Self>) -> Result<()>;
}

// ============================================================================
// Object Traits
// ============================================================================

/// Reader interface for an object in the hierarchy.
///
/// Corresponds to `Alembic::AbcCoreAbstract::ObjectReader` in the reference.
pub trait ObjectReader: Send + Sync {
    /// Get the object header.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getHeader()`
    fn getHeader(&self) -> &ObjectHeader;

    /// Get the parent object (None for root).
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getParent()`
    ///
    /// **Note:** Current implementations return None due to Rust ownership
    /// constraints. Use `getFullName()` to get the object path and navigate
    /// via the archive if parent access is needed.
    fn getParent(&self) -> Option<&dyn ObjectReader>;

    /// Get the number of child objects.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getNumChildren()`
    fn getNumChildren(&self) -> usize;

    /// Get a child by index.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getChild(size_t)`
    fn getChildByIndex(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>>;

    /// Get a child by name.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getChild(const std::string&)`
    fn getChild(&self, name: &str) -> Option<Box<dyn ObjectReader + '_>>;

    /// Get the properties compound.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getProperties()`
    fn getProperties(&self) -> &dyn CompoundPropertyReader;

    /// Get object name (convenience).
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getName()`
    fn getName(&self) -> &str {
        &self.getHeader().name
    }

    /// Get full path (convenience).
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getFullName()`
    fn getFullName(&self) -> &str {
        &self.getHeader().full_name
    }

    /// Check if object matches a schema.
    fn matchesSchema(&self, schema: &str) -> bool {
        self.getHeader().meta_data.matches_schema(schema)
    }
    
    /// Get metadata (convenience).
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getMetaData()`
    fn getMetaData(&self) -> &MetaData {
        &self.getHeader().meta_data
    }
    
    /// Get header of child object at index.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getChildHeader(size_t)`
    fn getChildHeader(&self, _index: usize) -> Option<&ObjectHeader> {
        None
    }
    
    /// Get header of child object by name.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getChildHeader(const std::string&)`
    fn getChildHeaderByName(&self, _name: &str) -> Option<&ObjectHeader> {
        None
    }
    
    // ========================================================================
    // Instance support
    // ========================================================================
    
    /// Check if this object is an instance root.
    fn isInstanceRoot(&self) -> bool {
        false
    }
    
    /// Check if this object has been reached via an instance path.
    fn isInstanceDescendant(&self) -> bool {
        false
    }
    
    /// If this object is an instance, returns the source path.
    fn instanceSourcePath(&self) -> &str {
        ""
    }
    
    /// Check if child at index is an instance.
    fn isChildInstance(&self, _index: usize) -> bool {
        false
    }
    
    /// Check if child with given name is an instance.
    fn isChildInstanceByName(&self, _name: &str) -> bool {
        false
    }
    
    // ========================================================================
    // Hash support
    // ========================================================================
    
    /// Get aggregated properties hash if available.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getPropertiesHash()`
    fn getPropertiesHash(&self) -> Option<[u8; 16]> {
        None
    }
    
    /// Get aggregated children hash if available.
    ///
    /// Reference: `AbcCoreAbstract::ObjectReader::getChildrenHash()`
    fn getChildrenHash(&self) -> Option<[u8; 16]> {
        None
    }
}

/// Writer interface for an object.
///
/// Corresponds to `Alembic::AbcCoreAbstract::ObjectWriter` in the reference.
pub trait ObjectWriter: Send {
    /// Get the object header.
    ///
    /// Reference: `AbcCoreAbstract::ObjectWriter::getHeader()`
    fn getHeader(&self) -> &ObjectHeader;

    /// Add a child object.
    fn addChild(&mut self, header: ObjectHeader) -> Result<&mut dyn ObjectWriter>;

    /// Get the properties compound for writing.
    ///
    /// Reference: `AbcCoreAbstract::ObjectWriter::getProperties()`
    fn getProperties(&mut self) -> &mut dyn CompoundPropertyWriter;

    /// Get object name.
    ///
    /// Reference: `AbcCoreAbstract::ObjectWriter::getName()`
    fn getName(&self) -> &str {
        &self.getHeader().name
    }
}

// ============================================================================
// Property Traits
// ============================================================================

/// Base reader interface for any property.
///
/// Corresponds to `Alembic::AbcCoreAbstract::BasePropertyReader` in the reference.
pub trait PropertyReader: Send + Sync {
    /// Get the property header.
    ///
    /// Reference: `AbcCoreAbstract::BasePropertyReader::getHeader()`
    fn getHeader(&self) -> &PropertyHeader;

    /// Check if this is a scalar property.
    fn isScalar(&self) -> bool {
        self.getHeader().is_scalar()
    }

    /// Check if this is an array property.
    fn isArray(&self) -> bool {
        self.getHeader().is_array()
    }

    /// Check if this is a compound property.
    fn isCompound(&self) -> bool {
        self.getHeader().is_compound()
    }

    /// Get property name.
    ///
    /// Reference: `AbcCoreAbstract::BasePropertyReader::getName()`
    fn getName(&self) -> &str {
        &self.getHeader().name
    }

    /// Try to cast to scalar property reader.
    fn asScalar(&self) -> Option<&dyn ScalarPropertyReader> {
        None
    }

    /// Try to cast to array property reader.
    fn asArray(&self) -> Option<&dyn ArrayPropertyReader> {
        None
    }

    /// Try to cast to compound property reader.
    fn asCompound(&self) -> Option<&dyn CompoundPropertyReader> {
        None
    }
}

/// Reader for scalar properties (single value per sample).
///
/// Corresponds to `Alembic::AbcCoreAbstract::ScalarPropertyReader` in the reference.
pub trait ScalarPropertyReader: PropertyReader {
    /// Get the number of samples.
    ///
    /// Reference: `AbcCoreAbstract::ScalarPropertyReader::getNumSamples()`
    fn getNumSamples(&self) -> usize;

    /// Check if this property is constant (all samples identical).
    ///
    /// Reference: `AbcCoreAbstract::ScalarPropertyReader::isConstant()`
    fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }

    /// Read a sample into the provided buffer.
    ///
    /// Reference: `AbcCoreAbstract::ScalarPropertyReader::getSample()`
    fn getSample(&self, index: usize, out: &mut [u8]) -> Result<()>;

    /// Read sample as typed value.
    fn getSampleTyped<T: bytemuck::Pod + Default>(&self, index: usize) -> Result<T>
    where
        Self: Sized,
    {
        let mut value = T::default();
        let bytes = bytemuck::bytes_of_mut(&mut value);
        self.getSample(index, bytes)?;
        Ok(value)
    }
    
    /// Read sample into a Vec (for variable-length data like strings).
    fn getSampleVec(&self, index: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 4096];
        self.getSample(index, &mut buf)?;
        if let Some(end) = buf.iter().position(|&b| b == 0) {
            buf.truncate(end);
        }
        Ok(buf)
    }
    
    /// Get the key (digest) of a sample for deduplication/raw copy.
    ///
    /// This returns a 16-byte digest that can be used to preserve
    /// the exact sample when copying files.
    fn getKey(&self, index: usize) -> Result<SampleDigest>;
}

/// Reader for array properties (array of values per sample).
///
/// Corresponds to `Alembic::AbcCoreAbstract::ArrayPropertyReader` in the reference.
pub trait ArrayPropertyReader: PropertyReader {
    /// Get the number of samples.
    ///
    /// Reference: `AbcCoreAbstract::ArrayPropertyReader::getNumSamples()`
    fn getNumSamples(&self) -> usize;

    /// Check if this property is constant.
    ///
    /// Reference: `AbcCoreAbstract::ArrayPropertyReader::isConstant()`
    fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }

    /// Get the size (in elements) of a sample.
    fn getSampleLen(&self, index: usize) -> Result<usize>;

    /// Read a sample into the provided buffer.
    ///
    /// Reference: `AbcCoreAbstract::ArrayPropertyReader::getSample()`
    fn getSample(&self, index: usize, out: &mut [u8]) -> Result<usize>;

    /// Read sample as Vec of bytes.
    fn getSampleVec(&self, index: usize) -> Result<Vec<u8>>;
    
    /// Get the key (digest) of a sample for deduplication.
    ///
    /// Reference: `AbcCoreAbstract::ArrayPropertyReader::getKey()`
    fn getKey(&self, index: usize) -> Result<SampleDigest>;
    
    /// Get the dimensions of a sample.
    ///
    /// Reference: `AbcCoreAbstract::ArrayPropertyReader::getDimensions()`
    fn getDimensions(&self, index: usize) -> Result<Vec<usize>>;

    /// Read sample as typed Vec.
    fn getSampleTyped<T: bytemuck::Pod + Clone>(&self, index: usize) -> Result<Vec<T>>
    where
        Self: Sized,
    {
        let data = self.getSampleVec(index)?;
        let slice: &[T] = bytemuck::try_cast_slice(&data).map_err(|_| crate::util::Error::invalid("cast error"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as f32 array.
    fn getAsFloat32Array(&self, index: usize) -> Result<Vec<f32>> {
        let data = self.getSampleVec(index)?;
        let slice: &[f32] = bytemuck::try_cast_slice(&data)
            .map_err(|_| crate::util::Error::invalid("cannot cast to f32"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as i32 array.
    fn getAsInt32Array(&self, index: usize) -> Result<Vec<i32>> {
        let data = self.getSampleVec(index)?;
        let slice: &[i32] = bytemuck::try_cast_slice(&data)
            .map_err(|_| crate::util::Error::invalid("cannot cast to i32"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as string array.
    fn getAsStringArray(&self, index: usize) -> Result<Vec<String>> {
        let data = self.getSampleVec(index)?;
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
///
/// Corresponds to `Alembic::AbcCoreAbstract::CompoundPropertyReader` in the reference.
pub trait CompoundPropertyReader: PropertyReader {
    /// Get the number of sub-properties.
    ///
    /// Reference: `AbcCoreAbstract::CompoundPropertyReader::getNumProperties()`
    fn getNumProperties(&self) -> usize;

    /// Get a property by index.
    ///
    /// Reference: `AbcCoreAbstract::CompoundPropertyReader::getProperty(size_t)`
    fn getProperty(&self, index: usize) -> Option<Box<dyn PropertyReader + '_>>;

    /// Get a property by name.
    ///
    /// Reference: `AbcCoreAbstract::CompoundPropertyReader::getProperty(const std::string&)`
    fn getPropertyByName(&self, name: &str) -> Option<Box<dyn PropertyReader + '_>>;

    /// Check if a property exists.
    fn hasProperty(&self, name: &str) -> bool {
        self.getPropertyByName(name).is_some()
    }

    /// Get property names.
    fn getPropertyNames(&self) -> Vec<String> {
        (0..self.getNumProperties())
            .filter_map(|i| self.getProperty(i).map(|p| p.getName().to_string()))
            .collect()
    }
}

// ============================================================================
// Property Writer Traits
// ============================================================================

/// Base writer interface for properties.
///
/// Corresponds to `Alembic::AbcCoreAbstract::BasePropertyWriter` in the reference.
pub trait PropertyWriter: Send {
    /// Get the property header.
    ///
    /// Reference: `AbcCoreAbstract::BasePropertyWriter::getHeader()`
    fn getHeader(&self) -> &PropertyHeader;
}

/// Writer for scalar properties.
///
/// Corresponds to `Alembic::AbcCoreAbstract::ScalarPropertyWriter` in the reference.
pub trait ScalarPropertyWriter: PropertyWriter {
    /// Write a sample from raw bytes.
    ///
    /// Reference: `AbcCoreAbstract::ScalarPropertyWriter::setSample()`
    fn setSample(&mut self, data: &[u8]) -> Result<()>;

    /// Write a sample from typed value.
    fn setSampleTyped<T: bytemuck::Pod>(&mut self, value: &T) -> Result<()>
    where
        Self: Sized,
    {
        self.setSample(bytemuck::bytes_of(value))
    }
}

/// Writer for array properties.
///
/// Corresponds to `Alembic::AbcCoreAbstract::ArrayPropertyWriter` in the reference.
pub trait ArrayPropertyWriter: PropertyWriter {
    /// Write a sample from raw bytes.
    ///
    /// Reference: `AbcCoreAbstract::ArrayPropertyWriter::setSample()`
    fn setSample(&mut self, data: &[u8]) -> Result<()>;

    /// Write a sample from typed slice.
    fn setSampleTyped<T: bytemuck::Pod>(&mut self, values: &[T]) -> Result<()>
    where
        Self: Sized,
    {
        self.setSample(bytemuck::cast_slice(values))
    }
}

/// Writer for compound properties.
///
/// Corresponds to `Alembic::AbcCoreAbstract::CompoundPropertyWriter` in the reference.
pub trait CompoundPropertyWriter: PropertyWriter {
    /// Add a scalar property.
    fn addScalar(&mut self, header: PropertyHeader) -> Result<Box<dyn ScalarPropertyWriter + '_>>;

    /// Add an array property.
    fn addArray(&mut self, header: PropertyHeader) -> Result<Box<dyn ArrayPropertyWriter + '_>>;

    /// Add a compound property.
    fn addCompound(&mut self, header: PropertyHeader) -> Result<Box<dyn CompoundPropertyWriter + '_>>;

    /// Get an existing property by name.
    fn getProperty(&mut self, name: &str) -> Option<&mut dyn PropertyWriter>;
}

// ============================================================================
// Helper Wrapper
// ============================================================================

/// Wrapper to box a reference to ObjectReader.
struct ObjectReaderRef<'a>(&'a dyn ObjectReader);

impl<'a> ObjectReader for ObjectReaderRef<'a> {
    fn getHeader(&self) -> &ObjectHeader {
        self.0.getHeader()
    }

    fn getParent(&self) -> Option<&dyn ObjectReader> {
        self.0.getParent()
    }

    fn getNumChildren(&self) -> usize {
        self.0.getNumChildren()
    }

    fn getChildByIndex(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>> {
        self.0.getChildByIndex(index)
    }

    fn getChild(&self, name: &str) -> Option<Box<dyn ObjectReader + '_>> {
        self.0.getChild(name)
    }

    fn getProperties(&self) -> &dyn CompoundPropertyReader {
        self.0.getProperties()
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
