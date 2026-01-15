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
//! println!("Root has {} children", archive.getTop().getNumChildren());
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
    pub fn getName(&self) -> &str {
        self.reader.name()
    }

    /// Get the number of time samplings in the archive.
    pub fn getNumTimeSamplings(&self) -> usize {
        self.reader.num_time_samplings()
    }

    /// Get a time sampling by index.
    pub fn getTimeSampling(&self, index: usize) -> Option<&TimeSampling> {
        self.reader.time_sampling(index)
    }

    /// Get the root object of the archive.
    pub fn getTop(&self) -> IObject<'_> {
        IObject::new(self.reader.root())
    }
    
    /// Find an object by its full path.
    /// 
    /// # Arguments
    /// * `path` - Full path like "/parent/child" or "parent/child"
    /// 
    /// Returns None if the object is not found.
    pub fn find_object(&self, path: &str) -> Option<IObject<'_>> {
        self.reader.find_object(path).map(IObject::from_owned)
    }
    
    /// Get the archive version.
    /// 
    /// Returns the Alembic library version this archive was written with.
    /// Format: AABBCC where AA=major, BB=minor, CC=patch (e.g., 10703 = 1.7.3)
    pub fn getArchiveVersion(&self) -> i32 {
        self.reader.archive_version()
    }
    
    /// Get the maximum number of samples for a given time sampling index.
    /// 
    /// Returns None if the index is invalid or the information isn't available
    /// (for archives created before version 1.1.3).
    pub fn max_num_samples_for_time_sampling(&self, index: usize) -> Option<usize> {
        self.reader.max_num_samples_for_time_sampling(index)
    }
    
    /// Check if this archive is valid.
    /// 
    /// In Rust, this always returns true for a successfully constructed archive.
    /// Provided for API parity with the C++ Alembic library.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
    
    /// Check if an object exists at the given path.
    /// 
    /// Path format: "/parent/child/grandchild"
    pub fn has_object(&self, path: &str) -> bool {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return true;
        }
        
        // Use a recursive function to check existence
        fn check_path<'a>(obj: IObject<'a>, parts: &[&str]) -> bool {
            if parts.is_empty() {
                return true;
            }
            if let Some(child) = obj.getChildByName(parts[0]) {
                check_path(child, &parts[1..])
            } else {
                false
            }
        }
        
        let parts: Vec<&str> = path.split('/').collect();
        check_path(self.getTop(), &parts)
    }
    
    /// Get the application name that created this archive.
    /// Returns None if not available.
    pub fn app_name(&self) -> Option<&str> {
        self.reader.archive_metadata().get("_ai_Application")
    }
    
    /// Get the date the archive was written.
    /// Returns None if not available.
    pub fn date_written(&self) -> Option<&str> {
        self.reader.archive_metadata().get("_ai_DateWritten")
    }
    
    /// Get the user description.
    /// Returns None if not available.
    pub fn user_description(&self) -> Option<&str> {
        self.reader.archive_metadata().get("_ai_Description")
    }
    
    /// Get the DCC FPS setting.
    /// Returns None if not available.
    pub fn dcc_fps(&self) -> Option<f64> {
        self.reader.archive_metadata().get("_ai_DCC_FPS")
            .and_then(|s: &str| s.parse().ok())
    }
    
    /// Get raw archive metadata.
    pub fn archive_metadata(&self) -> &MetaData {
        self.reader.archive_metadata()
    }
    
    /// Get the combined bounding box of all geometry in the archive.
    /// 
    /// Computes the union of self_bounds from all geometry objects
    /// at the given sample index. Returns None if no bounds are found.
    /// 
    /// # Arguments
    /// * `sample_index` - The sample index to query bounds at (0 for static)
    pub fn archive_bounds(&self, sample_index: usize) -> Option<crate::util::BBox3d> {
        let root = self.getTop();
        let mut combined = None;
        collect_bounds_recursive(&root, BoundsSelector::Index(sample_index), &mut combined);
        combined
    }
    
    /// Get the combined bounding box at time.
    /// 
    /// Computes bounds using time-based sampling interpolation.
    pub fn archive_bounds_at_time(&self, time: f64) -> Option<crate::util::BBox3d> {
        // For now, find nearest sample index from default time sampling
        let root = self.getTop();
        let mut combined = None;
        collect_bounds_recursive(&root, BoundsSelector::Time(time), &mut combined);
        combined
    }
}

