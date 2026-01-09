//! High-level Alembic API.
//!
//! This module provides the main entry points for reading and writing Alembic files:
//! - [`IArchive`] / [`OArchive`] - Archive (file) access
//! - [`IObject`] / [`OObject`] - Hierarchical scene objects
//! - [`ICompoundProperty`] / [`OCompoundProperty`] - Property containers
//! - [`IScalarProperty`] / [`OScalarProperty`] - Single-value properties
//! - [`IArrayProperty`] / [`OArrayProperty`] - Array properties
//!
//! ## Example
//!
//! ```ignore
//! use alembic::abc::IArchive;
//!
//! let archive = IArchive::open("animation.abc")?;
//! println!("Root has {} children", archive.root().num_children());
//! ```

use std::path::Path;

use crate::core::{
    ArchiveReader, ObjectReader, CompoundPropertyReader, PropertyReader,
    ScalarPropertyReader, ArrayPropertyReader,
    ObjectHeader, PropertyHeader, TimeSampling, SampleSelector, MetaData,
};
use crate::ogawa::OgawaArchiveReader;
use crate::util::Result;

// ============================================================================
// Archives
// ============================================================================

/// Input archive for reading Alembic files.
///
/// This is the main entry point for reading .abc files.
pub struct IArchive {
    reader: Box<dyn ArchiveReader>,
}

impl IArchive {
    /// Open an Alembic file for reading.
    ///
    /// # Example
    /// ```ignore
    /// let archive = IArchive::open("scene.abc")?;
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = OgawaArchiveReader::open(path)?;
        Ok(Self { reader: Box::new(reader) })
    }

    /// Get the file name/path.
    pub fn name(&self) -> &str {
        self.reader.name()
    }

    /// Get the number of time samplings in the archive.
    pub fn num_time_samplings(&self) -> usize {
        self.reader.num_time_samplings()
    }

    /// Get a time sampling by index.
    pub fn time_sampling(&self, index: u32) -> Option<&TimeSampling> {
        self.reader.time_sampling(index as usize)
    }

    /// Get the root object of the archive.
    pub fn root(&self) -> IObject<'_> {
        IObject::new(self.reader.root())
    }
    
    /// Get the archive version.
    /// 
    /// Returns the Alembic library version this archive was written with.
    /// Format: AABBCC where AA=major, BB=minor, CC=patch (e.g., 10703 = 1.7.3)
    pub fn archive_version(&self) -> i32 {
        self.reader.archive_version()
    }
    
    /// Get the maximum number of samples for a given time sampling index.
    /// 
    /// Returns None if the index is invalid or the information isn't available
    /// (for archives created before version 1.1.3).
    pub fn max_num_samples_for_time_sampling(&self, index: u32) -> Option<usize> {
        self.reader.max_num_samples_for_time_sampling(index as usize)
    }
    
    /// Check if this archive is valid.
    /// 
    /// In Rust, this always returns true for a successfully constructed archive.
    /// Provided for API parity with the C++ Alembic library.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

/// Output archive for writing Alembic files.
pub struct OArchive {
    #[allow(dead_code)]
    inner: crate::ogawa::OArchive,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let inner = crate::ogawa::OArchive::create(path)?;
        Ok(Self { inner })
    }

    /// Get the root object of the archive.
    pub fn root(&mut self) -> OObject<'_> {
        OObject { _phantom: std::marker::PhantomData }
    }
    
    /// Check if this archive is valid.
    /// 
    /// In Rust, this always returns true for a successfully constructed archive.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

// ============================================================================
// Objects
// ============================================================================

/// Input object for reading scene hierarchy.
/// 
/// Can either borrow from parent (for root) or own the reader (for children).
pub struct IObject<'a> {
    reader: IObjectReader<'a>,
}

