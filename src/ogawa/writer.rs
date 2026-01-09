//! Ogawa format writer implementation.
//!
//! Provides complete support for writing Alembic files in Ogawa format.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};

use super::format::*;
use crate::core::{MetaData, TimeSampling, TimeSamplingType, ArraySampleContentKey};
use crate::util::{DataType, PlainOldDataType, Error, Result};

/// Output stream for writing Ogawa data.
pub struct OStream {
    writer: BufWriter<File>,
    pos: u64,
}

impl OStream {
    /// Create a new output stream for the given file path.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(Self {
            writer: BufWriter::with_capacity(2 * 1024 * 1024, file), // 2MB buffer
            pos: 0,
        })
    }

    /// Get the current write position.
    #[inline]
    pub fn pos(&self) -> u64 {
        self.pos
    }

    /// Write bytes and advance position.
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.pos += data.len() as u64;
        Ok(())
    }

    /// Write a u64 value (little-endian).
    pub fn write_u64(&mut self, value: u64) -> Result<()> {
        self.writer.write_u64::<LittleEndian>(value)?;
        self.pos += 8;
        Ok(())
    }

    /// Write a u32 value (little-endian).
    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        self.writer.write_u32::<LittleEndian>(value)?;
        self.pos += 4;
        Ok(())
    }

    /// Write a u16 value (little-endian).
    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        self.writer.write_u16::<LittleEndian>(value)?;
        self.pos += 2;
        Ok(())
    }

    /// Write a u8 value.
    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.writer.write_u8(value)?;
        self.pos += 1;
        Ok(())
    }
    
    /// Write an i32 value (little-endian).
    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.writer.write_i32::<LittleEndian>(value)?;
        self.pos += 4;
        Ok(())
    }
    
    /// Write an f64 value (little-endian).
    pub fn write_f64(&mut self, value: f64) -> Result<()> {
        self.writer.write_f64::<LittleEndian>(value)?;
        self.pos += 8;
        Ok(())
    }

    /// Seek to a position and return the current position.
    pub fn seek(&mut self, pos: u64) -> Result<u64> {
        self.writer.flush()?;
        let new_pos = self.writer.seek(SeekFrom::Start(pos))?;
        self.pos = new_pos;
        Ok(new_pos)
    }

    /// Seek to end and return the position.
    pub fn seek_end(&mut self) -> Result<u64> {
        self.writer.flush()?;
        let new_pos = self.writer.seek(SeekFrom::End(0))?;
        self.pos = new_pos;
        Ok(new_pos)
    }

    /// Flush the buffer to disk.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Library version for written archives (1.7.9).
const ALEMBIC_LIBRARY_VERSION: i32 = 10709;

/// Ogawa file version.
const OGAWA_FILE_VERSION: i32 = 1;

/// Acyclic time per cycle marker.
const ACYCLIC_TIME_PER_CYCLE: f64 = -f64::MAX;

/// Size of digest/key prefix in data blocks.
const DATA_KEY_SIZE: usize = 16;

// ============================================================================
// OArchive - Main Archive Writer
// ============================================================================

/// Ogawa archive writer.
pub struct OArchive {
    name: String,
    stream: OStream,
    frozen: bool,
    time_samplings: Vec<TimeSampling>,
    max_samples: Vec<u32>,
    indexed_metadata: Vec<MetaData>,
    metadata_map: HashMap<String, usize>,
    archive_metadata: MetaData,
    application_writer: String,
    compression_hint: i32,
    /// Deduplication map: content key -> file position
    dedup_map: HashMap<ArraySampleContentKey, u64>,
    /// Enable/disable deduplication (enabled by default)
    dedup_enabled: bool,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let name = path.as_ref().to_string_lossy().to_string();
        let mut stream = OStream::create(&path)?;

        // Write header with placeholder for root position
        stream.write_bytes(OGAWA_MAGIC)?;
        stream.write_u8(NOT_FROZEN_FLAG)?;
        stream.write_u16(CURRENT_VERSION)?;
        stream.write_u64(0)?; // Root position placeholder

        // Default identity time sampling at index 0
        let identity_ts = TimeSampling::identity();
        