/// Selector for bounds collection - either direct index or time-based.
#[derive(Clone, Copy)]
enum BoundsSelector {
    Index(usize),
    Time(f64),
}

impl BoundsSelector {
    /// Resolve to sample index given the number of samples.
    fn resolve(&self, num_samples: usize) -> usize {
        match self {
            Self::Index(i) => *i,
            Self::Time(t) => estimate_sample_index_at_time(*t, num_samples),
        }
    }
}

/// Estimate sample index from time.
/// Uses simple linear interpolation assuming 24fps if multiple samples exist.
/// For accurate results, use schema-specific time sampling queries.
fn estimate_sample_index_at_time(time: f64, num_samples: usize) -> usize {
    if time <= 0.0 || num_samples <= 1 {
        return 0;
    }
    // Assume 24fps as common animation rate
    let frame = (time * 24.0).floor() as usize;
    frame.min(num_samples - 1)
}

/// Recursively collect bounds from all geometry objects.
fn collect_bounds_recursive(obj: &IObject<'_>, sel: BoundsSelector, combined: &mut Option<crate::util::BBox3d>) {
    use crate::util::BBox3d;
    use crate::geom::*;
    
    // Helper to merge computed bounds
    fn merge_sample_bounds(sample_bounds: Option<BBox3d>, positions_bounds: (glam::Vec3, glam::Vec3), combined: &mut Option<BBox3d>) {
        if let Some(bounds) = sample_bounds {
            merge_bounds(combined, bounds);
        } else {
            let (min, max) = positions_bounds;
            merge_bounds(combined, BBox3d::new(
                glam::dvec3(min.x as f64, min.y as f64, min.z as f64),
                glam::dvec3(max.x as f64, max.y as f64, max.z as f64),
            ));
        }
    }
    
    // Check for geometry schemas and get their bounds
    if let Some(mesh) = IPolyMesh::new(obj) {
        let idx = sel.resolve(mesh.getNumSamples());
        if let Ok(sample) = mesh.get_sample(idx) {
            merge_sample_bounds(sample.self_bounds, sample.compute_bounds(), combined);
        }
    } else if let Some(subd) = ISubD::new(obj) {
        let idx = sel.resolve(subd.getNumSamples());
        if let Ok(sample) = subd.get_sample(idx) {
            merge_sample_bounds(sample.self_bounds, sample.compute_bounds(), combined);
        }
    } else if let Some(points) = IPoints::new(obj) {
        let idx = sel.resolve(points.getNumSamples());
        if let Ok(sample) = points.get_sample(idx) {
            merge_sample_bounds(sample.self_bounds, sample.compute_bounds(), combined);
        }
    } else if let Some(curves) = ICurves::new(obj) {
        let idx = sel.resolve(curves.getNumSamples());
        if let Ok(sample) = curves.get_sample(idx) {
            merge_sample_bounds(sample.self_bounds, sample.compute_bounds(), combined);
        }
    } else if let Some(nupatch) = INuPatch::new(obj) {
        let idx = sel.resolve(nupatch.getNumSamples());
        if let Ok(sample) = nupatch.get_sample(idx) {
            merge_sample_bounds(sample.self_bounds, sample.compute_bounds(), combined);
        }
    }
    
    // Recurse into children
    for i in 0..obj.getNumChildren() {
        if let Some(child) = obj.getChild(i) {
            collect_bounds_recursive(&child, sel, combined);
        }
    }
}

/// Merge a bounds into the combined bounds.
fn merge_bounds(combined: &mut Option<crate::util::BBox3d>, bounds: crate::util::BBox3d) {
    // Check that bounds are valid (min <= max)
    if bounds.min.x <= bounds.max.x && bounds.min.y <= bounds.max.y && bounds.min.z <= bounds.max.z {
        match combined {
            Some(c) => c.expand_by_box(&bounds),
            None => *combined = Some(bounds),
        }
    }
}