enum IObjectReader<'a> {
    Borrowed(&'a dyn ObjectReader),
    Owned(Box<dyn ObjectReader + 'a>),
}

impl<'a> IObjectReader<'a> {
    fn as_ref(&self) -> &dyn ObjectReader {
        match self {
            Self::Borrowed(r) => *r,
            Self::Owned(r) => r.as_ref(),
        }
    }
}

impl<'a> IObject<'a> {
    /// Create a new object wrapper (borrowed).
    fn new(reader: &'a dyn ObjectReader) -> Self {
        Self { reader: IObjectReader::Borrowed(reader) }
    }
    
    /// Create from owned reader.
    fn from_owned(reader: Box<dyn ObjectReader + 'a>) -> Self {
        Self { reader: IObjectReader::Owned(reader) }
    }

    /// Get the object header.
    pub fn header(&self) -> &ObjectHeader {
        self.reader.as_ref().header()
    }

    /// Get the name of this object.
    pub fn name(&self) -> &str {
        self.reader.as_ref().name()
    }

    /// Get the full path from root.
    pub fn full_name(&self) -> &str {
        self.reader.as_ref().full_name()
    }

    /// Get the number of child objects.
    pub fn num_children(&self) -> usize {
        self.reader.as_ref().num_children()
    }
    
    /// Get a child object by index.
    pub fn child(&self, index: usize) -> Option<IObject<'_>> {
        self.reader.as_ref().child(index).map(IObject::from_owned)
    }
    
    /// Get a child object by name.
    pub fn child_by_name(&self, name: &str) -> Option<IObject<'_>> {
        self.reader.as_ref().child_by_name(name).map(IObject::from_owned)
    }
    
    /// Iterate over all children.
    pub fn children(&self) -> impl Iterator<Item = IObject<'_>> + '_ {
        (0..self.num_children()).filter_map(|i| self.child(i))
    }

    /// Check if this object matches a schema.
    pub fn matches_schema(&self, schema: &str) -> bool {
        self.reader.as_ref().matches_schema(schema)
    }

    /// Get the properties compound.
    pub fn properties(&self) -> ICompoundProperty<'_> {
        ICompoundProperty::new(self.reader.as_ref().properties())
    }
    
    /// Get the metadata.
    pub fn meta_data(&self) -> &MetaData {
        self.reader.as_ref().meta_data()
    }
    
    /// Get the header of a child object by index without creating a full object.
    pub fn child_header(&self, index: usize) -> Option<&ObjectHeader> {
        self.reader.as_ref().child_header(index)
    }
    
    /// Get the header of a child object by name without creating a full object.
    pub fn child_header_by_name(&self, name: &str) -> Option<&ObjectHeader> {
        self.reader.as_ref().child_header_by_name(name)
    }
    
    // ========================================================================
    // Instance support
    // ========================================================================
    
    /// Check if this object is an instance root (directly instances another object).
    /// 
    /// An object can reference another object in the same archive and act as
    /// an instance. This method returns true if this object is such an instance.
    pub fn is_instance_root(&self) -> bool {
        self.reader.as_ref().is_instance_root()
    }
    
    /// Check if this object has been reached via an instance path.
    /// 
    /// This returns true if this object is either an instance itself or
    /// any of its ancestors is an instance.
    pub fn is_instance_descendant(&self) -> bool {
        self.reader.as_ref().is_instance_descendant()
    }
    
    /// Get the source path if this is an instance.
    /// 
    /// If this object is an instance (is_instance_root() returns true),
    /// this returns the path to the source object that is being instanced.
    /// Otherwise returns an empty string.
    pub fn instance_source_path(&self) -> &str {
        self.reader.as_ref().instance_source_path()
    }
    
    /// Check if the child at the given index is an instance.
    pub fn is_child_instance(&self, index: usize) -> bool {
        self.reader.as_ref().is_child_instance(index)
    }
    
    /// Check if the child with the given name is an instance.
    pub fn is_child_instance_by_name(&self, name: &str) -> bool {
        self.reader.as_ref().is_child_instance_by_name(name)
    }
    
    // ========================================================================
    // Hash support
    // ========================================================================
    
    /// Get the aggregated properties hash if available.
    /// 
    /// This returns a 16-byte digest that can be used to quickly
    /// compare if the properties have changed between two objects.
    pub fn properties_hash(&self) -> Option<[u8; 16]> {
        self.reader.as_ref().properties_hash()
    }
    
    /// Get the aggregated children hash if available.
    /// 
    /// This returns a 16-byte digest that can be used to quickly
    /// compare if the child hierarchy has changed.
    pub fn children_hash(&self) -> Option<[u8; 16]> {
        self.reader.as_ref().children_hash()
    }
    
    /// Check if this object is valid.
    /// 
    /// In Rust, this always returns true for a successfully constructed object.
    /// Provided for API parity with the C++ Alembic library.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

/// Output object for writing scene hierarchy.
pub struct OObject<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl OObject<'_> {
    /// Get the name of this object.
    pub fn name(&self) -> &str {
        ""
    }
    
    /// Check if this object is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

// ============================================================================
// Properties
// ============================================================================

/// Input compound property (container for other properties).
pub struct ICompoundProperty<'a> {
    reader: &'a dyn CompoundPropertyReader,
}