        Ok(Self {
            name,
            stream,
            frozen: false,
            time_samplings: vec![identity_ts],
            max_samples: vec![0],
            indexed_metadata: vec![MetaData::new()], // Index 0 is always empty
            metadata_map: HashMap::new(),
            archive_metadata: MetaData::new(),
            application_writer: String::from("alembic-rs"),
            compression_hint: -1,
            dedup_map: HashMap::new(),
            dedup_enabled: true,
        })
    }
    
    /// Enable or disable deduplication.
    /// Deduplication saves space by storing identical data only once.
    pub fn set_dedup_enabled(&mut self, enabled: bool) {
        self.dedup_enabled = enabled;
    }
    
    /// Check if deduplication is enabled.
    pub fn dedup_enabled(&self) -> bool {
        self.dedup_enabled
    }
    
    /// Get number of deduplicated samples.
    pub fn dedup_count(&self) -> usize {
        self.dedup_map.len()
    }
    
    /// Get the archive name/path.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Set the application writer string.
    pub fn set_application_writer(&mut self, writer: &str) {
        self.application_writer = writer.to_string();
    }
    
    /// Get the application writer string.
    pub fn application_writer(&self) -> &str {
        &self.application_writer
    }
    
    /// Set compression hint (-1 = no compression, 0-9 = compression level).
    pub fn set_compression_hint(&mut self, hint: i32) {
        self.compression_hint = hint.clamp(-1, 9);
    }
    
    /// Get compression hint.
    pub fn compression_hint(&self) -> i32 {
        self.compression_hint
    }
    
    /// Set archive metadata.
    pub fn set_archive_metadata(&mut self, md: MetaData) {
        self.archive_metadata = md;
    }
    
    /// Add a time sampling and return its index.
    pub fn add_time_sampling(&mut self, ts: TimeSampling) -> u32 {
        // Check if this time sampling already exists
        for (i, existing) in self.time_samplings.iter().enumerate() {
            if existing.is_equivalent(&ts) {
                return i as u32;
            }
        }
        // Add new time sampling
        let index = self.time_samplings.len() as u32;
        self.time_samplings.push(ts);
        self.max_samples.push(0);
        index
    }
    
    /// Update max samples for a time sampling.
    pub fn update_max_samples(&mut self, ts_index: u32, num_samples: u32) {
        if let Some(max) = self.max_samples.get_mut(ts_index as usize) {
            *max = (*max).max(num_samples);
        }
    }
    
    /// Get the number of time samplings.
    pub fn num_time_samplings(&self) -> usize {
        self.time_samplings.len()
    }
    
    /// Get a time sampling by index.
    pub fn time_sampling(&self, index: usize) -> Option<&TimeSampling> {
        self.time_samplings.get(index)
    }
    
    /// Add or get indexed metadata, returns index.
    pub fn add_indexed_metadata(&mut self, md: &MetaData) -> u8 {
        let serialized = md.serialize();
        
        // Check if empty
        if serialized.is_empty() {
            return 0;
        }
        
        // Check if already indexed
        if let Some(&idx) = self.metadata_map.get(&serialized) {
            return idx as u8;
        }
        
        // Check if fits in indexed metadata (max 254 entries, max 255 bytes each)
        if self.indexed_metadata.len() >= 254 || serialized.len() > 255 {
            return 0xff; // Use inline metadata
        }
        
        let idx = self.indexed_metadata.len();
        self.indexed_metadata.push(md.clone());
        self.metadata_map.insert(serialized, idx);
        idx as u8
    }

    /// Check if the archive has been frozen (finalized).
    #[inline]
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Get access to the underlying stream.
    #[inline]
    pub fn stream(&mut self) -> &mut OStream {
        &mut self.stream
    }

    /// Write raw data block and return its position.
    pub fn write_data(&mut self, data: &[u8]) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        if data.is_empty() {
            return Ok(0); // Empty data marker
        }

        let pos = self.stream.pos();
        self.stream.write_u64(data.len() as u64)?;
        self.stream.write_bytes(data)?;
        Ok(pos)
    }
    
    /// Write data with 16-byte key prefix and deduplication.
    /// 
    /// If identical data was already written, returns the existing position.
    /// Otherwise writes the data and returns the new position.
    pub fn write_keyed_data(&mut self, data: &[u8]) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }
        
        // Compute content key for deduplication
        let content_key = ArraySampleContentKey::from_data(data);
        
        // Check for duplicate if deduplication is enabled
        if self.dedup_enabled {
            if let Some(&existing_pos) = self.dedup_map.get(&content_key) {
                return Ok(existing_pos);
            }
        }
        
        let pos = self.stream.pos();
        // Total size = 16 (key) + data.len()
        let total_size = DATA_KEY_SIZE + data.len();
        self.stream.write_u64(total_size as u64)?;
        
        // Write the MD5 digest as key
        self.stream.write_bytes(content_key.digest())?;
        self.stream.write_bytes(data)?;
        
        // Store in dedup map
        if self.dedup_enabled {
            self.dedup_map.insert(content_key, pos);
        }
        
        Ok(pos)
    }
    
    /// Write data with specific key (for known digest).
    pub fn write_keyed_data_with_key(&mut self, data: &[u8], key: &[u8; 16]) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }
        
        let pos = self.stream.pos();
        let total_size = DATA_KEY_SIZE + data.len();
        self.stream.write_u64(total_size as u64)?;
        self.stream.write_bytes(key)?;
        self.stream.write_bytes(data)?;
        Ok(pos)
    }

    /// Write a group and return its position.
    pub fn write_group(&mut self, children: &[u64]) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        if children.is_empty() {
            return Ok(0); // Empty group marker
        }

        let pos = self.stream.pos();
        self.stream.write_u64(children.len() as u64)?;
        for &child in children {
            self.stream.write_u64(child)?;
        }
        Ok(pos)
    }
    
    /// Serialize time samplings to bytes.
    fn serialize_time_samplings(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        
        for (i, ts) in self.time_samplings.iter().enumerate() {
            let max_sample = self.max_samples.get(i).copied().unwrap_or(0);
            buf.extend_from_slice(&max_sample.to_le_bytes());
            
            let (tpc, samples): (f64, Vec<f64>) = match &ts.sampling_type {
                TimeSamplingType::Identity => {
                    (1.0, vec![0.0])
                }
                TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                    (*time_per_cycle, vec![*start_time])
                }
                TimeSamplingType::Cyclic { time_per_cycle, times } => {
                    (*time_per_cycle, times.clone())
                }
                TimeSamplingType::Acyclic { times } => {
                    (ACYCLIC_TIME_PER_CYCLE, times.clone())
                }
            };
            
            buf.extend_from_slice(&tpc.to_le_bytes());
            buf.extend_from_slice(&(samples.len() as u32).to_le_bytes());
            for sample in samples {
                buf.extend_from_slice(&sample.to_le_bytes());
            }
        }
        
        buf
    }
    
    /// Serialize indexed metadata to bytes.
    fn serialize_indexed_metadata(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        
        // Skip index 0 (always empty)
        for md in self.indexed_metadata.iter().skip(1) {
            let serialized = md.serialize();
            buf.push(serialized.len() as u8);
            buf.extend_from_slice(serialized.as_bytes());
        }
        
        buf
    }
    
    /// Write the complete archive with given root object.
    pub fn write_archive(&mut self, root: &OObject) -> Result<()> {
        if self.frozen {
            return Err(Error::Frozen);
        }
        
        // Write all objects recursively, collect positions
        let root_obj_pos = self.write_object(root, "/")?;
        
        // Serialize time samplings
        let ts_data = self.serialize_time_samplings();
        let ts_pos = self.write_data(&ts_data)?;
        
        // Serialize indexed metadata
        let idx_meta_data = self.serialize_indexed_metadata();
        let idx_meta_pos = if idx_meta_data.is_empty() {
            0
        } else {
            self.write_data(&idx_meta_data)?
        };
        
        // Write archive metadata
        let archive_meta_str = self.archive_metadata.serialize();
        let archive_meta_pos = if archive_meta_str.is_empty() {
            0
        } else {
            self.write_data(archive_meta_str.as_bytes())?
        };
        
        // Write version data
        let version_pos = self.write_data(&OGAWA_FILE_VERSION.to_le_bytes())?;
        
        // Write file version data
        let file_version_pos = self.write_data(&ALEMBIC_LIBRARY_VERSION.to_le_bytes())?;
        
        // Build root group:
        // Child 0: version (data)
        // Child 1: file version (data)
        // Child 2: root object (group)
        // Child 3: archive metadata (data)
        // Child 4: time samplings (data)
        // Child 5: indexed metadata (data)
        let root_children = vec![
            make_data_offset(version_pos),
            make_data_offset(file_version_pos),
            make_group_offset(root_obj_pos),
            make_data_offset(archive_meta_pos),
            make_data_offset(ts_pos),
            make_data_offset(idx_meta_pos),
        ];
        
        let root_pos = self.write_group(&root_children)?;
        
        // Finalize
        self.frozen = true;
        
        // Update header
        self.stream.seek(FROZEN_OFFSET as u64)?;
        self.stream.write_u8(FROZEN_FLAG)?;
        self.stream.seek(ROOT_POS_OFFSET as u64)?;
        self.stream.write_u64(root_pos)?;
        
        self.stream.seek_end()?;
        self.stream.flush()?;
        
        Ok(())
    }
    
    /// Write an object and return its group position.
    fn write_object(&mut self, obj: &OObject, parent_path: &str) -> Result<u64> {
        let full_path = if parent_path == "/" {
            format!("/{}", obj.name)
        } else {
            format!("{}/{}", parent_path, obj.name)
        };
        
        // Write properties compound
        let props_pos = self.write_properties(&obj.properties)?;
        
        // Write child objects
        let mut child_positions = Vec::new();
        for child in &obj.children {
            let child_pos = self.write_object(child, &full_path)?;
            child_positions.push(child_pos);
        }
        
        // Write object headers (for children)
        let headers_data = self.serialize_object_headers(&obj.children, &full_path);
        let headers_pos = if headers_data.is_empty() {
            0
        } else {
            self.write_data(&headers_data)?
        };
        
        // Build object group:
        // Child 0: properties compound (group)
        // Children 1..n-1: child objects (groups)
        // Child n-1: object headers (data)
        let mut children = Vec::new();
        children.push(make_group_offset(props_pos));
        for pos in child_positions {
            children.push(make_group_offset(pos));
        }
        if !headers_data.is_empty() {
            children.push(make_data_offset(headers_pos));
        }
        
        self.write_group(&children)
    }
    
    /// Write properties compound and return its position.
    fn write_properties(&mut self, props: &[OProperty]) -> Result<u64> {
        if props.is_empty() {
            // Write empty compound
            return self.write_group(&[]);
        }
        
        // Write each property
        let mut prop_positions = Vec::new();
        for prop in props {
            let prop_pos = self.write_property(prop)?;
            prop_positions.push(prop_pos);
        }
        
        // Write property headers
        let headers_data = self.serialize_property_headers(props);
        let headers_pos = self.write_data(&headers_data)?;
        
        // Build compound group
        let mut children = Vec::new();
        for pos in prop_positions {
            children.push(make_group_offset(pos));
        }
        children.push(make_data_offset(headers_pos));
        
        self.write_group(&children)
    }
    
    /// Write a single property and return its position.
    fn write_property(&mut self, prop: &OProperty) -> Result<u64> {
        match &prop.data {
            OPropertyData::Scalar(samples) => {
                // Each sample is keyed data
                let mut children = Vec::new();
                for sample in samples {
                    let pos = self.write_keyed_data(sample)?;
                    children.push(make_data_offset(pos));
                }
                self.write_group(&children)
            }
            OPropertyData::Array(samples) => {
                // Each sample is (data, dimensions) pair
                let mut children = Vec::new();
                for (data, dims) in samples {
                    let data_pos = self.write_keyed_data(data)?;
                    let dims_data: Vec<u8> = dims.iter()
                        .flat_map(|d| (*d as u64).to_le_bytes())
                        .collect();
                    let dims_pos = self.write_data(&dims_data)?;
                    children.push(make_data_offset(data_pos));
                    children.push(make_data_offset(dims_pos));
                }
                self.write_group(&children)
            }
            OPropertyData::Compound(sub_props) => {
                self.write_properties(sub_props)
            }
        }
    }
    
    /// Serialize object headers for children.
    fn serialize_object_headers(&mut self, children: &[OObject], _parent_path: &str) -> Vec<u8> {
        if children.is_empty() {
            return Vec::new();
        }
        
        let mut buf = Vec::new();
        
        for child in children {
            // Name size (u32) + name + metadata index (u8)
            let name_bytes = child.name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            
            let meta_idx = self.add_indexed_metadata(&child.meta_data);
            if meta_idx == 0xff {
                // Inline metadata
                buf.push(0xff);
                let meta_str = child.meta_data.serialize();
                buf.extend_from_slice(&(meta_str.len() as u32).to_le_bytes());
                buf.extend_from_slice(meta_str.as_bytes());
            } else {
                buf.push(meta_idx);
            }
        }
        
        // Add 32 bytes of hash (zeros for simplicity)
        buf.extend_from_slice(&[0u8; 32]);
        
        buf
    }
    
    /// Serialize property headers.
    fn serialize_property_headers(&mut self, props: &[OProperty]) -> Vec<u8> {
        let mut buf = Vec::new();
        
        for prop in props {
            let info = self.build_property_info(prop);
            buf.extend_from_slice(&info.to_le_bytes());
            
            // Size hint determines how we write variable-length fields
            let size_hint = ((info >> 2) & 0x03) as u8;
            
            // For non-compound: next_sample_index, first/last changed, time_sampling_index
            if !matches!(prop.data, OPropertyData::Compound(_)) {
                let num_samples = prop.num_samples() as u32;
                write_with_hint(&mut buf, num_samples, size_hint);
                
                // first/last changed (if flag set)
                if (info & 0x0200) != 0 {
                    write_with_hint(&mut buf, prop.first_changed_index, size_hint);
                    write_with_hint(&mut buf, prop.last_changed_index, size_hint);
                }
                
                // time sampling index (if flag set)
                if (info & 0x0100) != 0 {
                    write_with_hint(&mut buf, prop.time_sampling_index, size_hint);
                }
            }
            
            // Property name
            let name_bytes = prop.name.as_bytes();
            write_with_hint(&mut buf, name_bytes.len() as u32, size_hint);
            buf.extend_from_slice(name_bytes);
            
            // Metadata (if inline)
            let meta_idx = self.add_indexed_metadata(&prop.meta_data);
            if meta_idx == 0xff {
                let meta_str = prop.meta_data.serialize();
                write_with_hint(&mut buf, meta_str.len() as u32, size_hint);
                buf.extend_from_slice(meta_str.as_bytes());
            }
        }
        
        buf
    }
    
    /// Build property info bitmask.
    fn build_property_info(&mut self, prop: &OProperty) -> u32 {
        let mut info: u32 = 0;
        
        // Property type (bits 0-1)
        match &prop.data {
            OPropertyData::Compound(_) => {
                info |= 0; // Compound
            }
            OPropertyData::Scalar(_) => {
                info |= 1; // Scalar
            }
            OPropertyData::Array(_) => {
                info |= 2; // Array
            }
        }
        
        // Size hint (bits 2-3) - use 2 (u32) for simplicity
        info |= 2 << 2;
        
        // For non-compound properties
        if !matches!(prop.data, OPropertyData::Compound(_)) {
            // POD type (bits 4-7)
            let pod = pod_to_u8(prop.data_type.pod) as u32;
            info |= (pod & 0x0f) << 4;
            
            // Extent (bits 12-19)
            info |= (prop.data_type.extent as u32 & 0xff) << 12;
            
            // Is homogenous (bit 10) - always true for now
            info |= 0x400;
            
            // Time sampling index flag (bit 8)
            if prop.time_sampling_index != 0 {
                info |= 0x0100;
            }
            
            // First/last changed flag (bit 9)
            let num_samples = prop.num_samples() as u32;
            if prop.first_changed_index != 1 || prop.last_changed_index != num_samples.saturating_sub(1) {
                info |= 0x0200;
            }
            
            // All samples same flag (bit 11)
            if prop.first_changed_index == 0 && prop.last_changed_index == 0 && num_samples > 1 {
                info |= 0x800;
            }
        }
        
        // Metadata index (bits 20-27)
        let meta_idx = self.add_indexed_metadata(&prop.meta_data);
        info |= (meta_idx as u32) << 20;
        
        info
    }

    /// Finalize and close the archive.
    pub fn close(mut self) -> Result<()> {
        if !self.frozen {
            // Write empty root
            let empty_root = OObject::new("");
            self.write_archive(&empty_root)?;
        }
        self.stream.flush()?;
        Ok(())
    }
}