/// Output archive for writing Alembic files.
pub struct OArchive {
    inner: crate::ogawa::OArchive,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let inner = crate::ogawa::OArchive::create(path)?;
        Ok(Self { inner })
    }
    
    /// Get the archive name/path.
    pub fn getName(&self) -> &str {
        self.inner.name()
    }
    
    /// Add a time sampling and return its index.
    /// 
    /// If an equivalent time sampling already exists, returns its index.
    /// Index 0 is always identity (static) time sampling.
    pub fn add_time_sampling(&mut self, ts: crate::core::TimeSampling) -> u32 {
        self.inner.add_time_sampling(ts)
    }
    
    /// Get the number of time samplings.
    pub fn getNumTimeSamplings(&self) -> usize {
        self.inner.num_time_samplings()
    }
    
    /// Get a time sampling by index.
    pub fn time_sampling(&self, index: u32) -> Option<&crate::core::TimeSampling> {
        self.inner.time_sampling(index as usize)
    }
    
    /// Set compression hint (-1 = no compression, 0-9 = compression level).
    pub fn set_compression_hint(&mut self, hint: i32) {
        self.inner.set_compression_hint(hint);
    }
    
    /// Get compression hint.
    pub fn compression_hint(&self) -> i32 {
        self.inner.compression_hint()
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
    
    /// Close and finalize the archive.
    pub fn close(self) -> Result<()> {
        self.inner.close()
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
    pub fn getHeader(&self) -> &ObjectHeader {
        self.reader.as_ref().header()
    }

    /// Get the name of this object.
    pub fn getName(&self) -> &str {
        self.reader.as_ref().name()
    }

    /// Get the full path from root.
    pub fn getFullName(&self) -> &str {
        self.reader.as_ref().full_name()
    }
    
    /// Check if this is the root object.
    /// 
    /// Root objects have an empty name and path "/".
    pub fn isRoot(&self) -> bool {
        let name = self.getName();
        name.is_empty() || name == "ABC"
    }
    
    /// Get the parent object.
    /// 
    /// Returns None if this is the root object or if parent access is unavailable.
    /// 
    /// **Note:** Due to Rust's ownership model, this may return None even for
    /// non-root objects. Use [`getParentFullName()`](Self::getParentFullName) 
    /// combined with archive navigation as an alternative.
    pub fn getParent(&self) -> Option<IObject<'_>> {
        self.reader.as_ref().parent().map(|p| {
            // Wrap borrowed parent reader
            IObject { reader: IObjectReader::Borrowed(p) }
        })
    }
    
    /// Get the full path of the parent object.
    /// 
    /// Returns None if this is the root object.
    /// Use this path with archive traversal to access the parent.
    /// 
    /// Note: Due to Rust's ownership model, we cannot return a direct
    /// reference to the parent. Use `archive.root()` and navigate
    /// to the returned path to get the parent object.
    pub fn getParentFullName(&self) -> Option<String> {
        if self.isRoot() {
            return None;
        }
        let full = self.getFullName();
        // Find last '/' and return everything before it
        if let Some(pos) = full.rfind('/') {
            if pos == 0 {
                // Parent is root
                Some("/".to_string())
            } else {
                Some(full[..pos].to_string())
            }
        } else {
            Some("/".to_string())
        }
    }

    /// Get the number of child objects.
    pub fn getNumChildren(&self) -> usize {
        self.reader.as_ref().num_children()
    }
    
    /// Get a child object by index.
    pub fn getChild(&self, index: usize) -> Option<IObject<'_>> {
        self.reader.as_ref().child(index).map(IObject::from_owned)
    }
    
    /// Get a child object by name.
    pub fn getChildByName(&self, name: &str) -> Option<IObject<'_>> {
        self.reader.as_ref().child_by_name(name).map(IObject::from_owned)
    }
    
    /// Iterate over all children.
    pub fn getChildren(&self) -> impl Iterator<Item = IObject<'_>> + '_ {
        (0..self.getNumChildren()).filter_map(|i| self.getChild(i))
    }

    /// Check if this object matches a schema.
    pub fn matchesSchema(&self, schema: &str) -> bool {
        self.reader.as_ref().matches_schema(schema)
    }

    /// Get the properties compound.
    pub fn getProperties(&self) -> ICompoundProperty<'_> {
        ICompoundProperty::new(self.reader.as_ref().properties())
    }
    
    /// Get the metadata.
    pub fn getMetaData(&self) -> &MetaData {
        self.reader.as_ref().meta_data()
    }
    
    /// Get the header of a child object by index without creating a full object.
    pub fn getChildHeader(&self, index: usize) -> Option<&ObjectHeader> {
        self.reader.as_ref().child_header(index)
    }
    
    /// Get the header of a child object by name without creating a full object.
    pub fn getChildHeaderByName(&self, name: &str) -> Option<&ObjectHeader> {
        self.reader.as_ref().child_header_by_name(name)
    }
    
    // ========================================================================
    // Instance support
    // ========================================================================
    
    /// Check if this object is an instance root (directly instances another object).
    /// 
    /// An object can reference another object in the same archive and act as
    /// an instance. This method returns true if this object is such an instance.
    pub fn isInstanceRoot(&self) -> bool {
        self.reader.as_ref().is_instance_root()
    }
    
    /// Check if this object has been reached via an instance path.
    /// 
    /// This returns true if this object is either an instance itself or
    /// any of its ancestors is an instance.
    pub fn isInstanceDescendant(&self) -> bool {
        self.reader.as_ref().is_instance_descendant()
    }
    
    /// Get the source path if this is an instance.
    /// 
    /// If this object is an instance (is_instance_root() returns true),
    /// this returns the path to the source object that is being instanced.
    /// Otherwise returns an empty string.
    pub fn getInstanceSourcePath(&self) -> &str {
        self.reader.as_ref().instance_source_path()
    }
    
    /// Check if the child at the given index is an instance.
    pub fn isChildInstance(&self, index: usize) -> bool {
        self.reader.as_ref().is_child_instance(index)
    }
    
    /// Check if the child with the given name is an instance.
    pub fn isChildInstanceByName(&self, name: &str) -> bool {
        self.reader.as_ref().is_child_instance_by_name(name)
    }
    
    // ========================================================================
    // Hash support
    // ========================================================================
    
    /// Get the aggregated properties hash if available.
    /// 
    /// This returns a 16-byte digest that can be used to quickly
    /// compare if the properties have changed between two objects.
    pub fn getPropertiesHash(&self) -> Option<[u8; 16]> {
        self.reader.as_ref().properties_hash()
    }
    
    /// Get the aggregated children hash if available.
    /// 
    /// This returns a 16-byte digest that can be used to quickly
    /// compare if the child hierarchy has changed.
    pub fn getChildrenHash(&self) -> Option<[u8; 16]> {
        self.reader.as_ref().children_hash()
    }
    
    // Note: getArchive() is not implemented in Rust due to ownership constraints.
    // In C++ Alembic, IObject stores a pointer back to its archive, but Rust's
    // borrow checker prevents this pattern. Use the archive reference directly
    // when you need archive-level operations.
    //
    // Workaround: Pass the archive alongside objects when needed, or use
    // getFullName() to navigate from a known archive reference.
    
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
    pub fn getName(&self) -> &str {
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
    
    /// Get the underlying trait object reader.
    /// Useful for passing to utility functions.
    pub fn as_reader(&self) -> &dyn CompoundPropertyReader {
        self.reader
    }

    /// Get the property header.
    pub fn getHeader(&self) -> &PropertyHeader {
        self.reader.header()
    }

    /// Get the number of sub-properties.
    pub fn getNumProperties(&self) -> usize {
        self.reader.num_properties()
    }

    /// Check if a property exists.
    pub fn has_property(&self, name: &str) -> bool {
        self.reader.has_property(name)
    }
    
    /// Get property header by index.
    pub fn property_header(&self, index: usize) -> Option<PropertyHeader> {
        self.reader.property(index).map(|p| p.header().clone())
    }
    
    /// Get property header by name.
    pub fn property_header_by_name(&self, name: &str) -> Option<PropertyHeader> {
        self.reader.property_by_name(name).map(|p| p.header().clone())
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
    
    /// Check if a scalar property exists by name.
    pub fn has_scalar_property(&self, name: &str) -> bool {
        if let Some(prop) = self.property_by_name(name) {
            prop.is_scalar()
        } else {
            false
        }
    }
    
    /// Check if an array property exists by name.
    pub fn has_array_property(&self, name: &str) -> bool {
        if let Some(prop) = self.property_by_name(name) {
            prop.is_array()
        } else {
            false
        }
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
    pub fn getHeader(&self) -> &PropertyHeader {
        self.reader.header()
    }
    
    /// Get the property name.
    pub fn getName(&self) -> &str {
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
    
    /// Get the time sampling index for this property.
    /// Use this with IArchive::time_sampling() to get the actual TimeSampling.
    pub fn time_sampling_index(&self) -> u32 {
        self.reader.header().time_sampling_index
    }
}

/// Input scalar property (single value per sample).
pub struct IScalarProperty<'a> {
    reader: &'a dyn ScalarPropertyReader,
}

impl<'a> IScalarProperty<'a> {
    /// Get the property header.
    pub fn getHeader(&self) -> &PropertyHeader {
        self.reader.header()
    }

    /// Get the number of samples.
    pub fn getNumSamples(&self) -> usize {
        self.reader.num_samples()
    }

    /// Check if this property is constant.
    pub fn is_constant(&self) -> bool {
        self.reader.is_constant()
    }

    /// Read a sample into the provided buffer.
    /// 
    /// For time-based selectors, use `read_sample_with_ts()` which properly resolves time.
    /// This method defaults time-based selectors to sample 0.
    pub fn read_sample(&self, sel: impl Into<SampleSelector>, out: &mut [u8]) -> Result<()> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0, // Use read_sample_with_ts() for time-based selection
        };
        self.reader.read_sample(index, out)
    }
    
    /// Read a sample with proper time-based selection.
    /// 
    /// Pass the TimeSampling from `archive.time_sampling(prop.time_sampling_index())`.
    pub fn read_sample_with_ts(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling, out: &mut [u8]) -> Result<()> {
        let index = sel.into().get_index(ts, self.getNumSamples());
        self.reader.read_sample(index, out)
    }
    
    /// Get the time sampling index.
    pub fn time_sampling_index(&self) -> u32 {
        self.reader.header().time_sampling_index
    }
    
    /// Check if this property is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
}

