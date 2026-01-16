//! Alembic trait implementations backed by Ogawa format.
//!
//! This module bridges the low-level Ogawa reader/writer with the
//! abstract Core layer traits.

use std::path::Path;
use std::sync::Arc;

use super::{IArchive as OgawaIArchive, IGroup, IData};
use super::read_util::{
    read_time_samplings_and_max, read_indexed_metadata, read_object_headers,
    read_property_headers, ParsedObjectHeader, ParsedPropertyHeader, PropertyType,
    ALEMBIC_OGAWA_FILE_VERSION, MIN_ALEMBIC_VERSION,
};
use crate::core::{
    ArchiveReader, ObjectReader, CompoundPropertyReader, PropertyReader,
    ScalarPropertyReader, ArrayPropertyReader,
    ObjectHeader, PropertyHeader, MetaData, TimeSampling,
    ArraySampleKey, ReadArraySampleCache,
};
use crate::util::{Result, Error};

/// Size of the key/digest prefix in data blocks (16 bytes).
const DATA_KEY_SIZE: usize = 16;

// ============================================================================
// Archive Reader
// ============================================================================

/// Alembic archive reader backed by Ogawa format.
pub struct OgawaArchiveReader {
    name: String,
    #[allow(dead_code)]
    inner: Arc<OgawaIArchive>,
    archive_version: i32,
    time_samplings: Vec<TimeSampling>,
    max_samples: Vec<u32>,
    indexed_metadata: Arc<Vec<MetaData>>,
    root_data: Arc<ObjectData>,
    root_header: ObjectHeader,
    /// Array sample cache for read performance.
    #[allow(dead_code)]
    cache: Arc<ReadArraySampleCache>,
}

impl OgawaArchiveReader {
    /// Open an Alembic file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let name = path.to_string_lossy().to_string();
        let inner = Arc::new(OgawaIArchive::open(path)?);
        
        Self::init(name, inner)
    }
    
    fn init(name: String, inner: Arc<OgawaIArchive>) -> Result<Self> {
        // Create cache (64 MB default)
        let cache = Arc::new(ReadArraySampleCache::default());
        
        let group = inner.root();
        let num_children = group.num_children();
        
        // Validate archive structure
        // Child 0: version (data), Child 1: file version (data)
        // Child 2: root object (group), Child 3: metadata (data)
        // Child 4: time samplings (data), Child 5: indexed metadata (data)
        if num_children <= 5 {
            return Err(Error::invalid("Invalid Alembic file: not enough children"));
        }
        
        // Validate child types
        if !group.is_child_data(0)? || !group.is_child_data(1)? ||
           !group.is_child_group(2)? || !group.is_child_data(3)? ||
           !group.is_child_data(4)? || !group.is_child_data(5)? {
            return Err(Error::invalid("Invalid Alembic file structure"));
        }
        
        // Read version
        let version_data = group.data(0)?;
        if version_data.size() != 4 {
            return Err(Error::invalid("Invalid version data size"));
        }
        let version_bytes = version_data.read_all()?;
        let version = i32::from_le_bytes([
            version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3]
        ]);
        
        if !(0..=ALEMBIC_OGAWA_FILE_VERSION).contains(&version) {
            return Err(Error::invalid(format!("Unsupported file version: {}", version)));
        }
        
        // Read file version
        let file_version_data = group.data(1)?;
        if file_version_data.size() != 4 {
            return Err(Error::invalid("Invalid file version data size"));
        }
        let fv_bytes = file_version_data.read_all()?;
        let archive_version = i32::from_le_bytes([
            fv_bytes[0], fv_bytes[1], fv_bytes[2], fv_bytes[3]
        ]);
        
        if archive_version < MIN_ALEMBIC_VERSION {
            return Err(Error::invalid(format!("Unsupported Alembic version: {}", archive_version)));
        }
        
        // Read time samplings
        let time_data = group.data(4)?;
        let (time_samplings, max_samples) = read_time_samplings_and_max(&time_data)?;
        
        // Read indexed metadata
        let metadata_data = group.data(5)?;
        let indexed_metadata = Arc::new(read_indexed_metadata(&metadata_data)?);
        
        // Create root object data
        let root_group = group.group(2)?;
        let root_data = Arc::new(ObjectData::new(
            root_group,
            "",
            indexed_metadata.clone(),
            cache.clone(),
        )?);
        
        // Create root header
        let mut root_header = ObjectHeader::new("ABC", "/");
        
        // Read archive metadata
        let archive_meta_data = group.data(3)?;
        if archive_meta_data.size() > 0 {
            let meta_bytes = archive_meta_data.read_all()?;
            let meta_str = std::str::from_utf8(&meta_bytes)
                .map_err(|e| Error::other(format!("Invalid UTF-8 in archive metadata: {}", e)))?;
            root_header.meta_data = MetaData::parse(meta_str);
        }
        
        Ok(Self {
            name,
            inner,
            archive_version,
            time_samplings,
            max_samples,
            indexed_metadata,
            root_data,
            root_header,
            cache,
        })
    }
    
    /// Get the Alembic file version.
    pub fn getArchiveVersion(&self) -> i32 {
        self.archive_version
    }
    
    /// Get max samples for a time sampling index.
    pub fn max_samples_for_time_sampling(&self, index: usize) -> Option<u32> {
        self.max_samples.get(index).copied()
    }
    
    /// Get indexed metadata.
    pub fn indexed_metadata(&self) -> &[MetaData] {
        &self.indexed_metadata
    }
}

