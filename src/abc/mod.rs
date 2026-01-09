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
    ObjectHeader, PropertyHeader, TimeSampling, SampleSelector,
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