impl<'a> ICompoundProperty<'a> {
    fn new(reader: &'a dyn CompoundPropertyReader) -> Self {
        Self { reader }
    }

    /// Get the property header.
    pub fn header(&self) -> &PropertyHeader {
        self.reader.header()
    }

    /// Get the number of sub-properties.
    pub fn num_properties(&self) -> usize {
        self.reader.num_properties()
    }

    /// Check if a property exists.
    pub fn has_property(&self, name: &str) -> bool {
        self.reader.has_property(name)
    }

    /// Get property names.
    pub fn property_names(&self) -> Vec<String> {
        self.reader.property_names()
    }
    
    /// Get a property by index.
    pub fn property(&self, index: usize) -> Option<IProperty<'_>> {
        self.reader.property(index).map(IProperty::new)
    }
    
    /// Get a property by name.
    pub fn property_by_name(&self, name: &str) -> Option<IProperty<'_>> {
        self.reader.property_by_name(name).map(IProperty::new)
    }
    
    /// Check if this property is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

/// Output compound property.
pub struct OCompoundProperty<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

/// Generic input property - wraps scalar, array, or compound.
pub struct IProperty<'a> {
    reader: Box<dyn PropertyReader + 'a>,
}

impl<'a> IProperty<'a> {
    fn new(reader: Box<dyn PropertyReader + 'a>) -> Self {
        Self { reader }
    }
    
    /// Get the property header.
    pub fn header(&self) -> &PropertyHeader {
        self.reader.header()
    }
    
    /// Get the property name.
    pub fn name(&self) -> &str {
        self.reader.name()
    }
    
    /// Check if this is a scalar property.
    pub fn is_scalar(&self) -> bool {
        self.reader.is_scalar()
    }
    
    /// Check if this is an array property.
    pub fn is_array(&self) -> bool {
        self.reader.is_array()
    }
    
    /// Check if this is a compound property.
    pub fn is_compound(&self) -> bool {
        self.reader.is_compound()
    }
    
    /// Get as compound property reader.
    pub fn as_compound(&self) -> Option<ICompoundProperty<'_>> {
        self.reader.as_compound().map(ICompoundProperty::new)
    }
    
    /// Get as scalar property reader.
    pub fn as_scalar(&self) -> Option<&dyn ScalarPropertyReader> {
        self.reader.as_scalar()
    }
    
    /// Get as array property reader.
    pub fn as_array(&self) -> Option<&dyn ArrayPropertyReader> {
        self.reader.as_array()
    }
    
    /// Check if this property is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

/// Input scalar property (single value per sample).
pub struct IScalarProperty<'a> {
    reader: &'a dyn ScalarPropertyReader,
}

impl<'a> IScalarProperty<'a> {
    /// Get the property header.
    pub fn header(&self) -> &PropertyHeader {
        self.reader.header()
    }

    /// Get the number of samples.
    pub fn num_samples(&self) -> usize {
        self.reader.num_samples()
    }

    /// Check if this property is constant.
    pub fn is_constant(&self) -> bool {
        self.reader.is_constant()
    }

    /// Read a sample into the provided buffer.
    pub fn read_sample(&self, sel: impl Into<SampleSelector>, out: &mut [u8]) -> Result<()> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i,
            _ => 0, // TODO: Implement time-based selection
        };
        self.reader.read_sample(index, out)
    }
}

/// Output scalar property.
pub struct OScalarProperty<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

/// Input array property (array of values per sample).
pub struct IArrayProperty<'a> {
    reader: &'a dyn ArrayPropertyReader,
}

impl<'a> IArrayProperty<'a> {
    /// Get the property header.
    pub fn header(&self) -> &PropertyHeader {
        self.reader.header()
    }

    /// Get the number of samples.
    pub fn num_samples(&self) -> usize {
        self.reader.num_samples()
    }

    /// Check if this property is constant.
    pub fn is_constant(&self) -> bool {
        self.reader.is_constant()
    }

    /// Get the number of elements in a sample.
    pub fn sample_len(&self, sel: impl Into<SampleSelector>) -> Result<usize> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i,
            _ => 0,
        };
        self.reader.sample_len(index)
    }

    /// Read a sample as bytes.
    pub fn read_sample_vec(&self, sel: impl Into<SampleSelector>) -> Result<Vec<u8>> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i,
            _ => 0,
        };
        self.reader.read_sample_vec(index)
    }
}

/// Output array property.
pub struct OArrayProperty<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iarchive_error_on_missing() {
        let result = IArchive::open("nonexistent.abc");
        assert!(result.is_err());
    }
}