impl ArchiveReader for OgawaArchiveReader {
    fn getName(&self) -> &str {
        &self.name
    }
    
    fn getNumTimeSamplings(&self) -> usize {
        self.time_samplings.len()
    }
    
    fn getTimeSampling(&self, index: usize) -> Option<&TimeSampling> {
        self.time_samplings.get(index)
    }
    
    fn getTop(&self) -> &dyn ObjectReader {
        // Return a wrapper that provides ObjectReader trait
        // We need a static reference, so we'll use a different approach
        // For now, return self as a "root object"
        self
    }
    
    fn getArchiveVersion(&self) -> i32 {
        self.archive_version
    }
    
    fn getMaxNumSamplesForTimeSamplingIndex(&self, index: usize) -> Option<usize> {
        self.max_samples.get(index).map(|&v| v as usize)
    }
    
    fn getArchiveMetaData(&self) -> &MetaData {
        &self.root_header.meta_data
    }
    
    fn findObject(&self, path: &str) -> Option<Box<dyn ObjectReader + '_>> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            // Empty path - return root (self implements ObjectReader)
            // We can't return self directly due to lifetime, so navigate to return the children iter
            return None; // Root object is accessed via archive.root() directly
        }
        
        // Start from root_data and traverse the path using concrete types
        let mut current_data = self.root_data.clone();
        let mut current_header: Option<ObjectHeader>;
        
        for (i, part) in parts.iter().enumerate() {
            // Find child by name in current data
            let child_idx = current_data.children.iter().position(|h| h.name == *part)?;
            let parsed_header = &current_data.children[child_idx];
            
            // Build object header
            current_header = Some(ObjectHeader {
                name: parsed_header.name.clone(),
                full_name: parsed_header.full_name.clone(),
                meta_data: parsed_header.metadata.clone(),
            });
            
            // If not last part, need to descend into child
            if i < parts.len() - 1 {
                let group_index = (child_idx + 1) as u64;
                let child_group = current_data.group.group(group_index).ok()?;
                let cache = current_data.cache.clone();
                current_data = Arc::new(ObjectData::new(
                    child_group,
                    &parsed_header.full_name,
                    current_data.indexed_metadata.clone(),
                    cache,
                ).ok()?);
            } else {
                // Last part - create the reader
                let group_index = (child_idx + 1) as u64;
                let child_group = current_data.group.group(group_index).ok()?;
                let child_data = Arc::new(ObjectData::new(
                    child_group,
                    &parsed_header.full_name,
                    current_data.indexed_metadata.clone(),
                    current_data.cache.clone(),
                ).ok()?);
                
                return Some(Box::new(OgawaObjectReader {
                    header: current_header.unwrap(),
                    data: child_data,
                }));
            }
        }
        
        None
    }
}

// Implement ObjectReader for OgawaArchiveReader (as the root object)
impl ObjectReader for OgawaArchiveReader {
    fn getHeader(&self) -> &ObjectHeader {
        &self.root_header
    }
    
    fn getParent(&self) -> Option<&dyn ObjectReader> {
        None
    }
    
    fn getNumChildren(&self) -> usize {
        self.root_data.getNumChildren()
    }
    