/// Output scalar property.
pub struct OScalarProperty<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

// ============================================================================
// Typed Scalar Property
// ============================================================================

/// Typed input scalar property.
/// 
/// Provides type-safe access to scalar property values.
/// Use this when you know the exact type of the property data.
/// 
/// # Example
/// ```ignore
/// let prop: ITypedScalarProperty<f32> = ITypedScalarProperty::new(scalar_reader)?;
/// let value: f32 = prop.get_value(0)?;
/// ```
pub struct ITypedScalarProperty<'a, T> {
    reader: &'a dyn ScalarPropertyReader,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: bytemuck::Pod + Default> ITypedScalarProperty<'a, T> {
    /// Create from a scalar property reader.
    /// Returns None if the data type size doesn't match T.
    pub fn new(reader: &'a dyn ScalarPropertyReader) -> Option<Self> {
        let expected_size = std::mem::size_of::<T>();
        let actual_size = reader.header().data_type.num_bytes();
        if expected_size == actual_size {
            Some(Self { reader, _phantom: std::marker::PhantomData })
        } else {
            None
        }
    }
    
    /// Create from IScalarProperty.
    pub fn from_scalar(prop: &'a IScalarProperty<'a>) -> Option<Self> {
        Self::new(prop.reader)
    }
    
    /// Get the property header.
    pub fn getHeader(&self) -> &PropertyHeader {
        self.reader.header()
    }
    
    /// Get the number of samples.
    pub fn getNumSamples(&self) -> usize {
        self.reader.num_samples()
    }
    
    /// Check if this property is constant.
    pub fn is_constant(&self) -> bool {
        self.reader.is_constant()
    }
    
    /// Get a typed value at the given sample index.
    pub fn get_value(&self, index: usize) -> Result<T> {
        let mut value = T::default();
        let bytes = bytemuck::bytes_of_mut(&mut value);
        self.reader.read_sample(index, bytes)?;
        Ok(value)
    }
    
    /// Get a typed value using a sample selector.
    pub fn get(&self, sel: impl Into<SampleSelector>) -> Result<T> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0,
        };
        self.get_value(index)
    }
    
    /// Get a typed value with time-based selection.
    pub fn get_with_ts(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling) -> Result<T> {
        let index = sel.into().get_index(ts, self.getNumSamples());
        self.get_value(index)
    }
    
    /// Get the time sampling index.
    pub fn time_sampling_index(&self) -> u32 {
        self.reader.header().time_sampling_index
    }
    
    /// Check if valid.
    pub fn valid(&self) -> bool {
        true
    }
}