/// Write value with size hint.
fn write_with_hint(buf: &mut Vec<u8>, value: u32, hint: u8) {
    match hint {
        0 => buf.push(value as u8),
        1 => buf.extend_from_slice(&(value as u16).to_le_bytes()),
        _ => buf.extend_from_slice(&value.to_le_bytes()),
    }
}

/// Convert POD type to u8.
fn pod_to_u8(pod: PlainOldDataType) -> u8 {
    match pod {
        PlainOldDataType::Boolean => 0,
        PlainOldDataType::Uint8 => 1,
        PlainOldDataType::Int8 => 2,
        PlainOldDataType::Uint16 => 3,
        PlainOldDataType::Int16 => 4,
        PlainOldDataType::Uint32 => 5,
        PlainOldDataType::Int32 => 6,
        PlainOldDataType::Uint64 => 7,
        PlainOldDataType::Int64 => 8,
        PlainOldDataType::Float16 => 9,
        PlainOldDataType::Float32 => 10,
        PlainOldDataType::Float64 => 11,
        PlainOldDataType::String => 12,
        PlainOldDataType::Wstring => 13,
        PlainOldDataType::Unknown => 0,
    }
}

// ============================================================================
// OObject - Object for Writing
// ============================================================================

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

// ============================================================================
// OProperty - Property for Writing
// ============================================================================