    fn getChildByIndex(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>> {
        match self.root_data.child(index)? {
            Ok(reader) => Some(Box::new(reader)),
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("[alembic] Warning: failed to read child at index {}: {}", index, _e);
                None
            }
        }
    }
    
    fn getChild(&self, name: &str) -> Option<Box<dyn ObjectReader + '_>> {
        match self.root_data.child_by_name(name)? {
            Ok(reader) => Some(Box::new(reader)),
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("[alembic] Warning: failed to read child '{}': {}", name, _e);
                None
            }
        }
    }
    
    fn getProperties(&self) -> &dyn CompoundPropertyReader {
        self.root_data.properties()
    }
}

// ============================================================================
// Object Reader
// ============================================================================

/// Ogawa-backed object reader.
pub struct OgawaObjectReader {
    header: ObjectHeader,
    data: Arc<ObjectData>,
}

impl ObjectReader for OgawaObjectReader {
    fn getHeader(&self) -> &ObjectHeader {
        &self.header
    }
    
    fn getParent(&self) -> Option<&dyn ObjectReader> {
        // Parent tracking is not implemented due to Rust ownership constraints.
        // In a tree structure, returning &dyn ObjectReader to parent would require
        // either unsafe self-referential structs or Arc<Mutex> overhead.
        // 
        // Workaround: Use object paths (full_name()) to navigate the hierarchy,
        // or maintain your own parent references when traversing.
        None
    }
    
    fn getNumChildren(&self) -> usize {
        self.data.getNumChildren()
    }
    
    fn getChildByIndex(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>> {
        match self.data.child(index)? {
            Ok(reader) => Some(Box::new(reader)),
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("[alembic] Warning: failed to read child at index {}: {}", index, _e);
                None
            }
        }
    }
    
    fn getChild(&self, name: &str) -> Option<Box<dyn ObjectReader + '_>> {
        match self.data.child_by_name(name)? {
            Ok(reader) => Some(Box::new(reader)),
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("[alembic] Warning: failed to read child '{}': {}", name, _e);
                None
            }
        }
    }
    
    fn getProperties(&self) -> &dyn CompoundPropertyReader {
        self.data.properties()
    }
}

// ============================================================================
// Object Data (internal)
// ============================================================================

/// Internal object data container.
struct ObjectData {
    group: IGroup,
    children: Vec<ParsedObjectHeader>,
    properties: CompoundData,
    indexed_metadata: Arc<Vec<MetaData>>,
    cache: Arc<ReadArraySampleCache>,
}

impl ObjectData {
    fn new(
        group: IGroup,
        parent_name: &str,
        indexed_metadata: Arc<Vec<MetaData>>,
        cache: Arc<ReadArraySampleCache>,
    ) -> Result<Self> {
        let num_children = group.num_children();
        
        // Parse child headers from last data child
        let children = if num_children > 0 && group.is_child_data(num_children - 1)? {
            let headers_data = group.data(num_children - 1)?;
            read_object_headers(&headers_data, parent_name, &indexed_metadata)?
        } else {
            Vec::new()
        };
        
        // Parse properties from first child if it's a group
        let properties = if num_children > 0 && group.is_child_group(0)? {
            let props_group = group.group(0)?;
            CompoundData::from_group(props_group, &indexed_metadata, cache.clone())?
        } else {
            CompoundData::empty()
        };
        
        Ok(Self {
            group,
            children,
            properties,
            indexed_metadata,
            cache,
        })
    }
    
    fn getNumChildren(&self) -> usize {
        self.children.len()
    }
    
    fn properties(&self) -> &CompoundData {
        &self.properties
    }
    
    /// Get child object at index.
    /// Object structure: child 0 = properties, children 1..n-1 = child objects, child n-1 = headers
    fn child(&self, index: usize) -> Option<Result<OgawaObjectReader>> {
        if index >= self.children.len() {
            return None;
        }
        
        let header = &self.children[index];
        // Child objects start at group index 1 (index 0 is properties compound)
        let group_index = (index + 1) as u64;
        
        Some(self.create_child_reader(group_index, header))
    }
    
    fn child_by_name(&self, name: &str) -> Option<Result<OgawaObjectReader>> {
        let index = self.children.iter().position(|h| h.name == name)?;
        self.child(index)
    }
    
