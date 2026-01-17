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
use spooky_hash::SpookyHash;

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

/// Library version for written archives (1.8.8).
const ALEMBIC_LIBRARY_VERSION: i32 = 10808;

/// Ogawa file version.
/// Ogawa file format version - matches C++ ALEMBIC_OGAWA_FILE_VERSION = 0
const OGAWA_FILE_VERSION: i32 = 0;

/// Acyclic time per cycle marker.
/// Must match C++: std::numeric_limits<chrono_t>::max() / 32.0
const ACYCLIC_TIME_PER_CYCLE: f64 = f64::MAX / 32.0;

/// Size of digest/key prefix in data blocks.
const DATA_KEY_SIZE: usize = 16;

// ============================================================================
// OArchive - Main Archive Writer
// ============================================================================

/// Deferred group for bottom-up writing.
/// Matches C++ OGroup freeze behavior.
#[derive(Debug)]
struct DeferredGroup {
    /// Children of this group (data positions have MSB set, group indices don't)
    children: Vec<u64>,
    /// Final position after writing (set during flush)
    final_pos: Option<u64>,
}

impl DeferredGroup {
    fn new(children: Vec<u64>) -> Self {
        Self {
            children,
            final_pos: None,
        }
    }
}

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
    /// Deferred groups for bottom-up writing
    deferred_groups: Vec<DeferredGroup>,
    /// Use deferred group writing (C++ compatible mode)
    deferred_mode: bool,
}