/// Property data variants.
#[derive(Clone)]
pub enum OPropertyData {
    /// Scalar property samples.
    Scalar(Vec<Vec<u8>>),
    /// Array property samples (data, dimensions).
    Array(Vec<(Vec<u8>, Vec<usize>)>),
    /// Compound property children.
    Compound(Vec<OProperty>),
}

/// Property for writing.
#[derive(Clone)]
pub struct OProperty {
    /// Property name.
    pub name: String,
    /// Data type.
    pub data_type: DataType,
    /// Metadata.
    pub meta_data: MetaData,
    /// Time sampling index.
    pub time_sampling_index: u32,
    /// First changed sample index.
    pub first_changed_index: u32,
    /// Last changed sample index.
    pub last_changed_index: u32,
    /// Property data.
    pub data: OPropertyData,
}

impl OProperty {
    /// Create a scalar property.
    pub fn scalar(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 1,
            last_changed_index: 0,
            data: OPropertyData::Scalar(Vec::new()),
        }
    }
    
    /// Create an array property.
    pub fn array(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 1,
            last_changed_index: 0,
            data: OPropertyData::Array(Vec::new()),
        }
    }
    
    /// Create a compound property.
    pub fn compound(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: DataType::UNKNOWN,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 0,
            last_changed_index: 0,
            data: OPropertyData::Compound(Vec::new()),
        }
    }
    
    /// Set metadata.
    pub fn with_meta_data(mut self, md: MetaData) -> Self {
        self.meta_data = md;
        self
    }
    
    /// Set time sampling index.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }
    
    /// Add a scalar sample.
    pub fn add_scalar_sample(&mut self, data: &[u8]) {
        if let OPropertyData::Scalar(samples) = &mut self.data {
            samples.push(data.to_vec());
            self.update_changed_indices();
        }
    }
    
    /// Add a scalar sample from Pod type.
    pub fn add_scalar_pod<T: bytemuck::Pod>(&mut self, value: &T) {
        self.add_scalar_sample(bytemuck::bytes_of(value));
    }
    
    /// Add an array sample.
    pub fn add_array_sample(&mut self, data: &[u8], dims: &[usize]) {
        if let OPropertyData::Array(samples) = &mut self.data {
            samples.push((data.to_vec(), dims.to_vec()));
            self.update_changed_indices();
        }
    }
    
    /// Add array sample from Pod slice.
    pub fn add_array_pod<T: bytemuck::Pod>(&mut self, values: &[T]) {
        let data = bytemuck::cast_slice(values);
        self.add_array_sample(data, &[values.len()]);
    }
    
    /// Add a child property (for compound).
    pub fn add_child(&mut self, prop: OProperty) -> Option<&mut OProperty> {
        if let OPropertyData::Compound(children) = &mut self.data {
            children.push(prop);
            children.last_mut()
        } else {
            None
        }
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        match &self.data {
            OPropertyData::Scalar(s) => s.len(),
            OPropertyData::Array(s) => s.len(),
            OPropertyData::Compound(_) => 0,
        }
    }
    
    /// Update changed indices based on samples.
    fn update_changed_indices(&mut self) {
        let n = self.num_samples() as u32;
        if n > 0 {
            self.first_changed_index = 1.min(n);
            self.last_changed_index = n.saturating_sub(1);
        }
    }
}

// ============================================================================
// Schema Builders
// ============================================================================

/// PolyMesh sample data.
pub struct OPolyMeshSample {
    pub positions: Vec<glam::Vec3>,
    pub face_counts: Vec<i32>,
    pub face_indices: Vec<i32>,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub normals: Option<Vec<glam::Vec3>>,
    pub uvs: Option<Vec<glam::Vec2>>,
}

impl OPolyMeshSample {
    /// Create new sample with required data.
    pub fn new(positions: Vec<glam::Vec3>, face_counts: Vec<i32>, face_indices: Vec<i32>) -> Self {
        Self {
            positions,
            face_counts,
            face_indices,
            velocities: None,
            normals: None,
            uvs: None,
        }
    }
}

/// PolyMesh schema writer.
pub struct OPolyMesh {
    object: OObject,
    geom_compound: OProperty,
    arb_geom_compound: Option<OProperty>,
}

impl OPolyMesh {
    /// Create a new PolyMesh.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_PolyMesh_v1");
        object.meta_data = meta;
        
        Self {
            object,
            geom_compound: OProperty::compound(".geom"),
            arb_geom_compound: None,
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OPolyMeshSample) {
        // Positions (P)
        let positions_prop = self.get_or_create_array(".geom", "P", 
            DataType::new(PlainOldDataType::Float32, 3));
        positions_prop.add_array_pod(&sample.positions);
        
        // Face counts (.faceCounts)
        let face_counts_prop = self.get_or_create_array(".geom", ".faceCounts",
            DataType::new(PlainOldDataType::Int32, 1));
        face_counts_prop.add_array_pod(&sample.face_counts);
        
        // Face indices (.faceIndices)
        let face_indices_prop = self.get_or_create_array(".geom", ".faceIndices",
            DataType::new(PlainOldDataType::Int32, 1));
        face_indices_prop.add_array_pod(&sample.face_indices);
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let vel_prop = self.get_or_create_array(".geom", ".velocities",
                DataType::new(PlainOldDataType::Float32, 3));
            vel_prop.add_array_pod(vels);
        }
        
        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            let normal_prop = self.get_or_create_array(".geom", "N",
                DataType::new(PlainOldDataType::Float32, 3));
            normal_prop.add_array_pod(normals);
        }
        
        // UVs (optional) - stored in .arbGeomParams
        if let Some(ref uvs) = sample.uvs {
            if self.arb_geom_compound.is_none() {
                self.arb_geom_compound = Some(OProperty::compound(".arbGeomParams"));
            }
            let uv_prop = self.get_or_create_arb_array("uv",
                DataType::new(PlainOldDataType::Float32, 2));
            uv_prop.add_array_pod(uvs);
        }
    }
    
    /// Get or create array property in compound.
    fn get_or_create_array(&mut self, _compound_name: &str, prop_name: &str, data_type: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            // Find existing
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            // Create new
            let prop = OProperty::array(prop_name, data_type);
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create arb geom param array.
    fn get_or_create_arb_array(&mut self, prop_name: &str, data_type: DataType) -> &mut OProperty {
        let arb = self.arb_geom_compound.get_or_insert_with(|| OProperty::compound(".arbGeomParams"));
        if let OPropertyData::Compound(children) = &mut arb.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let prop = OProperty::array(prop_name, data_type);
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        if let Some(arb) = self.arb_geom_compound {
            self.object.properties.push(arb);
        }
        self.object
    }
}