    fn create_child_reader(&self, group_index: u64, header: &ParsedObjectHeader) -> Result<OgawaObjectReader> {
        let child_group = self.group.group(group_index)?;
        let child_data = Arc::new(ObjectData::new(
            child_group,
            &header.full_name,
            self.indexed_metadata.clone(),
            self.cache.clone(),
        )?);
        
        let obj_header = ObjectHeader {
            name: header.name.clone(),
            full_name: header.full_name.clone(),
            meta_data: header.metadata.clone(),
        };
        
        Ok(OgawaObjectReader {
            header: obj_header,
            data: child_data,
        })
    }
}

// ============================================================================
// Compound Property Data
// ============================================================================

/// Compound property data container.
pub struct CompoundData {
    header: PropertyHeader,
    sub_properties: Vec<ParsedPropertyHeader>,
    group: Option<IGroup>,
    indexed_metadata: Arc<Vec<MetaData>>,
    cache: Arc<ReadArraySampleCache>,
}

impl CompoundData {
    fn empty() -> Self {
        Self {
            header: PropertyHeader::compound(".prop"),
            sub_properties: Vec::new(),
            group: None,
            indexed_metadata: Arc::new(Vec::new()),
            cache: Arc::new(ReadArraySampleCache::new(0)), // Empty cache for empty compound
        }
    }
    
    fn from_group(group: IGroup, indexed_metadata: &[MetaData], cache: Arc<ReadArraySampleCache>) -> Result<Self> {
        let num_children = group.num_children();
        
        // Property headers are in the last data child
        let sub_properties = if num_children > 0 && group.is_child_data(num_children - 1)? {
            let headers_data = group.data(num_children - 1)?;
            read_property_headers(&headers_data, indexed_metadata)?
        } else {
            Vec::new()
        };
        
        Ok(Self {
            header: PropertyHeader::compound(".prop"),
            sub_properties,
            group: Some(group),
            indexed_metadata: Arc::new(indexed_metadata.to_vec()),
            cache,
        })
    }
    
    /// Get the property child group at the given index.
    /// Properties are stored as children of the compound group, with headers at the end.
    fn property_group(&self, index: usize) -> Option<Result<IGroup>> {
        let group = self.group.as_ref()?;
        // Properties are at indices 0..n-1, headers are at index n-1
        // So property i is at group child index i
        if index >= self.sub_properties.len() {
            return None;
        }
        Some(group.group(index as u64))
    }
}

impl PropertyReader for CompoundData {
    fn getHeader(&self) -> &PropertyHeader {
        &self.header
    }
    
    fn asCompound(&self) -> Option<&dyn CompoundPropertyReader> {
        Some(self)
    }
}

impl CompoundPropertyReader for CompoundData {
    fn getNumProperties(&self) -> usize {
        self.sub_properties.len()
    }
    
    fn getProperty(&self, index: usize) -> Option<Box<dyn PropertyReader + '_>> {
        let parsed = self.sub_properties.get(index)?;
        
        // Get the property's group if available
        let prop_group = self.property_group(index).and_then(|r| r.ok());
        
        Some(Box::new(OgawaPropertyReader::new(
            parsed.clone(),
            prop_group,
            self.indexed_metadata.clone(),
            self.cache.clone(),
        )))
    }
    
    fn getPropertyByName(&self, name: &str) -> Option<Box<dyn PropertyReader + '_>> {
        let index = self.sub_properties.iter().position(|p| p.name == name)?;
        self.getProperty(index)
    }
}

// ============================================================================
// Property Readers
// ============================================================================

/// Full property reader with data access.
pub struct OgawaPropertyReader {
    header: PropertyHeader,
    parsed: ParsedPropertyHeader,
    group: Option<IGroup>,
    indexed_metadata: Arc<Vec<MetaData>>,
    /// Array sample cache for read performance.
    cache: Arc<ReadArraySampleCache>,
    /// Cached compound data (loaded on demand).
    compound_data: std::sync::OnceLock<Option<CompoundData>>,
}

impl OgawaPropertyReader {
    fn new(
        parsed: ParsedPropertyHeader,
        group: Option<IGroup>,
        indexed_metadata: Arc<Vec<MetaData>>,
        cache: Arc<ReadArraySampleCache>,
    ) -> Self {
        let header = match parsed.property_type {
            PropertyType::Compound => PropertyHeader::compound(&parsed.name),
            PropertyType::Scalar => PropertyHeader::scalar(
                &parsed.name,
                parsed.data_type,
            ),
            PropertyType::Array => PropertyHeader::array(
                &parsed.name,
                parsed.data_type,
            ),
        };
        
        Self { 
            header, 
            parsed, 
            group, 
            indexed_metadata,
            cache,
            compound_data: std::sync::OnceLock::new(),
        }
    }
    