// Type aliases for common typed scalar properties

/// Bool scalar property.
pub type IBoolProperty<'a> = ITypedScalarProperty<'a, u8>;
/// Int8 scalar property.
pub type ICharProperty<'a> = ITypedScalarProperty<'a, i8>;
/// UInt8 scalar property.
pub type IUcharProperty<'a> = ITypedScalarProperty<'a, u8>;
/// Int16 scalar property.
pub type IInt16Property<'a> = ITypedScalarProperty<'a, i16>;
/// UInt16 scalar property.
pub type IUInt16Property<'a> = ITypedScalarProperty<'a, u16>;
/// Int32 scalar property.
pub type IInt32Property<'a> = ITypedScalarProperty<'a, i32>;
/// UInt32 scalar property.
pub type IUInt32Property<'a> = ITypedScalarProperty<'a, u32>;
/// Int64 scalar property.
pub type IInt64Property<'a> = ITypedScalarProperty<'a, i64>;
/// UInt64 scalar property.
pub type IUInt64Property<'a> = ITypedScalarProperty<'a, u64>;
/// Float scalar property.
pub type IFloatProperty<'a> = ITypedScalarProperty<'a, f32>;
/// Double scalar property.
pub type IDoubleProperty<'a> = ITypedScalarProperty<'a, f64>;
/// Vec2f scalar property.
pub type IV2fProperty<'a> = ITypedScalarProperty<'a, [f32; 2]>;
/// Vec3f scalar property.
pub type IV3fProperty<'a> = ITypedScalarProperty<'a, [f32; 3]>;
/// Vec2d scalar property.
pub type IV2dProperty<'a> = ITypedScalarProperty<'a, [f64; 2]>;
/// Vec3d scalar property.
pub type IV3dProperty<'a> = ITypedScalarProperty<'a, [f64; 3]>;
/// Box3f scalar property (min + max).
pub type IBox3fProperty<'a> = ITypedScalarProperty<'a, [f32; 6]>;
/// Box3d scalar property (min + max).
pub type IBox3dProperty<'a> = ITypedScalarProperty<'a, [f64; 6]>;
/// Mat33f scalar property.
pub type IM33fProperty<'a> = ITypedScalarProperty<'a, [[f32; 3]; 3]>;
/// Mat44f scalar property.
pub type IM44fProperty<'a> = ITypedScalarProperty<'a, [[f32; 4]; 4]>;
/// Mat33d scalar property.
pub type IM33dProperty<'a> = ITypedScalarProperty<'a, [[f64; 3]; 3]>;
/// Mat44d scalar property.
pub type IM44dProperty<'a> = ITypedScalarProperty<'a, [[f64; 4]; 4]>;