/// Xform sample data.
pub struct OXformSample {
    pub matrix: glam::Mat4,
    pub inherits: bool,
}

impl OXformSample {
    /// Create identity sample.
    pub fn identity() -> Self {
        Self {
            matrix: glam::Mat4::IDENTITY,
            inherits: true,
        }
    }
    
    /// Create from matrix.
    pub fn from_matrix(matrix: glam::Mat4, inherits: bool) -> Self {
        Self { matrix, inherits }
    }
}

/// Xform schema writer.
pub struct OXform {
    object: OObject,
    samples: Vec<OXformSample>,
}

impl OXform {
    /// Create new Xform.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Xform_v3");
        object.meta_data = meta;
        
        Self {
            object,
            samples: Vec::new(),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: OXformSample) {
        self.samples.push(sample);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.samples.is_empty() {
            let mut geom = OProperty::compound(".xform");
            
            // .vals - the matrix values
            let mut vals = OProperty::array(".vals", DataType::new(PlainOldDataType::Float64, 1));
            for sample in &self.samples {
                let cols = sample.matrix.to_cols_array();
                let doubles: Vec<f64> = cols.iter().map(|f| *f as f64).collect();
                vals.add_array_pod(&doubles);
            }
            
            // .ops - operation types
            let mut ops = OProperty::array(".ops", DataType::new(PlainOldDataType::Uint8, 1));
            // Single matrix op = 0 (kMatrixOperation)
            let op_data = vec![0u8; 1];
            for _ in &self.samples {
                ops.add_array_sample(&op_data, &[1]);
            }
            
            // .inherits
            let mut inherits = OProperty::scalar(".inherits", DataType::new(PlainOldDataType::Boolean, 1));
            for sample in &self.samples {
                inherits.add_scalar_pod(&(sample.inherits as u8));
            }
            
            if let OPropertyData::Compound(children) = &mut geom.data {
                children.push(vals);
                children.push(ops);
                children.push(inherits);
            }
            
            self.object.properties.push(geom);
        }
        
        self.object
    }
    
    /// Add child xform.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// OCurves - Curves Schema Writer
// ============================================================================

use crate::geom::{CurveType, CurvePeriodicity, BasisType};

/// Curves sample data for output.
pub struct OCurvesSample {
    pub positions: Vec<glam::Vec3>,
    pub num_vertices: Vec<i32>,
    pub curve_type: CurveType,
    pub wrap: CurvePeriodicity,
    pub basis: BasisType,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub widths: Option<Vec<f32>>,
    pub normals: Option<Vec<glam::Vec3>>,
    pub uvs: Option<Vec<glam::Vec2>>,
    pub knots: Option<Vec<f32>>,
    pub orders: Option<Vec<i32>>,
}

impl OCurvesSample {
    /// Create new curves sample.
    pub fn new(positions: Vec<glam::Vec3>, num_vertices: Vec<i32>) -> Self {
        Self {
            positions,
            num_vertices,
            curve_type: CurveType::Linear,
            wrap: CurvePeriodicity::NonPeriodic,
            basis: BasisType::NoBasis,
            velocities: None,
            widths: None,
            normals: None,
            uvs: None,
            knots: None,
            orders: None,
        }
    }
    
    /// Set curve type.
    pub fn with_curve_type(mut self, ct: CurveType) -> Self {
        self.curve_type = ct;
        self
    }
    
    /// Set periodicity.
    pub fn with_wrap(mut self, wrap: CurvePeriodicity) -> Self {
        self.wrap = wrap;
        self
    }
    
    /// Set basis type.
    pub fn with_basis(mut self, basis: BasisType) -> Self {
        self.basis = basis;
        self
    }
}

/// Curves schema writer.
pub struct OCurves {
    object: OObject,
    geom_compound: OProperty,
}

impl OCurves {
    /// Create new Curves.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Curve_v2");
        object.meta_data = meta;
        
        Self {
            object,
            geom_compound: OProperty::compound(".geom"),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OCurvesSample) {
        // Positions (P)
        let p_prop = self.get_or_create_array("P", DataType::new(PlainOldDataType::Float32, 3));
        p_prop.add_array_pod(&sample.positions);
        
        // nVertices
        let nv_prop = self.get_or_create_array("nVertices", DataType::new(PlainOldDataType::Int32, 1));
        nv_prop.add_array_pod(&sample.num_vertices);
        
        // curveBasisAndType (combined scalar)
        let cbt_prop = self.get_or_create_scalar("curveBasisAndType", DataType::new(PlainOldDataType::Uint8, 4));
        let cbt_data = [
            sample.curve_type as u8,
            sample.wrap as u8,
            sample.basis as u8,
            0u8,
        ];
        cbt_prop.add_scalar_sample(&cbt_data);
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let v_prop = self.get_or_create_array(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            v_prop.add_array_pod(vels);
        }
        
        // Widths (optional)
        if let Some(ref widths) = sample.widths {
            let w_prop = self.get_or_create_array("width", DataType::new(PlainOldDataType::Float32, 1));
            w_prop.add_array_pod(widths);
        }
        
        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            let n_prop = self.get_or_create_array("N", DataType::new(PlainOldDataType::Float32, 3));
            n_prop.add_array_pod(normals);
        }
        
        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            let uv_prop = self.get_or_create_array("uv", DataType::new(PlainOldDataType::Float32, 2));
            uv_prop.add_array_pod(uvs);
        }
        
        // Knots (optional, for NURBS)
        if let Some(ref knots) = sample.knots {
            let k_prop = self.get_or_create_array("knots", DataType::new(PlainOldDataType::Float32, 1));
            k_prop.add_array_pod(knots);
        }
        