    /// Get compound data, loading it on demand.
    fn get_compound_data(&self) -> Option<&CompoundData> {
        self.compound_data.get_or_init(|| {
            if self.parsed.property_type != PropertyType::Compound {
                return None;
            }
            let group = self.group.clone()?;
            CompoundData::from_group(group, &self.indexed_metadata, self.cache.clone()).ok()
        }).as_ref()
    }
    
    /// Number of samples.
    fn num_samples_internal(&self) -> usize {
        self.parsed.next_sample_index as usize
    }
    
    /// Check if property is constant (all samples same).
    fn is_constant_internal(&self) -> bool {
        self.parsed.first_changed_index == 0 && self.parsed.last_changed_index == 0
    }
    
    /// Read scalar sample data at index.
    fn read_scalar_sample(&self, index: usize, out: &mut [u8]) -> Result<()> {
        let group = self.group.as_ref()
            .ok_or_else(|| Error::invalid("No property group"))?;
        
        // For scalar properties, each sample is a data child
        // Data format: 16-byte key + actual data
        let data = group.data(index as u64)?;
        
        if data.size() < DATA_KEY_SIZE as u64 {
            // Empty data or key-only
            if out.is_empty() {
                return Ok(());
            }
            return Err(Error::invalid("Scalar sample data too small"));
        }
        
        let data_bytes = data.read_all()?;
        let actual_data = &data_bytes[DATA_KEY_SIZE..];
        
        let copy_len = out.len().min(actual_data.len());
        out[..copy_len].copy_from_slice(&actual_data[..copy_len]);
        
        Ok(())
    }
    
    /// Get array sample length (number of elements).
    fn array_sample_len(&self, index: usize) -> Result<usize> {
        let group = self.group.as_ref()
            .ok_or_else(|| Error::invalid("No property group"))?;
        
        // Array properties: index * 2 = data, index * 2 + 1 = dimensions
        let dims_index = (index * 2 + 1) as u64;
        
        if dims_index >= group.num_children() {
            return Err(Error::invalid("Array sample index out of range"));
        }
        
        let dims_data = group.data(dims_index)?;
        let dims = read_dimensions(&dims_data)?;
        
        // Total elements = product of dimensions
        Ok(dims.iter().product())
    }
    
    /// Read array sample data.
    fn read_array_sample(&self, index: usize) -> Result<Vec<u8>> {
        let group = self.group.as_ref()
            .ok_or_else(|| Error::invalid("No property group"))?;
        
        // Array properties: index * 2 = data, index * 2 + 1 = dimensions
        let data_index = (index * 2) as u64;
        
        if data_index >= group.num_children() {
            return Err(Error::invalid("Array sample index out of range"));
        }
        
        let data = group.data(data_index)?;
        
        if data.size() <= DATA_KEY_SIZE as u64 {
            // Empty array
            return Ok(Vec::new());
        }
        
        // Check cache first using file position as key
        let cache_key = ArraySampleKey::new(data.pos(), index);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok((*cached).clone());
        }
        
        // Cache miss - read from file
        let data_bytes = data.read_all()?;
        let result = data_bytes[DATA_KEY_SIZE..].to_vec();
        
        // Store in cache
        self.cache.insert(cache_key, result.clone());
        
        Ok(result)
    }
    
    /// Read array sample key (digest) without reading full data.
    fn read_array_sample_key(&self, index: usize) -> Result<[u8; 16]> {
        let group = self.group.as_ref()
            .ok_or_else(|| Error::invalid("No property group"))?;
        
        let data_index = (index * 2) as u64;
        
        if data_index >= group.num_children() {
            return Err(Error::invalid("Array sample index out of range"));
        }
        
        let data = group.data(data_index)?;
        
        if data.size() < DATA_KEY_SIZE as u64 {
            return Ok([0u8; 16]); // Empty data has zero key
        }
        
        // Read only the first 16 bytes (the key)
        let all_data = data.read_all()?;
        let mut key = [0u8; 16];
        key.copy_from_slice(&all_data[..16]);
        Ok(key)
    }
    
    /// Read array sample dimensions.
    fn read_array_sample_dimensions(&self, index: usize) -> Result<Vec<usize>> {
        let group = self.group.as_ref()
            .ok_or_else(|| Error::invalid("No property group"))?;
        
        // Array properties: index * 2 = data, index * 2 + 1 = dimensions
        let dims_index = (index * 2 + 1) as u64;
        
        if dims_index >= group.num_children() {
            return Err(Error::invalid("Array sample index out of range"));
        }
        
        let dims_data = group.data(dims_index)?;
        read_dimensions(&dims_data)
    }
}