/// Context for computing object headers inside write_properties.
/// 
/// Object headers depend on data_hash which is computed during property writing,
/// so we need to pass the context to compute headers at the right moment.
/// This struct is passed from write_object to write_properties_with_object_headers.
struct ObjectHeadersContext<'a> {
    children: &'a [OObject],
    child_hash1: u64,
    child_hash2: u64,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let name = path.as_ref().to_string_lossy().to_string();
        let mut stream = OStream::create(&path)?;

        // Write header with placeholder for root position
        stream.write_bytes(OGAWA_MAGIC)?;
        stream.write_u8(NOT_FROZEN_FLAG)?;
        // Version as big-endian (matching C++ Alembic format: {0, 1} = version 1)
        stream.write_bytes(&CURRENT_VERSION.to_be_bytes())?;
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
            application_writer: String::new(),  // Empty by default for C++ parity
            compression_hint: -1,
            dedup_map: HashMap::new(),
            dedup_enabled: true,
            deferred_groups: Vec::new(),
            deferred_mode: false,  // Disabled for binary parity - write groups inline
        })
    }
    
    /// Enable or disable deduplication.
    /// Deduplication saves space by storing identical data only once.
    pub fn setDedupEnabled(&mut self, enabled: bool) {
        self.dedup_enabled = enabled;
    }
    
    /// Check if deduplication is enabled.
    pub fn isDedupEnabled(&self) -> bool {
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
    
    /// Get the archive name/path (Alembic API name).
    pub fn getName(&self) -> &str {
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
    pub fn setCompressionHint(&mut self, hint: i32) {
        self.compression_hint = hint.clamp(-1, 9);
    }
    
    /// Get compression hint.
    pub fn getCompressionHint(&self) -> i32 {
        self.compression_hint
    }
    
    /// Set the archive metadata (e.g. when copying from another file).
    /// This also clears the application_writer so we don't add our own app name.
    pub fn set_archive_metadata(&mut self, md: MetaData) {
        self.archive_metadata = md;
        // Clear app name so we don't override the source metadata
        self.application_writer.clear();
    }
    
    /// Set the application name (stored as _ai_Application in metadata).
    pub fn setAppName(&mut self, name: &str) {
        self.archive_metadata.set("_ai_Application", name);
    }
    
    /// Set the date written (stored as _ai_DateWritten in metadata).
    pub fn setDateWritten(&mut self, date: &str) {
        self.archive_metadata.set("_ai_DateWritten", date);
    }
    
    /// Set the user description (stored as _ai_Description in metadata).
    pub fn setUserDescription(&mut self, desc: &str) {
        self.archive_metadata.set("_ai_Description", desc);
    }
    
    /// Set the DCC FPS (stored as _ai_DCC_FPS in metadata).
    pub fn setDccFps(&mut self, fps: f64) {
        self.archive_metadata.set("_ai_DCC_FPS", fps.to_string());
    }
    
    /// Add a time sampling and return its index.
    pub fn addTimeSampling(&mut self, ts: TimeSampling) -> u32 {
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
    pub fn getNumTimeSamplings(&self) -> usize {
        self.time_samplings.len()
    }
    
    /// Get a time sampling by index.
    pub fn getTimeSampling(&self, index: usize) -> Option<&TimeSampling> {
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
    
    // =========================================================================
    // Deferred Group Writing (C++ compatible mode)
    // =========================================================================
    
    /// Marker constant for deferred group placeholders (uses high bits that won't conflict with real positions)
    const DEFERRED_GROUP_MARKER: u64 = 0x4000_0000_0000_0000; // Bit 62 set
    
    /// Add a deferred group and return a placeholder reference.
    /// The placeholder can be used as a child reference in parent groups.
    /// Actual position is resolved when flush_deferred_groups is called.
    fn add_deferred_group(&mut self, children: Vec<u64>) -> u64 {
        if children.is_empty() {
            return 0; // Empty group marker
        }
        let idx = self.deferred_groups.len();
        self.deferred_groups.push(DeferredGroup::new(children));
        // Return placeholder: marker | index
        Self::DEFERRED_GROUP_MARKER | (idx as u64)
    }
    
    /// Check if a value is a deferred group placeholder.
    #[inline]
    fn is_deferred_placeholder(value: u64) -> bool {
        // Check if marker bit is set and MSB (data marker) is not
        (value & Self::DEFERRED_GROUP_MARKER) != 0 && !is_data_offset(value)
    }
    
    /// Extract deferred group index from placeholder.
    #[inline]
    fn deferred_group_index(placeholder: u64) -> usize {
        (placeholder & !Self::DEFERRED_GROUP_MARKER) as usize
    }
    
    /// Flush all deferred groups, writing them in bottom-up order.
    /// Returns the position of the last (root) group.
    fn flush_deferred_groups(&mut self) -> Result<u64> {
        if self.deferred_groups.is_empty() {
            return Ok(0);
        }
        
        // Build dependency graph to determine write order
        // Groups must be written after all their child groups
        let mut group_deps: Vec<Vec<usize>> = vec![Vec::new(); self.deferred_groups.len()];
        
        for (i, group) in self.deferred_groups.iter().enumerate() {
            for &child in &group.children {
                if Self::is_deferred_placeholder(child) {
                    let child_idx = Self::deferred_group_index(child);
                    group_deps[i].push(child_idx);
                }
            }
        }
        
        // Topological sort: process groups with no unprocessed dependencies first
        let mut written: Vec<bool> = vec![false; self.deferred_groups.len()];
        let mut order: Vec<usize> = Vec::with_capacity(self.deferred_groups.len());
        
        while order.len() < self.deferred_groups.len() {
            let mut found = false;
            for i in 0..self.deferred_groups.len() {
                if written[i] {
                    continue;
                }
                // Check if all dependencies are satisfied
                let deps_ok = group_deps[i].iter().all(|&d| written[d]);
                if deps_ok {
                    order.push(i);
                    written[i] = true;
                    found = true;
                }
            }
            if !found {
                // Circular dependency - shouldn't happen
                return Err(Error::invalid("Circular dependency in deferred groups"));
            }
        }
        
        // Write groups in topological order (leaves first)
        let mut last_pos = 0u64;
        for &idx in &order {
            // Resolve children: replace deferred placeholders with actual positions
            let mut resolved_children = Vec::new();
            for &child in &self.deferred_groups[idx].children {
                if Self::is_deferred_placeholder(child) {
                    let child_idx = Self::deferred_group_index(child);
                    let child_pos = self.deferred_groups[child_idx].final_pos
                        .ok_or_else(|| Error::invalid("Deferred group not yet written"))?;
                    resolved_children.push(make_group_offset(child_pos));
                } else {
                    resolved_children.push(child);
                }
            }
            
            // Write the group
            let pos = self.stream.pos();
            self.stream.write_u64(resolved_children.len() as u64)?;
            for &child in &resolved_children {
                self.stream.write_u64(child)?;
            }
            
            self.deferred_groups[idx].final_pos = Some(pos);
            last_pos = pos;
        }
        
        Ok(last_pos)
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

        // Write version data first (as per official implementation)
        let version_pos = self.write_data(&OGAWA_FILE_VERSION.to_le_bytes())?;

        // Write library version data second (as per official implementation)
        let file_version_pos = self.write_data(&ALEMBIC_LIBRARY_VERSION.to_le_bytes())?;

        // Write all objects recursively, collect positions
        let (root_obj_pos, _, _) = self.write_object(root, "/")?;

        // C++ BINARY PARITY: Flush deferred groups FIRST.
        // In C++, object groups are frozen when their destructors run.
        // For "triangle" and "/" objects, their groups freeze during the
        // OwData destructor cascade triggered by m_data.reset() in AwImpl::~AwImpl.
        // This happens BEFORE archive metadata/time samplings/indexed metadata.
        let final_root_obj_pos = if self.deferred_mode {
            if Self::is_deferred_placeholder(root_obj_pos) {
                self.flush_deferred_groups()?;
                let idx = Self::deferred_group_index(root_obj_pos);
                self.deferred_groups[idx].final_pos
                    .ok_or_else(|| Error::invalid("Root object group not written"))?
            } else {
                root_obj_pos
            }
        } else {
            root_obj_pos
        };

        // Write archive metadata AFTER flushing object groups (C++ parity)
        // Include application_writer if not already set via set_app_name
        let mut archive_meta = self.archive_metadata.clone();
        if archive_meta.get("_ai_Application").is_none() && !self.application_writer.is_empty() {
            archive_meta.set("_ai_Application", &self.application_writer);
        }
        // Also set the Alembic version as in the reference implementation
        if archive_meta.get("_ai_AlembicVersion").is_none() {
            archive_meta.set("_ai_AlembicVersion", "Alembic 1.8.8 (built Aug  4 2025 10:01:52)");
        }
        let archive_meta_str = archive_meta.serialize();
        let archive_meta_pos = if archive_meta_str.is_empty() {
            0
        } else {
            self.write_data(archive_meta_str.as_bytes())?
        };

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
            make_group_offset(final_root_obj_pos),
            make_data_offset(archive_meta_pos),
            make_data_offset(ts_pos),
            make_data_offset(idx_meta_pos),
        ];

        // Archive root group is always written immediately (not deferred)
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
    
    /// Write an object and return (position, hash1, hash2).
    /// 
    /// Matches C++ OwData::writeHeaders() hashing approach:
    /// - dataHash: computed from properties via CpwData::computeHash()
    /// - childHash: computed from child object hashes
    /// - Both hashes written as 32-byte suffix in object headers
    /// - ioHash returned to parent: childHash updated with dataHash
    fn write_object(&mut self, obj: &OObject, parent_path: &str) -> Result<(u64, u64, u64)> {
        let full_path = if parent_path == "/" {
            format!("/{}", obj.name)
        } else {
            format!("{}/{}", parent_path, obj.name)
        };
        
        // C++ BINARY PARITY: Process child objects FIRST to get their hashes.
        // We need child_hashes before calling write_properties_with_object_headers
        // because object headers (which includes child info) must be written
        // BEFORE parent property headers, but AFTER child compound groups.
        let mut child_positions = Vec::new();
        let mut child_hashes: Vec<u64> = Vec::new();
        for child in &obj.children {
            let (child_pos, h1, h2) = self.write_object(child, &full_path)?;
            child_positions.push(child_pos);
            child_hashes.push(h1);
            child_hashes.push(h2);
        }
        
        // Compute child objects hash (ioHash) matching C++ OwData::writeHeaders()
        let (child_hash1, child_hash2) = if child_hashes.is_empty() {
            (0u64, 0u64)
        } else {
            let child_hash_bytes: Vec<u8> = child_hashes.iter()
                .flat_map(|h| h.to_le_bytes())
                .collect();
            let mut hasher = SpookyHash::new(0, 0);
            hasher.update(&child_hash_bytes);
            hasher.finalize()
        };
        
        // Write properties compound WITH object headers context.
        // Object headers will be written inside, between nested compound groups
        // and parent property headers, matching C++ destructor order.
        let obj_ctx = ObjectHeadersContext {
            children: &obj.children,
            child_hash1,
            child_hash2,
        };
        let (props_pos, data_hash1, data_hash2, headers_pos, _) = 
            self.write_properties_with_object_headers(&obj.properties, Some(obj_ctx))?;
        
        
        // Build object group:
        // Child 0: properties compound (group)
        // Children 1..n-1: child objects (groups)
        // Child n-1: object headers (data)
        let mut children = Vec::new();
        
        // In deferred mode, props_pos might be a placeholder, so check
        if Self::is_deferred_placeholder(props_pos) {
            children.push(props_pos); // Keep placeholder as-is
        } else {
            children.push(make_group_offset(props_pos));
        }
        
        for pos in child_positions {
            if Self::is_deferred_placeholder(pos) {
                children.push(pos); // Keep placeholder as-is
            } else {
                children.push(make_group_offset(pos));
            }
        }
        
        if headers_pos != 0 {
            children.push(make_data_offset(headers_pos));
        }
        
        let pos = if self.deferred_mode {
            self.add_deferred_group(children)
        } else {
            self.write_group(&children)?
        };
        
        // Compute the final hash returned to parent.
        // C++ in OwImpl::~OwImpl() does:
        //   1. hash.Init(0, 0)
        //   2. writeHeaders updates hash with child_hashes + data_hash
        //   3. hash.Update(metadata.serialize())
        //   4. hash.Update(object_name)
        //   5. hash.Final()
        let mut combined_hash = SpookyHash::new(0, 0);
        
        // Step 2a: Update with child hashes (if any)
        if !child_hashes.is_empty() {
            let child_hash_bytes: Vec<u8> = child_hashes.iter()
                .flat_map(|h| h.to_le_bytes())
                .collect();
            combined_hash.update(&child_hash_bytes);
        }
        // Step 2b: Update with data hash
        combined_hash.update(&data_hash1.to_le_bytes());
        combined_hash.update(&data_hash2.to_le_bytes());
        
        // Step 3: Update with metadata (if not empty)
        let meta_str = obj.meta_data.serialize();
        if !meta_str.is_empty() {
            combined_hash.update(meta_str.as_bytes());
        }
        
        // Step 4: Update with object name
        combined_hash.update(obj.name.as_bytes());
        
        // Step 5: Finalize
        let (final_h1, final_h2) = combined_hash.finalize();
        
        Ok((pos, final_h1, final_h2))
    }
    
    /// Write properties compound and return its position.
    #[allow(dead_code)]
    fn write_properties(&mut self, props: &[OProperty]) -> Result<u64> {
        let (pos, _, _, _) = self.write_properties_with_data(props)?;
        Ok(pos)
    }
    
    /// Write properties compound and return (position, hash1, hash2, raw_prop_hashes).
    /// 
    /// Matches C++ CpwData::computeHash() approach:
    /// - Collects (hash1, hash2) pairs from each child property
    /// - Updates hash with all child hashes
    /// 
    /// IMPORTANT: For NESTED compound properties, the caller must re-hash
    /// the returned prop_hashes WITH the property header (name + metadata)
    /// to match C++ CpwImpl::~CpwImpl behavior.
    fn write_properties_with_data(&mut self, props: &[OProperty]) -> Result<(u64, u64, u64, Vec<u64>)> {
        // Wrapper that calls internal without object headers (for recursive compound calls)
        let (pos, h1, h2, _, raw_hashes) = self.write_properties_with_object_headers(props, None)?;
        Ok((pos, h1, h2, raw_hashes))
    }
    
    /// Write properties compound with optional object headers context.
    /// 
    /// In C++, when an object (OwImpl) destructor runs, it writes object headers
    /// BEFORE the property compound (CpwImpl) destructor writes property headers.
    /// This means object headers must be written between:
    /// - Phase 2 (finalize nested compound groups) 
    /// - Phase 3 (write parent property headers)
    /// 
    /// The obj_ctx parameter is only passed from write_object for top-level
    /// property compounds, not for nested compound properties.
    /// 
    /// Returns (props_pos, data_hash1, data_hash2, object_headers_pos)
    /// Write properties compound with optional object headers.
    /// 
    /// Returns (props_pos, data_hash1, data_hash2, object_headers_pos, raw_prop_hashes).
    /// 
    /// The raw_prop_hashes are needed for NESTED compounds to recompute the hash
    /// with property header (matching C++ CpwImpl::~CpwImpl behavior).
    fn write_properties_with_object_headers(
        &mut self,
        props: &[OProperty],
        obj_ctx: Option<ObjectHeadersContext<'_>>,
    ) -> Result<(u64, u64, u64, u64, Vec<u64>)> {
        if props.is_empty() {
            // Empty compound - but we may still need to write object headers!
            // C++ calls dataHash.Final() even for empty compound, which returns non-zero
            // SpookyHash::new(0,0).finalize() matches C++ SpookyHash::Final() with no updates
            let hasher = SpookyHash::new(0, 0);
            let (h1, h2) = hasher.finalize();
            
            // Write object headers if context provided (for root object with children but no props)
            let obj_headers_pos = if let Some(ctx) = obj_ctx {
                let obj_headers = self.serialize_object_headers_with_hash(
                    ctx.children,
                    h1, h2,  // data_hash for empty compound
                    ctx.child_hash1, ctx.child_hash2,
                );
                self.write_data(&obj_headers)?
            } else {
                0
            };
            
            // Write empty compound group
            let pos = self.write_group(&[])?;
            return Ok((pos, h1, h2, obj_headers_pos, Vec::new()));
        }
        
        // Create sorted indices by data_write_order for data writing order
        // But keep original compound order for group children
        let mut sorted_indices: Vec<usize> = (0..props.len()).collect();
        sorted_indices.sort_by_key(|&i| props[i].data_write_order);
        
        // C++ binary parity: write ALL sample data first, then ALL property groups
        // Phase 1: Write sample data for all properties, collect group children
        let mut prop_states: Vec<(Vec<u64>, Option<(u64, u64)>)> = vec![(Vec::new(), None); props.len()];
        
        for &idx in &sorted_indices {
            let state = self.collect_property_sample_data(&props[idx])?;
            prop_states[idx] = state;
        }
        
        // Phase 2: Write property groups in REVERSE compound order
        // C++ writes groups during destruction, which is reverse of creation order
        let mut prop_positions = vec![0u64; props.len()];
        let mut prop_hashes_pairs = vec![(0u64, 0u64); props.len()];
        
        for idx in (0..props.len()).rev() {
            let (children, sample_hash) = std::mem::take(&mut prop_states[idx]);
            let (pos, h1, h2) = self.finalize_property_group(&props[idx], children, sample_hash)?;
            prop_positions[idx] = pos;
            prop_hashes_pairs[idx] = (h1, h2);
        }
        
        // Collect hashes in compound order (original order)
        let mut prop_hashes: Vec<u64> = Vec::new();
        for (h1, h2) in &prop_hashes_pairs {
            prop_hashes.push(*h1);
            prop_hashes.push(*h2);
        }
        
        // C++ BINARY PARITY: Write object headers BEFORE property headers.
        // In C++, the OwImpl destructor body calls m_data->finalize()->writeHeaders()
        // which writes object headers. Then member destructors run, and CpwData
        // destructor writes property headers. So object headers come first.
        //
        // Now that we have data_hash (computed from prop_hashes), we can compute
        // and write object headers if context was provided.
        // Compute compound hash matching C++ CpwData::computeHash()
        // hash.Update(&m_hashes.front(), m_hashes.size() * 8)
        // Note: For non-empty compounds, this computes hash of child hashes.
        // For empty compounds, C++ returns (0, 0) - but we don't reach here for empty.
        let (data_h1, data_h2) = {
            let hash_bytes: Vec<u8> = prop_hashes.iter()
                .flat_map(|h| h.to_le_bytes())
                .collect();
            let mut hasher = SpookyHash::new(0, 0);
            hasher.update(&hash_bytes);
            hasher.finalize()
        };
        
        let obj_headers_pos = if let Some(ctx) = obj_ctx {
            let obj_headers = self.serialize_object_headers_with_hash(
                ctx.children,
                data_h1, data_h2,
                ctx.child_hash1, ctx.child_hash2,
            );
            let pos = self.write_data(&obj_headers)?;
            pos
        } else {
            0
        };
        
        // Write property headers
        let headers_data = self.serialize_property_headers(props);
        let headers_pos = self.write_data(&headers_data)?;
        
        // Build compound group
        let mut children = Vec::new();
        for pos in prop_positions {
            if Self::is_deferred_placeholder(pos) {
                children.push(pos); // Keep placeholder as-is
            } else {
                children.push(make_group_offset(pos));
            }
        }
        children.push(make_data_offset(headers_pos));
        
        // IMPORTANT: Property compound groups must be written INLINE (not deferred)
        // to match C++ destructor ordering. In C++, when a CpwImpl destructor runs,
        // it writes property headers and then the OGroup destructor freezes the group.
        // This happens BEFORE the parent compound can continue writing.
        // Deferred mode is only for OBJECT groups, not property compound groups.
        let pos = self.write_group(&children)?;
        
        // data_h1, data_h2 already computed above before object headers
        // Return raw prop_hashes for nested compounds to recompute hash with property header
        Ok((pos, data_h1, data_h2, obj_headers_pos, prop_hashes))
    }
    
    /// Write a single property and return its position.
    #[allow(dead_code)]
    fn write_property(&mut self, prop: &OProperty) -> Result<u64> {
        let (pos, _, _) = self.write_property_with_data(prop)?;
        Ok(pos)
    }
    
    /// Phase 1: Collect sample data for a property without writing the group.
    /// Returns (group_children, sample_hash) for later finalization.
    fn collect_property_sample_data(&mut self, prop: &OProperty) -> Result<(Vec<u64>, Option<(u64, u64)>)> {
        let ts_idx = prop.time_sampling_index;
        
        match &prop.data {
            OPropertyData::Scalar(samples) => {
                self.update_max_samples(ts_idx, samples.len() as u32);
                
                let mut sample_hash: Option<(u64, u64)> = None;
                let mut children = Vec::new();
                
                for sample in samples {
                    let content_key = crate::core::ArraySampleContentKey::from_data(sample);
                    let digest = content_key.digest();
                    let d0 = u64::from_le_bytes(digest[0..8].try_into().unwrap());
                    let d1 = u64::from_le_bytes(digest[8..16].try_into().unwrap());
                    
                    sample_hash = match sample_hash {
                        None => Some((d0, d1)),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d0, d1)),
                    };
                    
                    let pos = self.write_keyed_data(sample)?;
                    children.push(make_data_offset(pos));
                }
                Ok((children, sample_hash))
            }
            OPropertyData::Array(samples) => {
                self.update_max_samples(ts_idx, samples.len() as u32);
                
                let mut sample_hash: Option<(u64, u64)> = None;
                let mut children = Vec::new();
                
                for (data, dims) in samples {
                    let content_key = crate::core::ArraySampleContentKey::from_data(data);
                    let digest = content_key.digest();
                    let mut d = (
                        u64::from_le_bytes(digest[0..8].try_into().unwrap()),
                        u64::from_le_bytes(digest[8..16].try_into().unwrap()),
                    );
                    hash_dimensions(dims, &mut d);
                    
                    sample_hash = match sample_hash {
                        None => Some(d),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d.0, d.1)),
                    };
                    
                    let data_pos = self.write_keyed_data(data)?;
                    
                    let dims_offset = if dims.len() <= 1 && 
                        !matches!(prop.data_type.pod, PlainOldDataType::String | PlainOldDataType::Wstring) 
                    {
                        EMPTY_DATA
                    } else {
                        let dims_data: Vec<u8> = dims.iter()
                            .flat_map(|dim| (*dim as u64).to_le_bytes())
                            .collect();
                        make_data_offset(self.write_data(&dims_data)?)
                    };
                    
                    children.push(make_data_offset(data_pos));
                    children.push(dims_offset);
                }
                Ok((children, sample_hash))
            }
            OPropertyData::Compound(_) => {
                // Compounds are handled recursively in finalize_property_group
                Ok((Vec::new(), None))
            }
        }
    }
    
    /// Phase 2: Finalize a property by writing its group and computing hash.
    fn finalize_property_group(
        &mut self, 
        prop: &OProperty, 
        children: Vec<u64>, 
        sample_hash: Option<(u64, u64)>
    ) -> Result<(u64, u64, u64)> {
        let ts_idx = prop.time_sampling_index;
        let time_sampling = self.time_samplings.get(ts_idx as usize)
            .cloned()
            .unwrap_or_else(TimeSampling::identity);
        
        match &prop.data {
            OPropertyData::Scalar(_) | OPropertyData::Array(_) => {
                // Write the property group INLINE (not deferred!)
                // This matches C++ destructor order: property groups are written
                // during property destruction, which happens in reverse compound order.
                // The compound group itself may be deferred, but individual property
                // groups must be written inline to maintain exact C++ binary parity.
                let pos = self.write_group(&children)?;
                
                // Compute final hash
                let mut hasher = SpookyHash::new(0, 0);
                hash_property_header(&mut hasher, prop, &time_sampling);
                
                if let Some((sh0, sh1)) = sample_hash {
                    let mut sample_bytes = Vec::with_capacity(16);
                    sample_bytes.extend_from_slice(&sh0.to_le_bytes());
                    sample_bytes.extend_from_slice(&sh1.to_le_bytes());
                    hasher.update(&sample_bytes);
                }
                
                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
            OPropertyData::Compound(sub_props) => {
                // C++ CpwImpl::~CpwImpl for NESTED compounds:
                // 1. computeHash(hash) - updates with m_hashes (child property hashes)
                // 2. HashPropertyHeader(header, hash) - adds property name + metadata
                // 3. hash.Final() - produces final hash
                //
                // write_properties_with_data returns raw prop_hashes so we can
                // recompute hash WITH property header for correct C++ parity.
                let (pos, _, _, raw_prop_hashes) = self.write_properties_with_data(sub_props)?;
                
                // Recompute hash: child_hashes + property_header
                let mut hasher = SpookyHash::new(0, 0);
                
                // First: hash child property hashes (m_hashes in C++)
                let hash_bytes: Vec<u8> = raw_prop_hashes.iter()
                    .flat_map(|h| h.to_le_bytes())
                    .collect();
                hasher.update(&hash_bytes);
                
                // Second: hash property header (name + metadata)
                hash_property_header(&mut hasher, prop, &time_sampling);
                
                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
        }
    }
    
    /// Write a single property and return (position, hash1, hash2).
    /// 
    /// Matches C++ SpwImpl/ApwImpl destructor hashing:
    /// 1. HashPropertyHeader(header, hash)
    /// 2. If samples exist, hash.Update(m_hash.d, 16)
    /// 3. hash.Final(&hash0, &hash1)
    fn write_property_with_data(&mut self, prop: &OProperty) -> Result<(u64, u64, u64)> {
        // Get time sampling for this property
        let ts_idx = prop.time_sampling_index;
        let time_sampling = self.time_samplings.get(ts_idx as usize)
            .cloned()
            .unwrap_or_else(TimeSampling::identity);
        
        match &prop.data {
            OPropertyData::Scalar(samples) => {
                // Update max_samples for this time sampling
                let num_samples = samples.len() as u32;
                self.update_max_samples(ts_idx, num_samples);
                
                // Accumulate sample hashes
                // C++ uses SpookyHash::ShortEnd to mix sample digests
                let mut sample_hash: Option<(u64, u64)> = None;
                
                // Write samples and accumulate hash
                let mut children = Vec::new();
                for sample in samples {
                    // Compute sample digest (MD5-based key)
                    let content_key = crate::core::ArraySampleContentKey::from_data(sample);
                    let digest = content_key.digest();
                    let d0 = u64::from_le_bytes(digest[0..8].try_into().unwrap());
                    let d1 = u64::from_le_bytes(digest[8..16].try_into().unwrap());
                    
                    // Accumulate hash
                    sample_hash = match sample_hash {
                        None => Some((d0, d1)),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d0, d1)),
                    };
                    
                    let pos = self.write_keyed_data(sample)?;
                    children.push(make_data_offset(pos));
                }
                // C++ writes property groups inline (not deferred) for binary parity
                // Only compound groups are deferred
                let pos = self.write_group(&children)?;
                
                // Final hash: HashPropertyHeader + sample hash
                let mut hasher = SpookyHash::new(0, 0);
                hash_property_header(&mut hasher, prop, &time_sampling);
                
                if let Some((sh0, sh1)) = sample_hash {
                    // Mix in accumulated sample hash
                    let mut sample_bytes = Vec::with_capacity(16);
                    sample_bytes.extend_from_slice(&sh0.to_le_bytes());
                    sample_bytes.extend_from_slice(&sh1.to_le_bytes());
                    hasher.update(&sample_bytes);
                }
                
                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
            OPropertyData::Array(samples) => {
                // Update max_samples for this time sampling
                let num_samples = samples.len() as u32;
                self.update_max_samples(ts_idx, num_samples);
                
                // Accumulate sample hashes with dimensions
                // C++ calls HashDimensions then SpookyHash::ShortEnd
                let mut sample_hash: Option<(u64, u64)> = None;
                
                // Write samples and accumulate hash
                let mut children = Vec::new();
                for (data, dims) in samples {
                    // Compute sample digest
                    let content_key = crate::core::ArraySampleContentKey::from_data(data);
                    let digest = content_key.digest();
                    let mut d = (
                        u64::from_le_bytes(digest[0..8].try_into().unwrap()),
                        u64::from_le_bytes(digest[8..16].try_into().unwrap()),
                    );
                    
                    // Hash dimensions into digest
                    hash_dimensions(dims, &mut d);
                    
                    // Accumulate hash
                    sample_hash = match sample_hash {
                        None => Some(d),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d.0, d.1)),
                    };
                    
                    let data_pos = self.write_keyed_data(data)?;
                    
                    // C++ WriteDimensions: use EMPTY_DATA for rank <= 1 (non-string types)
                    // This allows dimensions to be inferred from data size
                    let dims_offset = if dims.len() <= 1 && 
                        !matches!(prop.data_type.pod, PlainOldDataType::String | PlainOldDataType::Wstring) 
                    {
                        EMPTY_DATA  // No data written, just marker
                    } else {
                        let dims_data: Vec<u8> = dims.iter()
                            .flat_map(|dim| (*dim as u64).to_le_bytes())
                            .collect();
                        make_data_offset(self.write_data(&dims_data)?)
                    };
                    
                    children.push(make_data_offset(data_pos));
                    children.push(dims_offset);
                }
                // C++ writes property groups inline (not deferred) for binary parity
                // Only compound groups are deferred
                let pos = self.write_group(&children)?;
                
                // Final hash: HashPropertyHeader + sample hash
                let mut hasher = SpookyHash::new(0, 0);
                hash_property_header(&mut hasher, prop, &time_sampling);
                
                if let Some((sh0, sh1)) = sample_hash {
                    let mut sample_bytes = Vec::with_capacity(16);
                    sample_bytes.extend_from_slice(&sh0.to_le_bytes());
                    sample_bytes.extend_from_slice(&sh1.to_le_bytes());
                    hasher.update(&sample_bytes);
                }
                
                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
            OPropertyData::Compound(sub_props) => {
                // Same as finalize_property_group Compound branch:
                // C++ CpwImpl::~CpwImpl hashes child hashes + property header
                let (pos, _, _, raw_prop_hashes) = self.write_properties_with_data(sub_props)?;
                
                let mut hasher = SpookyHash::new(0, 0);
                let hash_bytes: Vec<u8> = raw_prop_hashes.iter()
                    .flat_map(|h| h.to_le_bytes())
                    .collect();
                hasher.update(&hash_bytes);
                hash_property_header(&mut hasher, prop, &time_sampling);
                
                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
        }
    }
    
    /// Serialize object headers for children (without hash suffix).
    #[allow(dead_code)]
    fn serialize_object_headers(&mut self, children: &[OObject], _parent_path: &str) -> Vec<u8> {
        self.serialize_object_headers_with_hash(children, 0, 0, 0, 0)
    }
    
    /// Serialize object headers for children with 32-byte SpookyHash suffix.
    /// 
    /// Format per C++ reference (OwData.cpp writeHeaders):
    /// - For each child: name_size + name + metadata_index [+ inline_metadata]
    /// - 32 bytes of hashes at end: [data_hash1, data_hash2, child_hash1, child_hash2]
    fn serialize_object_headers_with_hash(
        &mut self,
        children: &[OObject],
        data_hash1: u64,
        data_hash2: u64,
        child_hash1: u64,
        child_hash2: u64,
    ) -> Vec<u8> {
        // Even if no children, we need the hash suffix if we have data
        let mut buf = Vec::new();

        for child in children {
            // Name size (u32 with hint 2) + name
            let name_bytes = child.name.as_bytes();
            write_with_hint(&mut buf, name_bytes.len() as u32, 2); // Hint 2 for 4 bytes
            buf.extend_from_slice(name_bytes);

            // Metadata index (1 byte)
            let meta_idx = self.add_indexed_metadata(&child.meta_data);
            write_with_hint(&mut buf, meta_idx as u32, 0); // 1 byte for metadata index

            // If metadata index is 0xff, write inline metadata
            if meta_idx == 0xff {
                let meta_str = child.meta_data.serialize();
                write_with_hint(&mut buf, meta_str.len() as u32, 2); // Hint 2 for 4 bytes
                buf.extend_from_slice(meta_str.as_bytes());
            }
        }

        // Append 32 bytes of hashes (4 x u64, little-endian)
        // Order: data_hash1, data_hash2, child_hash1, child_hash2
        buf.extend_from_slice(&data_hash1.to_le_bytes());
        buf.extend_from_slice(&data_hash2.to_le_bytes());
        buf.extend_from_slice(&child_hash1.to_le_bytes());
        buf.extend_from_slice(&child_hash2.to_le_bytes());

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
                let num_samples = prop.getNumSamples() as u32;
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

        // Calculate size hint based on max of name size, metadata size, num samples, and time sampling index
        let name_size = prop.name.len() as u32;
        let meta_data = prop.meta_data.serialize();
        let meta_data_size = meta_data.len() as u32;
        let num_samples = prop.getNumSamples() as u32;
        let time_sampling_index = prop.time_sampling_index;

        let max_size = meta_data_size.max(name_size).max(num_samples).max(time_sampling_index);
        let size_hint = if max_size > 255 && max_size < 65536 {
            1
        } else if max_size >= 65536 {
            2
        } else {
            0
        };

        // Size hint (bits 2-3)
        info |= (size_hint & 0x03) << 2;

        // Property type (bits 0-1)
        // Bit 0 = is_scalar_like, bit 1 = is_array
        // 0=compound, 1=scalar, 2=array, 3=scalar-like-array
        match &prop.data {
            OPropertyData::Compound(_) => {
                info |= 0; // Compound
            }
            OPropertyData::Scalar(_) => {
                info |= 1; // Scalar (always scalar-like)
            }
            OPropertyData::Array(_) => {
                if prop.is_scalar_like {
                    info |= 3; // Scalar-like array
                } else {
                    info |= 2; // Regular array
                }
            }
        }

        // For non-compound properties
        if !matches!(prop.data, OPropertyData::Compound(_)) {
            // POD type (bits 4-7)
            let pod = pod_to_u8(prop.data_type.pod) as u32;
            info |= (pod & 0x0f) << 4;

            // Extent (bits 12-19)
            info |= (prop.data_type.extent as u32 & 0xff) << 12;

            // Is homogenous (bit 10)
            // C++ Alembic has a bug: WrittenSampleID stores extent * numPoints,
            // but comparison uses dims.numPoints(). For extent > 1 on first sample,
            // these differ and isHomogenous becomes false.
            // For scalar properties: always true
            // For array properties with extent > 1: false (matching C++ bug)
            // For array properties with extent == 1: true
            let is_array = matches!(prop.data, OPropertyData::Array(_));
            let is_homogenous = !is_array || prop.data_type.extent == 1;
            if is_homogenous {
                info |= 0x400;
            }

            // Time sampling index flag (bit 8)
            if prop.time_sampling_index != 0 {
                info |= 0x0100;
            }

            // Whether first/last index exists (bit 9)
            let num_samples = prop.getNumSamples() as u32;
            if prop.first_changed_index == 0 && prop.last_changed_index == 0 {
                // All samples same flag (bit 11)
                info |= 0x800;
            } else if prop.first_changed_index != 1 || prop.last_changed_index != num_samples.saturating_sub(1) {
                info |= 0x0200; // Whether first/last index exists
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

/// Push f64 (chrono_t) to buffer as little-endian bytes.
fn push_chrono(buf: &mut Vec<u8>, value: f64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Hash a property header matching C++ HashPropertyHeader().
/// 
/// This builds the same data buffer as C++ and updates the hasher.
/// For non-compound properties, includes:
/// - name bytes
/// - metadata serialized bytes  
/// - POD type (1 byte)
/// - extent (1 byte)
/// - 0 if scalar (1 byte) - arrays don't get this byte
/// - timePerCycle (8 bytes)
/// - samplesPerCycle count (4 bytes)
/// - each stored time (8 bytes)
fn hash_property_header(
    hasher: &mut SpookyHash,
    prop: &OProperty,
    time_sampling: &TimeSampling,
) {
    let mut data = Vec::new();
    
    // Name
    data.extend_from_slice(prop.name.as_bytes());
    
    // Metadata
    let meta = prop.meta_data.serialize();
    data.extend_from_slice(meta.as_bytes());
    
    // For non-compound properties
    if !matches!(prop.data, OPropertyData::Compound(_)) {
        // POD type
        data.push(pod_to_u8(prop.data_type.pod));
        // Extent
        data.push(prop.data_type.extent);
        
        // Scalar marker (only for scalars)
        if matches!(prop.data, OPropertyData::Scalar(_)) {
            data.push(0);
        }
        // Note: Arrays don't push anything here (matches C++)
        
        // Time per cycle
        let (tpc, times) = match &time_sampling.sampling_type {
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
        push_chrono(&mut data, tpc);
        
        // Samples per cycle count (4 bytes)
        let spc = times.len() as u32;
        data.extend_from_slice(&spc.to_le_bytes());
        
        // Each stored time
        for t in times {
            push_chrono(&mut data, t);
        }
    }
    
    if !data.is_empty() {
        hasher.update(&data);
    }
}

/// Hash dimensions for array samples, matching C++ HashDimensions().
fn hash_dimensions(dims: &[usize], digest: &mut (u64, u64)) {
    if dims.is_empty() {
        return;
    }
    
    let mut hasher = SpookyHash::new(0, 0);
    
    // Dimensions as u64 values
    let dims_bytes: Vec<u8> = dims.iter()
        .flat_map(|d| (*d as u64).to_le_bytes())
        .collect();
    hasher.update(&dims_bytes);
    
    // Update with existing digest
    let mut digest_bytes = Vec::with_capacity(16);
    digest_bytes.extend_from_slice(&digest.0.to_le_bytes());
    digest_bytes.extend_from_slice(&digest.1.to_le_bytes());
    hasher.update(&digest_bytes);
    
    *digest = hasher.finalize();
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
    /// Is scalar-like (for array properties that behave like scalars).
    /// When true, bit 0 of property type is set (ptype 3 instead of 2 for arrays).
    pub is_scalar_like: bool,
    /// Data write order - determines order of data in file (C++ parity).
    /// Lower values are written first. Properties with same order use compound order.
    pub data_write_order: u32,
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
            is_scalar_like: true,
            data_write_order: u32::MAX, // Default: use compound order
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
            is_scalar_like: false,
            data_write_order: u32::MAX,
        }
    }
    
    /// Create an array property that behaves like a scalar (scalar-like).
    pub fn scalar_like_array(name: &str, data_type: DataType) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            meta_data: MetaData::new(),
            time_sampling_index: 0,
            first_changed_index: 1,
            last_changed_index: 0,
            data: OPropertyData::Array(Vec::new()),
            is_scalar_like: true,
            data_write_order: u32::MAX,
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
            is_scalar_like: false,
            data_write_order: u32::MAX,
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
    
    /// Get or create an array child property by name.
    /// 
    /// If a property with the given name exists, returns it.
    /// Otherwise creates a new array property and returns it.
    pub fn get_or_create_array_child(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::array(name, dt));
            children.last_mut().unwrap()
        } else {
            panic!("get_or_create_array_child called on non-compound property")
        }
    }
    
    /// Get or create a scalar child property by name.
    /// 
    /// If a property with the given name exists, returns it.
    /// Otherwise creates a new scalar property and returns it.
    pub fn get_or_create_scalar_child(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            children.push(OProperty::scalar(name, dt));
            children.last_mut().unwrap()
        } else {
            panic!("get_or_create_scalar_child called on non-compound property")
        }
    }
    
    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        match &self.data {
            OPropertyData::Scalar(s) => s.len(),
            OPropertyData::Array(s) => s.len(),
            OPropertyData::Compound(_) => 0,
        }
    }
    
    /// Update changed indices based on samples.
    fn update_changed_indices(&mut self) {
        let n = self.getNumSamples() as u32;
        if n == 0 {
            self.first_changed_index = 0;
            self.last_changed_index = 0;
        } else if n == 1 {
            // Single sample = all samples same (static property)
            self.first_changed_index = 0;
            self.last_changed_index = 0;
        } else {
            // Multiple samples - assume all change (animation)
            self.first_changed_index = 1;
            self.last_changed_index = n - 1;
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
    time_sampling_index: u32,
}

impl OPolyMesh {
    /// Create a new PolyMesh.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_PolyMesh_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_PolyMesh_v1:.geom");
        object.meta_data = meta;
        
        // .geom compound with schema metadata
        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_PolyMesh_v1");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;
        
        Self {
            object,
            geom_compound: geom,
            arb_geom_compound: None,
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    /// 
    /// Property creation order follows C++ OPolyMeshSchema::createPositionsProperty():
    /// 1. P (positions) - created first, adds geoScope=vtx;interpretation=point to indexed metadata
    /// 2. .selfBnds - created by createSelfBoundsProperty() called after P
    /// 3. .faceIndices, .faceCounts - created in set() method
    /// 
    /// This order is critical for indexed metadata table parity with C++.
    pub fn add_sample(&mut self, sample: &OPolyMeshSample) {
        // C++ property CREATION order determines compound order (indexed metadata):
        // .selfBnds → P → .faceIndices → .faceCounts
        // 
        // C++ data WRITE order follows setSample() call order:
        // P → .faceIndices → .faceCounts → .selfBnds
        //
        // We must create properties in compound order, then write data in write order.
        
        // First ensure all properties exist in correct compound order
        // (only creates on first sample, subsequent calls find existing)
        // Compound order: .selfBnds → P → .faceIndices → .faceCounts
        // Data write order: P(0) → .faceIndices(1) → .faceCounts(2) → .selfBnds(3)
        let mut bnds_meta = MetaData::new();
        bnds_meta.set("interpretation", "box");
        let bnds_prop = self.get_or_create_scalar_with_meta(".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6), bnds_meta);
        bnds_prop.data_write_order = 3;
        
        let mut p_meta = MetaData::new();
        p_meta.set("geoScope", "vtx");
        p_meta.set("interpretation", "point");
        let p_prop = self.get_or_create_array_with_meta("P", 
            DataType::new(PlainOldDataType::Float32, 3), p_meta);
        p_prop.data_write_order = 0;
        
        let fi_prop = self.get_or_create_array_with_ts(".faceIndices",
            DataType::new(PlainOldDataType::Int32, 1));
        fi_prop.data_write_order = 1;
        
        let fc_prop = self.get_or_create_scalar_like_array_with_ts(".faceCounts",
            DataType::new(PlainOldDataType::Int32, 1));
        fc_prop.data_write_order = 2;
        
        // Add data (order here doesn't matter, data_write_order controls file layout)
        let positions_prop = self.get_or_create_array_with_meta("P", 
            DataType::new(PlainOldDataType::Float32, 3), MetaData::new());
        positions_prop.add_array_pod(&sample.positions);
        
        let face_indices_prop = self.get_or_create_array_with_ts(".faceIndices",
            DataType::new(PlainOldDataType::Int32, 1));
        face_indices_prop.add_array_pod(&sample.face_indices);
        
        let face_counts_prop = self.get_or_create_scalar_like_array_with_ts(".faceCounts",
            DataType::new(PlainOldDataType::Int32, 1));
        face_counts_prop.add_array_pod(&sample.face_counts);
        
        let bounds = Self::compute_bounds(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6), MetaData::new());
        self_bnds_prop.add_scalar_pod(&bounds);
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let vel_prop = self.get_or_create_array_with_ts(".velocities",
                DataType::new(PlainOldDataType::Float32, 3));
            vel_prop.add_array_pod(vels);
        }
        
        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            let normal_prop = self.get_or_create_array_with_ts("N",
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
    
    /// Compute bounding box from positions (returns [minX, minY, minZ, maxX, maxY, maxZ]).
    fn compute_bounds(positions: &[glam::Vec3]) -> [f64; 6] {
        if positions.is_empty() {
            return [0.0; 6];
        }
        let mut min = glam::DVec3::splat(f64::MAX);
        let mut max = glam::DVec3::splat(f64::MIN);
        for p in positions {
            let p = glam::DVec3::new(p.x as f64, p.y as f64, p.z as f64);
            min = min.min(p);
            max = max.max(p);
        }
        [min.x, min.y, min.z, max.x, max.y, max.z]
    }
    
    /// Get or create array property with time sampling index set.
    fn get_or_create_array_with_ts(&mut self, prop_name: &str, data_type: DataType) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            // Find existing
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            // Create new with time sampling
            let mut prop = OProperty::array(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create scalar-like array property with time sampling index set.
    /// Scalar-like arrays have property type 3 (bit 0 set) indicating they behave like scalars.
    fn get_or_create_scalar_like_array_with_ts(&mut self, prop_name: &str, data_type: DataType) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar_like_array(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create array property with metadata and time sampling.
    fn get_or_create_array_with_meta(&mut self, prop_name: &str, data_type: DataType, meta: MetaData) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            prop.meta_data = meta;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create scalar property with metadata and time sampling.
    fn get_or_create_scalar_with_meta(&mut self, prop_name: &str, data_type: DataType, meta: MetaData) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
            prop.meta_data = meta;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create arb geom param array.
    fn get_or_create_arb_array(&mut self, prop_name: &str, data_type: DataType) -> &mut OProperty {
        let ts_idx = self.time_sampling_index;
        let arb = self.arb_geom_compound.get_or_insert_with(|| OProperty::compound(".arbGeomParams"));
        if let OPropertyData::Compound(children) = &mut arb.data {
            if let Some(idx) = children.iter().position(|p| p.name == prop_name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(prop_name, data_type);
            prop.time_sampling_index = ts_idx;
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
    time_sampling_index: u32,
}

impl OXform {
    /// Create new Xform.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Xform_v3");
        // Xform has no schemaBaseType (empty string)
        meta.set("schemaObjTitle", "AbcGeom_Xform_v3:.xform");
        object.meta_data = meta;
        
        Self {
            object,
            samples: Vec::new(),
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn with_time_sampling(mut self, index: u32) -> Self {
        self.time_sampling_index = index;
        self
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: OXformSample) {
        self.samples.push(sample);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.samples.is_empty() {
            let mut geom = OProperty::compound(".xform");
            // .xform compound metadata
            let mut geom_meta = MetaData::new();
            geom_meta.set("schema", "AbcGeom_Xform_v3");
            geom.meta_data = geom_meta;
            
            // Per IXform.cpp: .ops is scalar with extent = num_ops
            // .vals is scalar with extent = total_num_values (16 for Matrix)
            // For Matrix4x4 operation: extent=16 for vals, extent=1 for ops
            
            // .vals - scalar with extent=16 for 4x4 matrix
            // Alembic row-major (row-vector v*M) and glam column-major (column-vector M*v)
            // have the SAME flat layout for translation: indices 12-14
            // So we can write glam's column-major directly
            let mut vals = OProperty::scalar(".vals", DataType::new(PlainOldDataType::Float64, 16));
            vals.time_sampling_index = self.time_sampling_index;
            for sample in &self.samples {
                let m = sample.matrix;
                // Write glam column-major directly - same layout as Alembic row-major
                let mat_f64: [f64; 16] = [
                    m.x_axis.x as f64, m.x_axis.y as f64, m.x_axis.z as f64, m.x_axis.w as f64,
                    m.y_axis.x as f64, m.y_axis.y as f64, m.y_axis.z as f64, m.y_axis.w as f64,
                    m.z_axis.x as f64, m.z_axis.y as f64, m.z_axis.z as f64, m.z_axis.w as f64,
                    m.w_axis.x as f64, m.w_axis.y as f64, m.w_axis.z as f64, m.w_axis.w as f64,
                ];
                vals.add_scalar_sample(bytemuck::cast_slice(&mat_f64));
            }
            
            // .ops - scalar with extent=1 (one Matrix op)
            // Op encoding: (type << 4) | hint
            // Matrix = type 3, hint 0 -> (3 << 4) | 0 = 0x30
            let mut ops = OProperty::scalar(".ops", DataType::new(PlainOldDataType::Uint8, 1));
            ops.time_sampling_index = self.time_sampling_index;
            for _ in &self.samples {
                ops.add_scalar_pod(&0x30u8);
            }
            
            // .inherits - scalar with extent=1
            let mut inherits = OProperty::scalar(".inherits", DataType::new(PlainOldDataType::Boolean, 1));
            inherits.time_sampling_index = self.time_sampling_index;
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
    time_sampling_index: u32,
}

impl OCurves {
    /// Create new Curves.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Curve_v2");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_Curve_v2:.geom");
        object.meta_data = meta;
        
        // .geom compound with schema metadata
        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_Curve_v2");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;
        
        Self {
            object,
            geom_compound: geom,
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    /// 
    /// Property creation order follows C++ OCurvesSchema pattern:
    /// P is created first, then .selfBnds via createSelfBoundsProperty().
    /// This order is critical for indexed metadata table parity with C++.
    pub fn add_sample(&mut self, sample: &OCurvesSample) {
        // C++ order: P → .selfBnds → nVertices → curveBasisAndType
        // P is created first, then createSelfBoundsProperty() is called
        
        // Positions (P) with metadata: geoScope=vtx, interpretation=point
        // Created FIRST to match C++ indexed metadata order
        let p_prop = self.get_or_create_array_with_meta("P", 
            DataType::new(PlainOldDataType::Float32, 3), Self::p_meta());
        p_prop.add_array_pod(&sample.positions);
        
        // Self bounds (.selfBnds) - created AFTER P
        let bounds = Self::compute_bounds(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6), Self::bnds_meta());
        self_bnds_prop.add_scalar_pod(&bounds);
        
        // nVertices
        let nv_prop = self.geom_compound.get_or_create_array_child("nVertices", DataType::new(PlainOldDataType::Int32, 1));
        nv_prop.add_array_pod(&sample.num_vertices);
        
        // curveBasisAndType (combined scalar)
        let cbt_prop = self.geom_compound.get_or_create_scalar_child("curveBasisAndType", DataType::new(PlainOldDataType::Uint8, 4));
        let cbt_data = [
            sample.curve_type as u8,
            sample.wrap as u8,
            sample.basis as u8,
            0u8,
        ];
        cbt_prop.add_scalar_sample(&cbt_data);
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let v_prop = self.geom_compound.get_or_create_array_child(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            v_prop.add_array_pod(vels);
        }
        
        // Widths (optional)
        if let Some(ref widths) = sample.widths {
            let w_prop = self.geom_compound.get_or_create_array_child("width", DataType::new(PlainOldDataType::Float32, 1));
            w_prop.add_array_pod(widths);
        }
        
        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            let n_prop = self.geom_compound.get_or_create_array_child("N", DataType::new(PlainOldDataType::Float32, 3));
            n_prop.add_array_pod(normals);
        }
        
        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            let uv_prop = self.geom_compound.get_or_create_array_child("uv", DataType::new(PlainOldDataType::Float32, 2));
            uv_prop.add_array_pod(uvs);
        }
        
        // Knots (optional, for NURBS)
        if let Some(ref knots) = sample.knots {
            let k_prop = self.geom_compound.get_or_create_array_child("knots", DataType::new(PlainOldDataType::Float32, 1));
            k_prop.add_array_pod(knots);
        }
        
        // Orders (optional, for NURBS)
        if let Some(ref orders) = sample.orders {
            let o_prop = self.geom_compound.get_or_create_array_child("orders", DataType::new(PlainOldDataType::Int32, 1));
            o_prop.add_array_pod(orders);
        }
    }
    
    /// Create P property metadata.
    fn p_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("geoScope", "vtx");
        meta.set("interpretation", "point");
        meta
    }
    
    /// Create bounds metadata.
    fn bnds_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("interpretation", "box");
        meta
    }
    
    /// Compute bounding box from positions.
    fn compute_bounds(positions: &[glam::Vec3]) -> [f64; 6] {
        if positions.is_empty() {
            return [0.0; 6];
        }
        let mut min = glam::DVec3::splat(f64::MAX);
        let mut max = glam::DVec3::splat(f64::MIN);
        for p in positions {
            let p = glam::DVec3::new(p.x as f64, p.y as f64, p.z as f64);
            min = min.min(p);
            max = max.max(p);
        }
        [min.x, min.y, min.z, max.x, max.y, max.z]
    }
    
    /// Get or create array property with metadata.
    fn get_or_create_array_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create scalar property with metadata.
    fn get_or_create_scalar_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
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
    time_sampling_index: u32,
}

impl OPoints {
    /// Create new Points.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Points_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_Points_v1:.geom");
        object.meta_data = meta;
        
        // .geom compound with schema metadata
        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_Points_v1");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;
        
        Self {
            object,
            geom_compound: geom,
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    /// 
    /// Property creation order follows C++ OPointsSchema pattern:
    /// P is created first, then .selfBnds via createSelfBoundsProperty().
    /// This order is critical for indexed metadata table parity with C++.
    pub fn add_sample(&mut self, sample: &OPointsSample) {
        // C++ order: P → .selfBnds → .pointIds
        // P is created first, then createSelfBoundsProperty() is called
        
        // Positions (P) with metadata: geoScope=var (varying), interpretation=point
        // Created FIRST to match C++ indexed metadata order
        let p_prop = self.get_or_create_array_with_meta("P", 
            DataType::new(PlainOldDataType::Float32, 3), 
            Self::p_meta());
        p_prop.add_array_pod(&sample.positions);
        
        // Self bounds (.selfBnds) - created AFTER P
        let bounds = Self::compute_bounds(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6), Self::bnds_meta());
        self_bnds_prop.add_scalar_pod(&bounds);
        
        // IDs (.pointIds) - C++ name
        let id_prop = self.geom_compound.get_or_create_array_child(".pointIds", DataType::new(PlainOldDataType::Uint64, 1));
        id_prop.add_array_pod(&sample.ids);
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let v_prop = self.geom_compound.get_or_create_array_child(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            v_prop.add_array_pod(vels);
        }
        
        // Widths (optional)
        if let Some(ref widths) = sample.widths {
            let w_prop = self.geom_compound.get_or_create_array_child("width", DataType::new(PlainOldDataType::Float32, 1));
            w_prop.add_array_pod(widths);
        }
    }
    
    /// Create P property metadata (geoScope=var for varying points).
    fn p_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("geoScope", "var");
        meta.set("interpretation", "point");
        meta
    }
    
    /// Create bounds metadata.
    fn bnds_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("interpretation", "box");
        meta
    }
    
    /// Compute bounding box from positions.
    fn compute_bounds(positions: &[glam::Vec3]) -> [f64; 6] {
        if positions.is_empty() {
            return [0.0; 6];
        }
        let mut min = glam::DVec3::splat(f64::MAX);
        let mut max = glam::DVec3::splat(f64::MIN);
        for p in positions {
            let p = glam::DVec3::new(p.x as f64, p.y as f64, p.z as f64);
            min = min.min(p);
            max = max.max(p);
        }
        [min.x, min.y, min.z, max.x, max.y, max.z]
    }
    
    /// Get or create array property with metadata.
    fn get_or_create_array_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create scalar property with metadata.
    fn get_or_create_scalar_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
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
    time_sampling_index: u32,
}

impl OSubD {
    /// Create new SubD.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_SubD_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_SubD_v1:.geom");
        object.meta_data = meta;
        
        // .geom compound with schema metadata
        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_SubD_v1");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;
        
        Self {
            object,
            geom_compound: geom,
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    /// 
    /// Property creation order follows C++ OSubDSchema pattern:
    /// P is created first, then .selfBnds via createSelfBoundsProperty().
    /// This order is critical for indexed metadata table parity with C++.
    pub fn add_sample(&mut self, sample: &OSubDSample) {
        // C++ order: P → .selfBnds → .faceIndices → .faceCounts
        // P is created first, then createSelfBoundsProperty() is called
        
        // Positions (P) with metadata: geoScope=vtx, interpretation=point
        // Created FIRST to match C++ indexed metadata order
        let p_prop = self.get_or_create_array_with_meta("P", 
            DataType::new(PlainOldDataType::Float32, 3), Self::p_meta());
        p_prop.add_array_pod(&sample.positions);
        
        // Self bounds (.selfBnds) - created AFTER P
        let bounds = Self::compute_bounds(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6), Self::bnds_meta());
        self_bnds_prop.add_scalar_pod(&bounds);
        
        // Face indices
        let fi_prop = self.geom_compound.get_or_create_array_child(".faceIndices", DataType::new(PlainOldDataType::Int32, 1));
        fi_prop.add_array_pod(&sample.face_indices);
        
        // Face counts
        let fc_prop = self.geom_compound.get_or_create_array_child(".faceCounts", DataType::new(PlainOldDataType::Int32, 1));
        fc_prop.add_array_pod(&sample.face_counts);
        
        // Scheme
        let scheme_prop = self.geom_compound.get_or_create_scalar_child(".scheme", DataType::new(PlainOldDataType::String, 1));
        scheme_prop.add_scalar_sample(sample.subdivision_scheme.as_bytes());
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let v_prop = self.geom_compound.get_or_create_array_child(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            v_prop.add_array_pod(vels);
        }
        
        // Creases (optional)
        if let Some(ref indices) = sample.crease_indices {
            let prop = self.geom_compound.get_or_create_array_child(".creaseIndices", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(indices);
        }
        if let Some(ref lengths) = sample.crease_lengths {
            let prop = self.geom_compound.get_or_create_array_child(".creaseLengths", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(lengths);
        }
        if let Some(ref sharpnesses) = sample.crease_sharpnesses {
            let prop = self.geom_compound.get_or_create_array_child(".creaseSharpnesses", DataType::new(PlainOldDataType::Float32, 1));
            prop.add_array_pod(sharpnesses);
        }
        
        // Corners (optional)
        if let Some(ref indices) = sample.corner_indices {
            let prop = self.geom_compound.get_or_create_array_child(".cornerIndices", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(indices);
        }
        if let Some(ref sharpnesses) = sample.corner_sharpnesses {
            let prop = self.geom_compound.get_or_create_array_child(".cornerSharpnesses", DataType::new(PlainOldDataType::Float32, 1));
            prop.add_array_pod(sharpnesses);
        }
        
        // Holes (optional)
        if let Some(ref holes) = sample.holes {
            let prop = self.geom_compound.get_or_create_array_child(".holes", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(holes);
        }
        
        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            let prop = self.geom_compound.get_or_create_array_child("uv", DataType::new(PlainOldDataType::Float32, 2));
            prop.add_array_pod(uvs);
        }
        if let Some(ref uvi) = sample.uv_indices {
            let prop = self.geom_compound.get_or_create_array_child(".uvIndices", DataType::new(PlainOldDataType::Int32, 1));
            prop.add_array_pod(uvi);
        }
    }
    
    /// Create P property metadata (geoScope=vtx, interpretation=point).
    fn p_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("geoScope", "vtx");
        meta.set("interpretation", "point");
        meta
    }
    
    /// Create bounds metadata.
    fn bnds_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("interpretation", "box");
        meta
    }
    
    /// Compute bounding box from positions.
    fn compute_bounds(positions: &[glam::Vec3]) -> [f64; 6] {
        if positions.is_empty() {
            return [0.0; 6];
        }
        let mut min = glam::DVec3::splat(f64::MAX);
        let mut max = glam::DVec3::splat(f64::MIN);
        for p in positions {
            let p = glam::DVec3::new(p.x as f64, p.y as f64, p.z as f64);
            min = min.min(p);
            max = max.max(p);
        }
        [min.x, min.y, min.z, max.x, max.y, max.z]
    }
    
    /// Get or create array property with metadata.
    fn get_or_create_array_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create scalar property with metadata.
    fn get_or_create_scalar_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
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
    time_sampling_index: u32,
}

impl OCamera {
    /// Create new Camera.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Camera_v1");
        // Camera has no schemaBaseType (empty string in C++)
        meta.set("schemaObjTitle", "AbcGeom_Camera_v1:.geom");
        object.meta_data = meta;
        
        Self {
            object,
            samples: Vec::new(),
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: CameraSample) {
        self.samples.push(sample);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.samples.is_empty() {
            let mut geom = OProperty::compound(".geom");
            // .geom compound metadata (Camera has no schemaBaseType)
            let mut geom_meta = MetaData::new();
            geom_meta.set("schema", "AbcGeom_Camera_v1");
            geom.meta_data = geom_meta;
            
            // Core properties stored as scalar with extent=16 (16 f64s per sample)
            let mut core = OProperty::scalar(".core", DataType::new(PlainOldDataType::Float64, 16));
            core.time_sampling_index = self.time_sampling_index;
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
                core.add_scalar_sample(bytemuck::cast_slice(&props));
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
    time_sampling_index: u32,
}

impl ONuPatch {
    /// Create new NuPatch.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_NuPatch_v2");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_NuPatch_v2:.geom");
        object.meta_data = meta;
        
        // .geom compound with schema metadata
        let mut geom_meta = MetaData::new();
        geom_meta.set("schema", "AbcGeom_NuPatch_v2");
        geom_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut geom = OProperty::compound(".geom");
        geom.meta_data = geom_meta;
        
        Self {
            object,
            geom_compound: geom,
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &ONuPatchSample) {
        // C++ order: P → .selfBnds → nu → nv → ...
        // P is created first, then createSelfBoundsProperty() is called
        
        // Positions (P) with metadata: geoScope=vtx, interpretation=point
        // Created FIRST to match C++ indexed metadata order
        let p_prop = self.get_or_create_array_with_meta("P", 
            DataType::new(PlainOldDataType::Float32, 3), Self::p_meta());
        p_prop.add_array_pod(&sample.positions);
        
        // Self bounds (.selfBnds) - created AFTER P
        let bounds = Self::compute_bounds(&sample.positions);
        let self_bnds_prop = self.get_or_create_scalar_with_meta(".selfBnds",
            DataType::new(PlainOldDataType::Float64, 6), Self::bnds_meta());
        self_bnds_prop.add_scalar_pod(&bounds);
        
        // numU, numV
        let nu_prop = self.geom_compound.get_or_create_scalar_child("nu", DataType::new(PlainOldDataType::Int32, 1));
        nu_prop.add_scalar_pod(&sample.num_u);
        
        let nv_prop = self.geom_compound.get_or_create_scalar_child("nv", DataType::new(PlainOldDataType::Int32, 1));
        nv_prop.add_scalar_pod(&sample.num_v);
        
        // Orders
        let uo_prop = self.geom_compound.get_or_create_scalar_child("uOrder", DataType::new(PlainOldDataType::Int32, 1));
        uo_prop.add_scalar_pod(&sample.u_order);
        
        let vo_prop = self.geom_compound.get_or_create_scalar_child("vOrder", DataType::new(PlainOldDataType::Int32, 1));
        vo_prop.add_scalar_pod(&sample.v_order);
        
        // Knots
        let uk_prop = self.geom_compound.get_or_create_array_child("uKnot", DataType::new(PlainOldDataType::Float32, 1));
        uk_prop.add_array_pod(&sample.u_knot);
        
        let vk_prop = self.geom_compound.get_or_create_array_child("vKnot", DataType::new(PlainOldDataType::Float32, 1));
        vk_prop.add_array_pod(&sample.v_knot);
        
        // Position weights (optional)
        if let Some(ref weights) = sample.position_weights {
            let prop = self.geom_compound.get_or_create_array_child("w", DataType::new(PlainOldDataType::Float32, 1));
            prop.add_array_pod(weights);
        }
        
        // Velocities (optional)
        if let Some(ref vels) = sample.velocities {
            let prop = self.geom_compound.get_or_create_array_child(".velocities", DataType::new(PlainOldDataType::Float32, 3));
            prop.add_array_pod(vels);
        }
        
        // UVs (optional)
        if let Some(ref uvs) = sample.uvs {
            let prop = self.geom_compound.get_or_create_array_child("uv", DataType::new(PlainOldDataType::Float32, 2));
            prop.add_array_pod(uvs);
        }
        
        // Normals (optional)
        if let Some(ref normals) = sample.normals {
            let prop = self.geom_compound.get_or_create_array_child("N", DataType::new(PlainOldDataType::Float32, 3));
            prop.add_array_pod(normals);
        }
    }
    
    /// Create P property metadata.
    fn p_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("geoScope", "vtx");
        meta.set("interpretation", "point");
        meta
    }
    
    /// Create bounds metadata.
    fn bnds_meta() -> MetaData {
        let mut meta = MetaData::new();
        meta.set("interpretation", "box");
        meta
    }
    
    /// Compute bounding box from positions.
    fn compute_bounds(positions: &[glam::Vec3]) -> [f64; 6] {
        if positions.is_empty() {
            return [0.0; 6];
        }
        let mut min = glam::DVec3::splat(f64::MAX);
        let mut max = glam::DVec3::splat(f64::MIN);
        for p in positions {
            let p = glam::DVec3::new(p.x as f64, p.y as f64, p.z as f64);
            min = min.min(p);
            max = max.max(p);
        }
        [min.x, min.y, min.z, max.x, max.y, max.z]
    }
    
    /// Get or create array property with metadata.
    fn get_or_create_array_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::array(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
            children.last_mut().unwrap()
        } else {
            unreachable!()
        }
    }
    
    /// Get or create scalar property with metadata.
    fn get_or_create_scalar_with_meta(&mut self, name: &str, dt: DataType, meta: MetaData) -> &mut OProperty {
        if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
            if let Some(idx) = children.iter().position(|p| p.name == name) {
                return &mut children[idx];
            }
            let mut prop = OProperty::scalar(name, dt);
            prop.meta_data = meta;
            prop.time_sampling_index = self.time_sampling_index;
            children.push(prop);
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
    time_sampling_index: u32,
}

impl OLight {
    /// Create new Light.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_Light_v1");
        // Light has no schemaBaseType (empty string in C++)
        meta.set("schemaObjTitle", "AbcGeom_Light_v1:.geom");
        object.meta_data = meta;
        
        Self {
            object,
            camera_samples: Vec::new(),
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a camera sample (light parameters stored as camera).
    pub fn add_camera_sample(&mut self, sample: CameraSample) {
        self.camera_samples.push(sample);
    }
    
    /// Build the object.
    pub fn build(mut self) -> OObject {
        if !self.camera_samples.is_empty() {
            let mut geom = OProperty::compound(".geom");
            // .geom compound metadata (Light has no schemaBaseType)
            let mut geom_meta = MetaData::new();
            geom_meta.set("schema", "AbcGeom_Light_v1");
            geom.meta_data = geom_meta;
            
            // Camera schema embedded in light
            let mut cam_compound = OProperty::compound(".camera");
            let mut core = OProperty::scalar(".core", DataType::new(PlainOldDataType::Float64, 16));
            core.time_sampling_index = self.time_sampling_index;
            
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
                core.add_scalar_sample(bytemuck::cast_slice(&props));
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
    time_sampling_index: u32,
}

impl OFaceSet {
    /// Create new FaceSet.
    pub fn new(name: &str) -> Self {
        let mut object = OObject::new(name);
        let mut meta = MetaData::new();
        meta.set("schema", "AbcGeom_FaceSet_v1");
        meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        meta.set("schemaObjTitle", "AbcGeom_FaceSet_v1:.faceset");
        object.meta_data = meta;
        
        // .faceset compound with schema metadata (FaceSet uses .faceset, not .geom)
        let mut faceset_meta = MetaData::new();
        faceset_meta.set("schema", "AbcGeom_FaceSet_v1");
        faceset_meta.set("schemaBaseType", "AbcGeom_GeomBase_v1");
        let mut faceset = OProperty::compound(".faceset");
        faceset.meta_data = faceset_meta;
        
        Self {
            object,
            geom_compound: faceset,  // Still named geom_compound for consistency
            time_sampling_index: 0,
        }
    }
    
    /// Set time sampling index for animated properties.
    pub fn set_time_sampling(&mut self, index: u32) {
        self.time_sampling_index = index;
    }
    
    /// Add a sample.
    pub fn add_sample(&mut self, sample: &OFaceSetSample) {
        let faces_prop = self.geom_compound.get_or_create_array_child(".faces", DataType::new(PlainOldDataType::Int32, 1));
        faces_prop.add_array_pod(&sample.faces);
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
        // Material has no schemaBaseType (empty string in C++)
        meta.set("schemaObjTitle", "AbcMaterial_Material_v1:.material");
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
        // .material compound metadata
        let mut mat_meta = MetaData::new();
        mat_meta.set("schema", "AbcMaterial_Material_v1");
        mat.meta_data = mat_meta;
        
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
        // Version is big-endian: {0, 1} = version 1
        assert_eq!(header[VERSION_OFFSET], 0);
        assert_eq!(header[VERSION_OFFSET + 1], 1);

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