/// Input array property (array of values per sample).
pub struct IArrayProperty<'a> {
    reader: &'a dyn ArrayPropertyReader,
}

impl<'a> IArrayProperty<'a> {
    /// Get the property header.
    pub fn getHeader(&self) -> &PropertyHeader {
        self.reader.header()
    }

    /// Get the number of samples.
    pub fn getNumSamples(&self) -> usize {
        self.reader.num_samples()
    }

    /// Check if this property is constant.
    pub fn is_constant(&self) -> bool {
        self.reader.is_constant()
    }

    /// Get the number of elements in a sample.
    /// 
    /// For time-based selectors, use `sample_len_with_ts()`.
    pub fn sample_len(&self, sel: impl Into<SampleSelector>) -> Result<usize> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0,
        };
        self.reader.sample_len(index)
    }
    
    /// Get the number of elements in a sample with time-based selection.
    pub fn sample_len_with_ts(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling) -> Result<usize> {
        let index = sel.into().get_index(ts, self.getNumSamples());
        self.reader.sample_len(index)
    }

    /// Read a sample as bytes.
    /// 
    /// For time-based selectors, use `read_sample_vec_with_ts()`.
    pub fn read_sample_vec(&self, sel: impl Into<SampleSelector>) -> Result<Vec<u8>> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0,
        };
        self.reader.read_sample_vec(index)
    }
    
    /// Read a sample as bytes with time-based selection.
    /// 
    /// Pass the TimeSampling from `archive.time_sampling(prop.time_sampling_index())`.
    pub fn read_sample_vec_with_ts(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling) -> Result<Vec<u8>> {
        let index = sel.into().get_index(ts, self.getNumSamples());
        self.reader.read_sample_vec(index)
    }
    
    /// Get the time sampling index.
    pub fn time_sampling_index(&self) -> u32 {
        self.reader.header().time_sampling_index
    }
    
    /// Get the key (digest) of a sample for deduplication.
    /// 
    /// Returns the 16-byte MD5 digest stored with the sample.
    /// Samples with the same key contain identical data.
    pub fn get_key(&self, sel: impl Into<SampleSelector>) -> Result<crate::core::SampleDigest> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0,
        };
        self.reader.sample_key(index)
    }
    
    /// Get the key with time-based selection.
    pub fn get_key_with_ts(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling) -> Result<crate::core::SampleDigest> {
        let index = sel.into().get_index(ts, self.getNumSamples());
        self.reader.sample_key(index)
    }
    
    /// Get the dimensions of a sample.
    /// 
    /// Returns `[num_elements]` for 1D arrays, `[rows, cols]` for 2D, etc.
    pub fn get_dimensions(&self, sel: impl Into<SampleSelector>) -> Result<Vec<usize>> {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0,
        };
        self.reader.sample_dimensions(index)
    }
    
    /// Get dimensions with time-based selection.
    pub fn get_dimensions_with_ts(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling) -> Result<Vec<usize>> {
        let index = sel.into().get_index(ts, self.getNumSamples());
        self.reader.sample_dimensions(index)
    }
    
    /// Read sample and convert to a different POD type.
    /// 
    /// Useful when you need data in a different format than stored.
    /// Supports conversions between numeric types (int <-> float, etc.).
    /// 
    /// # Example
    /// ```ignore
    /// // Read float data as doubles
    /// let doubles: Vec<f64> = prop.get_as::<f32, f64>(0)?;
    /// ```
    pub fn get_as<Src, Dst>(&self, sel: impl Into<SampleSelector>) -> Result<Vec<Dst>>
    where
        Src: bytemuck::Pod + Copy,
        Dst: From<Src> + Clone,
    {
        let sel = sel.into();
        let index = match sel {
            SampleSelector::Index(i) => i.min(self.getNumSamples().saturating_sub(1)),
            _ => 0,
        };
        let data = self.reader.read_sample_vec(index)?;
        let src_slice: &[Src] = bytemuck::try_cast_slice(&data).map_err(|_| crate::util::Error::invalid("cast error"))?;
        Ok(src_slice.iter().map(|&v| Dst::from(v)).collect())
    }
    
    /// Read sample and convert with time-based selection.
    pub fn get_as_with_ts<Src, Dst>(&self, sel: impl Into<SampleSelector>, ts: &TimeSampling) -> Result<Vec<Dst>>
    where
        Src: bytemuck::Pod + Copy,
        Dst: From<Src> + Clone,
    {
        let index = sel.into().get_index(ts, self.getNumSamples());
        let data = self.reader.read_sample_vec(index)?;
        let src_slice: &[Src] = bytemuck::try_cast_slice(&data).map_err(|_| crate::util::Error::invalid("cast error"))?;
        Ok(src_slice.iter().map(|&v| Dst::from(v)).collect())
    }
    
    /// Check if this array property behaves like a scalar (single element per sample).
    pub fn is_scalar_like(&self) -> bool {
        // Check if all samples have exactly 1 element
        let num = self.getNumSamples();
        if num == 0 {
            return true;
        }
        for i in 0..num {
            if let Ok(len) = self.reader.sample_len(i) {
                if len != 1 {
                    return false;
                }
            }
        }
        true
    }
    
    /// Check if this property is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        true
    }
    
    /// Read sample as f32 array.
    pub fn read_f32_array(&self, index: usize) -> Result<Vec<f32>> {
        let data = self.reader.read_sample_vec(index)?;
        let slice: &[f32] = bytemuck::try_cast_slice(&data)
            .map_err(|_| crate::util::Error::invalid("cannot cast to f32"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as i32 array.
    pub fn read_i32_array(&self, index: usize) -> Result<Vec<i32>> {
        let data = self.reader.read_sample_vec(index)?;
        let slice: &[i32] = bytemuck::try_cast_slice(&data)
            .map_err(|_| crate::util::Error::invalid("cannot cast to i32"))?;
        Ok(slice.to_vec())
    }
    
    /// Read sample as string array.
    /// 
    /// Alembic stores string arrays as null-terminated strings concatenated together.
    pub fn read_string_array(&self, index: usize) -> Result<Vec<String>> {
        let data = self.reader.read_sample_vec(index)?;
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
                strings.push(s);
            }
        }
        
        Ok(strings)
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