impl PropertyReader for OgawaPropertyReader {
    fn getHeader(&self) -> &PropertyHeader {
        &self.header
    }
    
    fn asCompound(&self) -> Option<&dyn CompoundPropertyReader> {
        self.get_compound_data().map(|c| c as &dyn CompoundPropertyReader)
    }
    
    fn asScalar(&self) -> Option<&dyn ScalarPropertyReader> {
        if self.parsed.property_type == PropertyType::Scalar {
            Some(self)
        } else {
            None
        }
    }
    
    fn asArray(&self) -> Option<&dyn ArrayPropertyReader> {
        if self.parsed.property_type == PropertyType::Array {
            Some(self)
        } else {
            None
        }
    }
}

impl ScalarPropertyReader for OgawaPropertyReader {
    fn getNumSamples(&self) -> usize {
        self.num_samples_internal()
    }
    
    fn isConstant(&self) -> bool {
        self.is_constant_internal()
    }
    
    fn getSample(&self, index: usize, out: &mut [u8]) -> Result<()> {
        // Handle constant optimization
        let actual_index = if self.is_constant_internal() && index > 0 {
            0
        } else {
            index.min(self.num_samples_internal().saturating_sub(1))
        };
        
        self.read_scalar_sample(actual_index, out)
    }
}

impl ArrayPropertyReader for OgawaPropertyReader {
    fn getNumSamples(&self) -> usize {
        self.num_samples_internal()
    }
    
    fn isConstant(&self) -> bool {
        self.is_constant_internal()
    }
    
    fn getSampleLen(&self, index: usize) -> Result<usize> {
        let actual_index = if self.is_constant_internal() && index > 0 {
            0
        } else {
            index.min(self.num_samples_internal().saturating_sub(1))
        };
        
        self.array_sample_len(actual_index)
    }
    
    fn getSample(&self, index: usize, out: &mut [u8]) -> Result<usize> {
        let actual_index = if self.is_constant_internal() && index > 0 {
            0
        } else {
            index.min(self.num_samples_internal().saturating_sub(1))
        };
        
        let data = self.read_array_sample(actual_index)?;
        let copy_len = out.len().min(data.len());
        out[..copy_len].copy_from_slice(&data[..copy_len]);
        Ok(copy_len)
    }
    
    fn getSampleVec(&self, index: usize) -> Result<Vec<u8>> {
        let actual_index = if self.is_constant_internal() && index > 0 {
            0
        } else {
            index.min(self.num_samples_internal().saturating_sub(1))
        };
        
        self.read_array_sample(actual_index)
    }
    
    fn getKey(&self, index: usize) -> Result<[u8; 16]> {
        let actual_index = if self.is_constant_internal() && index > 0 {
            0
        } else {
            index.min(self.num_samples_internal().saturating_sub(1))
        };
        
        self.read_array_sample_key(actual_index)
    }
    
    fn getDimensions(&self, index: usize) -> Result<Vec<usize>> {
        let actual_index = if self.is_constant_internal() && index > 0 {
            0
        } else {
            index.min(self.num_samples_internal().saturating_sub(1))
        };
        
        self.read_array_sample_dimensions(actual_index)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Read dimensions from a dimensions data block.
fn read_dimensions(data: &IData) -> Result<Vec<usize>> {
    if data.is_empty() {
        return Ok(vec![0]);
    }
    
    let bytes = data.read_all()?;
    
    // Dimensions are stored as u64 values
    if bytes.len() % 8 != 0 {
        return Err(Error::invalid("Invalid dimensions data size"));
    }
    
    let mut dims = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        let dim = u64::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3],
                                       chunk[4], chunk[5], chunk[6], chunk[7]]);
        dims.push(dim as usize);
    }
    
    Ok(dims)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compound_data_empty() {
        let data = CompoundData::empty();
        assert_eq!(data.getNumProperties(), 0);
        assert!(data.isCompound());
    }
}