        // Orders (optional, for NURBS)
        if let Some(ref orders) = sample.orders {
            let o_prop = self.get_or_create_array("orders", DataType::new(PlainOldDataType::Int32, 1));
            o_prop.add_array_pod(orders);
        }
    }
    
    fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    fn get_or_create_scalar(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::scalar(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }
    
    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// OPoints - Points Schema Writer
// ============================================================================

/// Points sample data for output.
pub struct OPointsSample {
    pub positions: Vec<glam::Vec3>,
    pub ids: Vec<u64>,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub widths: Option<Vec<f32>>,
}

impl OPointsSample {
    /// Create new points sample.
    pub fn new(positions: Vec<glam::Vec3>, ids: Vec<u64>) -> Self {
        Self {
            positions,
            ids,
            velocities: None,
            widths: None,
        }
    }
}

/// Points schema writer.
pub struct OPoints {
    object: OObject,
    geom_compound: OProperty,
}

impl OPoints {
    /// Create new Points.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Points_v1");
        object.meta_data = meta;
        
        Self {
            object,
            geom_compound: OProperty::compound(".geom"),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OPointsSample) {
        // Positions (P)
        let p_prop = self.get_or_create_array("P", DataType::new(PlainOldDataType::Float32, 3));
        p_prop.add_array_pod(&sample.positions);
        
        // IDs (id)
        let id_prop = self.get_or_create_array("id", DataType::new(PlainOldDataType::Uint64, 1));
        id_prop.add_array_pod(&sample.ids);
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let v_prop = self.get_or_create_array(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            v_prop.add_array_pod(vels);
        }
        
        // Widths (optional)
        if let Some(ref widths) = sample.widths {
            let w_prop = self.get_or_create_array("width", DataType::new(PlainOldDataType::Float32, 1));
            w_prop.add_array_pod(widths);
        }
    }
    
    fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }
    
    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// OSubD - Subdivision Surface Schema Writer
// ============================================================================

/// SubD sample data for output.
pub struct OSubDSample {
    pub positions: Vec<glam::Vec3>,
    pub face_counts: Vec<i32>,
    pub face_indices: Vec<i32>,
    pub subdivision_scheme: String,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub crease_indices: Option<Vec<i32>>,
    pub crease_lengths: Option<Vec<i32>>,
    pub crease_sharpnesses: Option<Vec<f32>>,
    pub corner_indices: Option<Vec<i32>>,
    pub corner_sharpnesses: Option<Vec<f32>>,
    pub holes: Option<Vec<i32>>,
    pub uvs: Option<Vec<glam::Vec2>>,
    pub uv_indices: Option<Vec<i32>>,
}

impl OSubDSample {
    /// Create new SubD sample.
    pub fn new(positions: Vec<glam::Vec3>, face_counts: Vec<i32>, face_indices: Vec<i32>) -> Self {
        Self {
            positions,
            face_counts,
            face_indices,
            subdivision_scheme: "catmullClark".to_string(),
            velocities: None,
            crease_indices: None,
            crease_lengths: None,
            crease_sharpnesses: None,
            corner_indices: None,
            corner_sharpnesses: None,
            holes: None,
            uvs: None,
            uv_indices: None,
        }
    }
    
    /// Set subdivision scheme.
    pub fn with_scheme(mut self, scheme: &str) -> Self {
        self.subdivision_scheme = scheme.to_string();
        self
    }
}

/// SubD schema writer.
pub struct OSubD {
    object: OObject,
    geom_compound: OProperty,
}

impl OSubD {
    /// Create new SubD.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_SubD_v1");
        object.meta_data = meta;
        
        Self {
            object,
            geom_compound: OProperty::compound(".geom"),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OSubDSample) {
        // Positions (P)
        let p_prop = self.get_or_create_array("P", DataType::new(PlainOldDataType::Float32, 3));
        p_prop.add_array_pod(&sample.positions);
        
        // Face counts
        let fc_prop = self.get_or_create_array(".faceCounts", DataType::new(PlainOldDataType::Int32, 1));
        fc_prop.add_array_pod(&sample.face_counts);
        
        // Face indices
        let fi_prop = self.get_or_create_array(".faceIndices", DataType::new(PlainOldDataType::Int32, 1));
        fi_prop.add_array_pod(&sample.face_indices);
        
        // Scheme
        let scheme_prop = self.get_or_create_scalar(".scheme", DataType::new(PlainOldDataType::String, 1));
        scheme_prop.add_scalar_sample(sample.subdivision_scheme.as_bytes());
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let v_prop = self.get_or_create_array(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            v_prop.add_array_pod(vels);
        }
        
        // Creases (optional)
        if let Some(ref indices) = sample.crease_indices {
            let prop = self.get_or_create_array(".creaseIndices", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(indices);
        }
        if let Some(ref lengths) = sample.crease_lengths {
            let prop = self.get_or_create_array(".creaseLengths", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(lengths);
        }
        if let Some(ref sharpnesses) = sample.crease_sharpnesses {
            let prop = self.get_or_create_array(".creaseSharpnesses", DataType::new(PlainOldDataType::Float32, 1));
            prop.add_array_pod(sharpnesses);
        }
        
        // Corners (optional)
        if let Some(ref indices) = sample.corner_indices {
            let prop = self.get_or_create_array(".cornerIndices", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(indices);
        }
        if let Some(ref sharpnesses) = sample.corner_sharpnesses {
            let prop = self.get_or_create_array(".cornerSharpnesses", DataType::new(PlainOldDataType::Float32, 1));
            prop.add_array_pod(sharpnesses);
        }
        
        // Holes (optional)
        if let Some(ref holes) = sample.holes {
            let prop = self.get_or_create_array(".holes", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(holes);
        }
        
        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            let prop = self.get_or_create_array("uv", DataType::new(PlainOldDataType::Float32, 2));
            prop.add_array_pod(uvs);
        }
        if let Some(ref uvi) = sample.uv_indices {
            let prop = self.get_or_create_array(".uvIndices", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(uvi);
        }
    }
    
    fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    fn get_or_create_scalar(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::scalar(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }
    
    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// OCamera - Camera Schema Writer
// ============================================================================

use crate::geom::CameraSample;

/// Camera schema writer.
pub struct OCamera {
    object: OObject,
    samples: Vec<CameraSample>,
}

impl OCamera {
    /// Create new Camera.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Camera_v1");
        object.meta_data = meta;
        
        Self {
            object,
            samples: Vec::new(),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: CameraSample) {
        self.samples.push(sample);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.samples.is_empty() {
            let mut geom = OProperty::compound(".geom");
            
            // Core properties stored as array of 16 f64s
            let mut core = OProperty::array(".core", DataType::new(PlainOldDataType::Float64, 1));
            for sample in &self.samples {
                let props: [f64; 16] = [
                    sample.focal_length,
                    sample.horizontal_aperture,
                    sample.horizontal_film_offset,
                    sample.vertical_aperture,
                    sample.vertical_film_offset,
                    sample.lens_squeeze_ratio,
                    sample.overscan_left,
                    sample.overscan_right,
                    sample.overscan_top,
                    sample.overscan_bottom,
                    sample.f_stop,
                    sample.focus_distance,
                    sample.shutter_open,
                    sample.shutter_close,
                    sample.near_clipping_plane,
                    sample.far_clipping_plane,
                ];
                core.add_array_pod(&props);
            }
            
            if let OPropertyData::Compound(children) = &mut geom.data {
                children.push(core);
            }
            
            self.object.properties.push(geom);
        }
        
        self.object
    }
    
    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// ONuPatch - NURBS Patch Schema Writer
// ============================================================================

/// NuPatch sample data for output.
pub struct ONuPatchSample {
    pub positions: Vec<glam::Vec3>,
    pub num_u: i32,
    pub num_v: i32,
    pub u_order: i32,
    pub v_order: i32,
    pub u_knot: Vec<f32>,
    pub v_knot: Vec<f32>,
    pub position_weights: Option<Vec<f32>>,
    pub velocities: Option<Vec<glam::Vec3>>,
    pub uvs: Option<Vec<glam::Vec2>>,
    pub normals: Option<Vec<glam::Vec3>>,
}

impl ONuPatchSample {
    /// Create new NuPatch sample.
    pub fn new(positions: Vec<glam::Vec3>, num_u: i32, num_v: i32, 
               u_order: i32, v_order: i32, u_knot: Vec<f32>, v_knot: Vec<f32>) -> Self {
        Self {
            positions,
            num_u,
            num_v,
            u_order,
            v_order,
            u_knot,
            v_knot,
            position_weights: None,
            velocities: None,
            uvs: None,
            normals: None,
        }
    }
}

/// NuPatch schema writer.
pub struct ONuPatch {
    object: OObject,
    geom_compound: OProperty,
}

impl ONuPatch {
    /// Create new NuPatch.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_NuPatch_v2");
        object.meta_data = meta;
        
        Self {
            object,
            geom_compound: OProperty::compound(".geom"),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &ONuPatchSample) {
        // Positions (P)
        let p_prop = self.get_or_create_array("P", DataType::new(PlainOldDataType::Float32, 3));
        p_prop.add_array_pod(&sample.positions);
        
        // numU, numV
        let nu_prop = self.get_or_create_scalar("nu", DataType::new(PlainOldDataType::Int32, 1));
        nu_prop.add_scalar_pod(&sample.num_u);
        
        let nv_prop = self.get_or_create_scalar("nv", DataType::new(PlainOldDataType::Int32, 1));
        nv_prop.add_scalar_pod(&sample.num_v);
        
        // Orders
        let uo_prop = self.get_or_create_scalar("uOrder", DataType::new(PlainOldDataType::Int32, 1));
        uo_prop.add_scalar_pod(&sample.u_order);
        
        let vo_prop = self.get_or_create_scalar("vOrder", DataType::new(PlainOldDataType::Int32, 1));
        vo_prop.add_scalar_pod(&sample.v_order);
        
        // Knots
        let uk_prop = self.get_or_create_array("uKnot", DataType::new(PlainOldDataType::Float32, 1));
        uk_prop.add_array_pod(&sample.u_knot);
        
        let vk_prop = self.get_or_create_array("vKnot", DataType::new(PlainOldDataType::Float32, 1));
        vk_prop.add_array_pod(&sample.v_knot);
        
        // Position weights (optional)
        if let Some(ref weights) = sample.position_weights {
            let prop = self.get_or_create_array("w", DataType::new(PlainOldDataType::Float32, 1));
            prop.add_array_pod(weights);
        }
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let prop = self.get_or_create_array(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            prop.add_array_pod(vels);
        }
        
        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            let prop = self.get_or_create_array("uv", DataType::new(PlainOldDataType::Float32, 2));
            prop.add_array_pod(uvs);
        }
        
        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            let prop = self.get_or_create_array("N", DataType::new(PlainOldDataType::Float32, 3));
            prop.add_array_pod(normals);
        }
    }
    
    fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    fn get_or_create_scalar(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::scalar(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }
    
    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// OLight - Light Schema Writer
// ============================================================================

/// Light schema writer.
pub struct OLight {
    object: OObject,
    camera_samples: Vec<CameraSample>,
}

impl OLight {
    /// Create new Light.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Light_v1");
        object.meta_data = meta;
        
        Self {
            object,
            camera_samples: Vec::new(),
        }
    }
    
    /// Add a camera sample (light parameters stored as camera).
    pub fn add_camera_sample(&mut self, sample: CameraSample) {
        self.camera_samples.push(sample);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.camera_samples.is_empty() {
            let mut geom = OProperty::compound(".geom");
            
            // Camera schema embedded in light
            let mut cam_compound = OProperty::compound(".camera");
            let mut core = OProperty::array(".core", DataType::new(PlainOldDataType::Float64, 1));
            
            for sample in &self.camera_samples {
                let props: [f64; 16] = [
                    sample.focal_length,
                    sample.horizontal_aperture,
                    sample.horizontal_film_offset,
                    sample.vertical_aperture,
                    sample.vertical_film_offset,
                    sample.lens_squeeze_ratio,
                    sample.overscan_left,
                    sample.overscan_right,
                    sample.overscan_top,
                    sample.overscan_bottom,
                    sample.f_stop,
                    sample.focus_distance,
                    sample.shutter_open,
                    sample.shutter_close,
                    sample.near_clipping_plane,
                    sample.far_clipping_plane,
                ];
                core.add_array_pod(&props);
            }
            
            if let OPropertyData::Compound(children) = &mut cam_compound.data {
                children.push(core);
            }
            
            if let OPropertyData::Compound(children) = &mut geom.data {
                children.push(cam_compound);
            }
            
            self.object.properties.push(geom);
        }
        
        self.object
    }
    
    /// Add child object.
    pub fn add_child(&mut self, child: OObject) {
        self.object.children.push(child);
    }
}

// ============================================================================
// OFaceSet - FaceSet Schema Writer
// ============================================================================

/// FaceSet sample data for output.
pub struct OFaceSetSample {
    pub faces: Vec<i32>,
}

impl OFaceSetSample {
    /// Create new FaceSet sample.
    pub fn new(faces: Vec<i32>) -> Self {
        Self { faces }
    }
}

/// FaceSet schema writer.
pub struct OFaceSet {
    object: OObject,
    geom_compound: OProperty,
}

impl OFaceSet {
    /// Create new FaceSet.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_FaceSet_v1");
        object.meta_data = meta;
        
        Self {
            object,
            geom_compound: OProperty::compound(".geom"),
        }
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OFaceSetSample) {
        let faces_prop = self.get_or_create_array(".faces", DataType::new(PlainOldDataType::Int32, 1));
        faces_prop.add_array_pod(&sample.faces);
    }
    
    fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        self.object.properties.push(self.geom_compound);
        self.object
    }
}

// ============================================================================
// OMaterial - Material Schema Writer
// ============================================================================

use crate::material::{ShaderParam, ShaderParamValue};

/// Material sample data for output.
pub struct OMaterialSample {
    pub targets: Vec<String>,
    pub shader_types: HashMap<String, Vec<String>>,
    pub shader_names: HashMap<(String, String), String>,
    pub params: Vec<ShaderParam>,
}

impl OMaterialSample {
    /// Create empty material sample.
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            shader_types: HashMap::new(),
            shader_names: HashMap::new(),
            params: Vec::new(),
        }
    }
    
    /// Add a shader.
    pub fn add_shader(&mut self, target: &str, shader_type: &str, shader_name: &str) {
        if !self.targets.contains(&target.to_string()) {
            self.targets.push(target.to_string());
        }
        self.shader_types.entry(target.to_string())
            .or_default()
            .push(shader_type.to_string());
        self.shader_names.insert(
            (target.to_string(), shader_type.to_string()),
            shader_name.to_string()
        );
    }
    
    /// Add a parameter.
    pub fn add_param(&mut self, param: ShaderParam) {
        self.params.push(param);
    }
}

impl Default for OMaterialSample {
    fn default() -> Self {
        Self::new()
    }
}

/// Material schema writer.
pub struct OMaterial {
    object: OObject,
    sample: OMaterialSample,
}

impl OMaterial {
    /// Create new Material.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcMaterial_Material_v1");
        object.meta_data = meta;
        
        Self {
            object,
            sample: OMaterialSample::new(),
        }
    }
    
    /// Set sample data.
    pub fn set_sample(&mut self, sample: OMaterialSample) {
        self.sample = sample;
    }
    
    /// Add a shader.
    pub fn add_shader(&mut self, target: &str, shader_type: &str, shader_name: &str) {
        self.sample.add_shader(target, shader_type, shader_name);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        let mut mat = OProperty::compound(".material");
        
        // Write targets
        if !self.sample.targets.is_empty() {
            let targets_str = self.sample.targets.join(";");
            let mut targets_prop = OProperty::scalar(".targets", DataType::new(PlainOldDataType::String, 1));
            targets_prop.add_scalar_sample(targets_str.as_bytes());
            
            if let OPropertyData::Compound(children) = &mut mat.data {
                children.push(targets_prop);
                
                // Write shader info per target
                for target in &self.sample.targets {
                    if let Some(types) = self.sample.shader_types.get(target) {
                        let types_str = types.join(";");
                        let mut types_prop = OProperty::scalar(
                            &format!(".{}.shaderTypes", target),
                            DataType::new(PlainOldDataType::String, 1)
                        );
                        types_prop.add_scalar_sample(types_str.as_bytes());
                        children.push(types_prop);
                        
                        for shader_type in types {
                            if let Some(name) = self.sample.shader_names.get(&(target.clone(), shader_type.clone())) {
                                let mut name_prop = OProperty::scalar(
                                    &format!(".{}.{}.shaderName", target, shader_type),
                                    DataType::new(PlainOldDataType::String, 1)
                                );
                                name_prop.add_scalar_sample(name.as_bytes());
                                children.push(name_prop);
                            }
                        }
                    }
                }
            }
        }
        
        // Write parameters
        for param in &self.sample.params {
            let prop_name = format!(".params.{}", param.name);
            let (dt, data) = match &param.value {
                ShaderParamValue::Float(v) => (
                    DataType::new(PlainOldDataType::Float32, 1),
                    bytemuck::bytes_of(v).to_vec()
                ),
                ShaderParamValue::Double(v) => (
                    DataType::new(PlainOldDataType::Float64, 1),
                    bytemuck::bytes_of(v).to_vec()
                ),
                ShaderParamValue::Vec2(v) => (
                    DataType::new(PlainOldDataType::Float32, 2),
                    bytemuck::bytes_of(v).to_vec()
                ),
                ShaderParamValue::Vec3(v) | ShaderParamValue::Color3(v) => (
                    DataType::new(PlainOldDataType::Float32, 3),
                    bytemuck::bytes_of(v).to_vec()
                ),
                ShaderParamValue::Vec4(v) | ShaderParamValue::Color4(v) => (
                    DataType::new(PlainOldDataType::Float32, 4),
                    bytemuck::bytes_of(v).to_vec()
                ),
                ShaderParamValue::Matrix(m) => (
                    DataType::new(PlainOldDataType::Float32, 16),
                    bytemuck::bytes_of(m).to_vec()
                ),
                ShaderParamValue::Int(v) => (
                    DataType::new(PlainOldDataType::Int32, 1),
                    bytemuck::bytes_of(v).to_vec()
                ),
                ShaderParamValue::String(s) => (
                    DataType::new(PlainOldDataType::String, 1),
                    s.as_bytes().to_vec()
                ),
                ShaderParamValue::Bool(v) => (
                    DataType::new(PlainOldDataType::Boolean, 1),
                    vec![*v as u8]
                ),
                ShaderParamValue::FloatArray(arr) => (
                    DataType::new(PlainOldDataType::Float32, 1),
                    bytemuck::cast_slice(arr).to_vec()
                ),
                ShaderParamValue::IntArray(arr) => (
                    DataType::new(PlainOldDataType::Int32, 1),
                    bytemuck::cast_slice(arr).to_vec()
                ),
                ShaderParamValue::StringArray(arr) => (
                    DataType::new(PlainOldDataType::String, 1),
                    arr.join(";").as_bytes().to_vec()
                ),
            };
            
            let mut prop = OProperty::scalar(&prop_name, dt);
            prop.add_scalar_sample(&data);
            
            if let OPropertyData::Compound(children) = &mut mat.data {
                children.push(prop);
            }
        }
        
        self.object.properties.push(mat);
        self.object
    }
}

// ============================================================================
// OCollections - Collections Schema Writer
// ============================================================================

/// Collections sample data for output.
pub struct OCollectionsSample {
    pub collections: HashMap<String, Vec<String>>,
}

impl OCollectionsSample {
    /// Create empty collections sample.
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
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
        
        Self {
            object,
            sample: OCollectionsSample::new(),
        }
    }
    
    /// Add a collection.
    pub fn add_collection(&mut self, name: &str, paths: Vec<String>) {
        self.sample.add_collection(name, paths);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        let mut coll = OProperty::compound(".collections");
        
        for (name, paths) in &self.sample.collections {
            // Each collection is an array of strings (paths)
            let paths_data: Vec<u8> = paths.iter()
                .flat_map(|p| {
                    let mut v = (p.len() as u32).to_le_bytes().to_vec();
                    v.extend_from_slice(p.as_bytes());
                    v
                })
                .collect();
            
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_empty_archive() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let path = temp.path();

        let archive = OArchive::create(path)?;
        archive.close()?;

        // Verify header
        let mut file = File::open(path)?;
        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header)?;

        assert_eq!(&header[0..5], OGAWA_MAGIC);
        assert_eq!(header[FROZEN_OFFSET], FROZEN_FLAG);
        assert_eq!(header[VERSION_OFFSET], 1);

        Ok(())
    }

    #[test]
    fn test_write_and_read_archive() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let path = temp.path();

        // Create archive with simple object
        let mut archive = OArchive::create(path)?;
        
        let mut root = OObject::new("");
        let child = OObject::new("test_child");
        root.add_child(child);
        
        archive.write_archive(&root)?;
        
        // Read back
        let reader = super::super::IArchive::open(path)?;
        assert!(reader.is_valid());
        assert!(reader.is_frozen());
        
        Ok(())
    }
    
    #[test]
    fn test_write_polymesh() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let path = temp.path();
        
        let mut archive = OArchive::create(path)?;
        
        // Create a simple triangle
        let mut mesh = OPolyMesh::new("triangle");
        mesh.add_sample(&OPolyMeshSample::new(
            vec![
                glam::Vec3::new(0.0, 0.0, 0.0),
                glam::Vec3::new(1.0, 0.0, 0.0),
                glam::Vec3::new(0.5, 1.0, 0.0),
            ],
            vec![3],
            vec![0, 1, 2],
        ));
        
        let mut root = OObject::new("");
        root.add_child(mesh.build());
        
        archive.write_archive(&root)?;
        
        // Verify file was written
        let reader = super::super::IArchive::open(path)?;
        assert!(reader.is_valid());
        
        Ok(())
    }
    
    #[test]
    fn test_write_xform() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let path = temp.path();
        
        let mut archive = OArchive::create(path)?;
        
        let mut xform = OXform::new("transform");
        xform.add_sample(OXformSample::identity());
        xform.add_sample(OXformSample::from_matrix(
            glam::Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0)),
            true,
        ));
        
        let mut root = OObject::new("");
        root.add_child(xform.build());
        
        archive.write_archive(&root)?;
        
        let reader = super::super::IArchive::open(path)?;
        assert!(reader.is_valid());
        
        Ok(())
    }
}
