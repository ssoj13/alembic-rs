//! Ogawa archive writer (core write path).
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/AwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/OwData.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/CpwData.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/SpwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp`

use std::collections::HashMap;
use std::path::Path;

use spooky_hash::SpookyHash;

use super::constants::{ALEMBIC_LIBRARY_VERSION, DATA_KEY_SIZE, OGAWA_FILE_VERSION};
use super::object::OObject;
use super::property::{OProperty, OPropertyData};
use super::stream::OStream;
use super::write_util::{
    encode_sample_for_pod, format_alembic_version, hash_dimensions, hash_property_header,
    pod_seed, pod_to_u8, write_with_hint,
};
use crate::core::{ArraySampleContentKey, MetaData, TimeSampling, TimeSamplingType};
use crate::ogawa::format::*;
use crate::util::{Error, PlainOldDataType, Result};

/// Deferred group for bottom-up writing.
/// Matches C++ OGroup freeze behavior.
#[derive(Debug)]
struct DeferredGroup {
    /// Children of this group (data positions have MSB set, group indices don't).
    children: Vec<u64>,
    /// Final position after writing (set during flush).
    final_pos: Option<u64>,
}

impl DeferredGroup {
    fn new(children: Vec<u64>) -> Self {
        Self { children, final_pos: None }
    }
}

/// Context for computing object headers inside write_properties.
///
/// Object headers depend on data_hash which is computed during property writing,
/// so we need to pass the context to compute headers at the right moment.
struct ObjectHeadersContext<'a> {
    children: &'a [OObject],
    child_hash1: u64,
    child_hash2: u64,
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
    /// Deduplication map: content key -> file position.
    dedup_map: HashMap<ArraySampleContentKey, u64>,
    /// Enable/disable deduplication (enabled by default).
    dedup_enabled: bool,
    /// Deferred groups for bottom-up writing.
    deferred_groups: Vec<DeferredGroup>,
    /// Use deferred group writing (C++ compatible mode).
    deferred_mode: bool,
    /// Library version to write (default: Alembic 1.8.x).
    library_version: i32,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let name = path.as_ref().to_string_lossy().to_string();
        let mut stream = OStream::create(&path)?;

        // Header with placeholder for root position.
        stream.write_bytes(OGAWA_MAGIC)?;
        stream.write_u8(NOT_FROZEN_FLAG)?;
        // Version as big-endian (matching C++ Alembic format: {0, 1} = version 1)
        stream.write_bytes(&CURRENT_VERSION.to_be_bytes())?;
        stream.write_u64(0)?; // Root position placeholder.

        // Default identity time sampling at index 0.
        let identity_ts = TimeSampling::identity();

        Ok(Self {
            name,
            stream,
            frozen: false,
            time_samplings: vec![identity_ts],
            max_samples: vec![0],
            indexed_metadata: vec![MetaData::new()], // Index 0 is always empty.
            metadata_map: HashMap::new(),
            archive_metadata: MetaData::new(),
            application_writer: String::new(), // Empty by default for C++ parity.
            compression_hint: -1,
            dedup_map: HashMap::new(),
            dedup_enabled: true,
            deferred_groups: Vec::new(),
            deferred_mode: false, // Disabled for binary parity - write groups inline.
            library_version: ALEMBIC_LIBRARY_VERSION,
        })
    }

    /// Enable or disable deduplication.
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

    /// Set the library version to write (for copying archives).
    pub fn set_library_version(&mut self, version: i32) {
        self.library_version = version;
    }

    /// Get the library version.
    pub fn library_version(&self) -> i32 {
        self.library_version
    }

    /// Set the archive metadata (e.g. when copying from another file).
    /// This also clears the application_writer so we don't add our own app name.
    pub fn set_archive_metadata(&mut self, md: MetaData) {
        self.archive_metadata = md;
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

    /// Set the DCC name (stored as _ai_DCC_Name in metadata).
    pub fn setDccName(&mut self, name: &str) {
        self.archive_metadata.set("_ai_DCC_Name", name);
    }

    /// Set the DCC version (stored as _ai_DCC_Version in metadata).
    pub fn setDccVersion(&mut self, version: &str) {
        self.archive_metadata.set("_ai_DCC_Version", version);
    }

    /// Set the DCC FPS (stored as _ai_DCC_FPS in metadata).
    pub fn setDccFps(&mut self, fps: f64) {
        self.archive_metadata.set("_ai_DCC_FPS", fps.to_string());
    }

    /// Add a time sampling and return its index.
    pub fn addTimeSampling(&mut self, ts: TimeSampling) -> u32 {
        for (i, existing) in self.time_samplings.iter().enumerate() {
            if existing.is_equivalent(&ts) {
                return i as u32;
            }
        }
        let index = self.time_samplings.len() as u32;
        self.time_samplings.push(ts);
        self.max_samples.push(0);
        index
    }

    /// Update max samples for a time sampling.
    pub fn update_max_samples(&mut self, ts_index: u32, num_samples: u32) {
        if let Some(max) = self.max_samples.get_mut(ts_index as usize) {
            let old = *max;
            *max = (*max).max(num_samples);
            if *max != old {
                eprintln!(
                    "[DEBUG] update_max_samples(ts_index={}, num_samples={}) -> max changed {} -> {}",
                    ts_index, num_samples, old, *max
                );
            }
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

        if serialized.is_empty() {
            return 0;
        }

        if let Some(&idx) = self.metadata_map.get(&serialized) {
            // DEBUG: Found existing metadata
            eprintln!("  [add_indexed_metadata] FOUND idx={} for '{}'", idx, serialized);
            return idx as u8;
        }
        
        // DEBUG: Not found, will add new
        eprintln!("  [add_indexed_metadata] NOT FOUND, adding new for '{}'", serialized);
        eprintln!("    Available in map:");
        for (k, v) in &self.metadata_map {
            eprintln!("      [{:3}] '{}'", v, k);
        }

        // max 254 entries + empty (index 0) => len < 255
        if self.indexed_metadata.len() >= 255 || serialized.len() > 255 {
            return 0xff;
        }

        let idx = self.indexed_metadata.len();
        self.indexed_metadata.push(md.clone());
        self.metadata_map.insert(serialized, idx);
        idx as u8
    }

    /// Set indexed metadata from source archive (for copying).
    pub fn set_indexed_metadata(&mut self, metadata: &[MetaData]) {
        self.indexed_metadata.clear();
        self.metadata_map.clear();

        self.indexed_metadata.push(MetaData::new());

        for (i, md) in metadata.iter().enumerate().skip(1) {
            self.indexed_metadata.push(md.clone());
            let serialized = md.serialize();
            if !serialized.is_empty() {
                self.metadata_map.insert(serialized, i);
            }
        }
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
            return Ok(0);
        }

        let pos = self.stream.pos();
        eprintln!("  [write_data] pos=0x{:04x} ({}) len={}", pos, pos, data.len());
        self.stream.write_u64(data.len() as u64)?;
        self.stream.write_bytes(data)?;
        Ok(pos)
    }

    /// Write data with 16-byte key prefix and deduplication.
    pub fn write_keyed_data(&mut self, data: &[u8], pod: PlainOldDataType) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        let encoded = encode_sample_for_pod(data, pod);
        let seed = pod_seed(pod);
        let content_key = ArraySampleContentKey::from_data(&encoded, seed);

        if self.dedup_enabled {
            if let Some(&existing_pos) = self.dedup_map.get(&content_key) {
                return Ok(existing_pos);
            }
        }

        let pos = self.stream.pos();
        eprintln!("  [write_keyed_data] pos=0x{:04x} ({}) len={}", pos, pos, encoded.len());
        let total_size = DATA_KEY_SIZE + encoded.len();
        self.stream.write_u64(total_size as u64)?;

        self.stream.write_bytes(content_key.digest())?;
        self.stream.write_bytes(&encoded)?;

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
            return Ok(0);
        }

        let pos = self.stream.pos();
        eprintln!("  [write_group] pos=0x{:04x} ({}) children={:?}", pos, pos, children);
        self.stream.write_u64(children.len() as u64)?;
        for &child in children {
            self.stream.write_u64(child)?;
        }
        Ok(pos)
    }

    // -------------------------------------------------------------------------
    // Deferred Group Writing (C++ compatible mode)
    // -------------------------------------------------------------------------

    /// Marker constant for deferred group placeholders.
    const DEFERRED_GROUP_MARKER: u64 = 0x4000_0000_0000_0000; // Bit 62 set.

    /// Add a deferred group and return a placeholder reference.
    fn add_deferred_group(&mut self, children: Vec<u64>) -> u64 {
        if children.is_empty() {
            return 0;
        }
        let idx = self.deferred_groups.len();
        self.deferred_groups.push(DeferredGroup::new(children));
        Self::DEFERRED_GROUP_MARKER | (idx as u64)
    }

    /// Check if a value is a deferred group placeholder.
    #[inline]
    fn is_deferred_placeholder(value: u64) -> bool {
        (value & Self::DEFERRED_GROUP_MARKER) != 0 && !is_data_offset(value)
    }

    /// Extract deferred group index from placeholder.
    #[inline]
    fn deferred_group_index(placeholder: u64) -> usize {
        (placeholder & !Self::DEFERRED_GROUP_MARKER) as usize
    }

    /// Flush all deferred groups, writing them in bottom-up order.
    fn flush_deferred_groups(&mut self) -> Result<u64> {
        if self.deferred_groups.is_empty() {
            return Ok(0);
        }

        let mut group_deps: Vec<Vec<usize>> = vec![Vec::new(); self.deferred_groups.len()];

        for (i, group) in self.deferred_groups.iter().enumerate() {
            for &child in &group.children {
                if Self::is_deferred_placeholder(child) {
                    let child_idx = Self::deferred_group_index(child);
                    group_deps[i].push(child_idx);
                }
            }
        }

        let mut written: Vec<bool> = vec![false; self.deferred_groups.len()];
        let mut order: Vec<usize> = Vec::with_capacity(self.deferred_groups.len());

        while order.len() < self.deferred_groups.len() {
            let mut found = false;
            for i in 0..self.deferred_groups.len() {
                if written[i] {
                    continue;
                }
                let deps_ok = group_deps[i].iter().all(|&d| written[d]);
                if deps_ok {
                    order.push(i);
                    written[i] = true;
                    found = true;
                }
            }
            if !found {
                return Err(Error::invalid("Circular dependency in deferred groups"));
            }
        }

        let mut last_pos = 0u64;
        for &idx in &order {
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

    // -------------------------------------------------------------------------
    // Serialization helpers
    // -------------------------------------------------------------------------

    fn serialize_time_samplings(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        for (i, ts) in self.time_samplings.iter().enumerate() {
            let max_sample = self.max_samples.get(i).copied().unwrap_or(0);
            buf.extend_from_slice(&max_sample.to_le_bytes());

            let (tpc, samples): (f64, Vec<f64>) = match &ts.sampling_type {
                TimeSamplingType::Identity => (1.0, vec![0.0]),
                TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                    (*time_per_cycle, vec![*start_time])
                }
                TimeSamplingType::Cyclic { time_per_cycle, times } => {
                    (*time_per_cycle, times.clone())
                }
                TimeSamplingType::Acyclic { times } => (super::constants::ACYCLIC_TIME_PER_CYCLE, times.clone()),
            };

            buf.extend_from_slice(&tpc.to_le_bytes());
            buf.extend_from_slice(&(samples.len() as u32).to_le_bytes());
            for sample in samples {
                buf.extend_from_slice(&sample.to_le_bytes());
            }
        }

        buf
    }

    fn serialize_indexed_metadata(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for md in self.indexed_metadata.iter().skip(1) {
            let serialized = md.serialize();
            buf.push(serialized.len() as u8);
            buf.extend_from_slice(serialized.as_bytes());
        }
        buf
    }

    // -------------------------------------------------------------------------
    // Top-level write
    // -------------------------------------------------------------------------

    /// Write the complete archive with given root object.
    pub fn write_archive(&mut self, root: &OObject) -> Result<()> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        let version_pos = self.write_data(&OGAWA_FILE_VERSION.to_le_bytes())?;
        let file_version_pos = self.write_data(&self.library_version.to_le_bytes())?;
        let (root_obj_pos, _, _) = self.write_object(root, "/")?;

        let final_root_obj_pos = if self.deferred_mode {
            if Self::is_deferred_placeholder(root_obj_pos) {
                self.flush_deferred_groups()?;
                let idx = Self::deferred_group_index(root_obj_pos);
                self.deferred_groups[idx]
                    .final_pos
                    .ok_or_else(|| Error::invalid("Root object group not written"))?
            } else {
                root_obj_pos
            }
        } else {
            root_obj_pos
        };

        // Archive metadata (includes application writer and alembic version).
        let mut archive_meta = self.archive_metadata.clone();
        if archive_meta.get("_ai_Application").is_none() && !self.application_writer.is_empty() {
            archive_meta.set("_ai_Application", &self.application_writer);
        }
        if archive_meta.get("_ai_AlembicVersion").is_none() {
            let version = format_alembic_version(self.library_version);
            archive_meta.set("_ai_AlembicVersion", &version);
        }
        let archive_meta_str = archive_meta.serialize();
        let archive_meta_pos = if archive_meta_str.is_empty() {
            0
        } else {
            self.write_data(archive_meta_str.as_bytes())?
        };

        let ts_data = self.serialize_time_samplings();
        let ts_pos = self.write_data(&ts_data)?;

        let idx_meta_data = self.serialize_indexed_metadata();
        let idx_meta_pos = if idx_meta_data.is_empty() {
            0
        } else {
            self.write_data(&idx_meta_data)?
        };

        let root_children = vec![
            make_data_offset(version_pos),
            make_data_offset(file_version_pos),
            make_group_offset(final_root_obj_pos),
            make_data_offset(archive_meta_pos),
            make_data_offset(ts_pos),
            make_data_offset(idx_meta_pos),
        ];

        let root_pos = self.write_group(&root_children)?;

        self.frozen = true;

        self.stream.seek(FROZEN_OFFSET as u64)?;
        self.stream.write_u8(FROZEN_FLAG)?;
        self.stream.seek(ROOT_POS_OFFSET as u64)?;
        self.stream.write_u64(root_pos)?;

        self.stream.seek_end()?;
        self.stream.flush()?;

        Ok(())
    }

    /// Write an object and return (position, hash1, hash2).
    fn write_object(&mut self, obj: &OObject, parent_path: &str) -> Result<(u64, u64, u64)> {
        let full_path = if parent_path == "/" {
            format!("/{}", obj.name)
        } else {
            format!("{}/{}", parent_path, obj.name)
        };

        let mut child_positions = Vec::new();
        let mut child_hashes: Vec<u64> = Vec::new();
        for child in &obj.children {
            let (child_pos, h1, h2) = self.write_object(child, &full_path)?;
            child_positions.push(child_pos);
            child_hashes.push(h1);
            child_hashes.push(h2);
        }

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

        let obj_ctx = ObjectHeadersContext { children: &obj.children, child_hash1, child_hash2 };
        let (props_pos, data_hash1, data_hash2, headers_pos, _) =
            self.write_properties_with_object_headers(&obj.properties, Some(obj_ctx))?;

        let mut children = Vec::new();
        if Self::is_deferred_placeholder(props_pos) {
            children.push(props_pos);
        } else {
            children.push(make_group_offset(props_pos));
        }

        for pos in child_positions {
            if Self::is_deferred_placeholder(pos) {
                children.push(pos);
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

        let mut combined_hash = SpookyHash::new(0, 0);
        if !child_hashes.is_empty() {
            let child_hash_bytes: Vec<u8> = child_hashes.iter()
                .flat_map(|h| h.to_le_bytes())
                .collect();
            combined_hash.update(&child_hash_bytes);
        }
        combined_hash.update(&data_hash1.to_le_bytes());
        combined_hash.update(&data_hash2.to_le_bytes());

        let meta_str = obj.meta_data.serialize();
        if !meta_str.is_empty() {
            combined_hash.update(meta_str.as_bytes());
        }

        combined_hash.update(obj.name.as_bytes());
        let (final_h1, final_h2) = combined_hash.finalize();

        Ok((pos, final_h1, final_h2))
    }

    // -------------------------------------------------------------------------
    // Property writing
    // -------------------------------------------------------------------------

    fn write_properties(&mut self, props: &[OProperty]) -> Result<u64> {
        let (pos, _, _, _) = self.write_properties_with_data(props)?;
        Ok(pos)
    }

    fn write_properties_with_data(&mut self, props: &[OProperty]) -> Result<(u64, u64, u64, Vec<u64>)> {
        let (pos, h1, h2, _, raw_hashes) = self.write_properties_with_object_headers(props, None)?;
        Ok((pos, h1, h2, raw_hashes))
    }

    fn write_properties_with_object_headers(
        &mut self,
        props: &[OProperty],
        obj_ctx: Option<ObjectHeadersContext<'_>>,
    ) -> Result<(u64, u64, u64, u64, Vec<u64>)> {
        if props.is_empty() {
            let hasher = SpookyHash::new(0, 0);
            let (h1, h2) = hasher.finalize();

            let obj_headers_pos = if let Some(ctx) = obj_ctx {
                let obj_headers = self.serialize_object_headers_with_hash(
                    ctx.children,
                    h1,
                    h2,
                    ctx.child_hash1,
                    ctx.child_hash2,
                );
                self.write_data(&obj_headers)?
            } else {
                0
            };

            let pos = self.write_group(&[])?;
            return Ok((pos, h1, h2, obj_headers_pos, Vec::new()));
        }

        let mut sorted_indices: Vec<usize> = (0..props.len()).collect();
        sorted_indices.sort_by_key(|&i| (props[i].data_write_order, i));

        let mut prop_states: Vec<(Vec<u64>, Option<(u64, u64)>)> = vec![(Vec::new(), None); props.len()];
        for &idx in &sorted_indices {
            let state = self.collect_property_sample_data(&props[idx])?;
            prop_states[idx] = state;
        }

        let mut prop_positions = vec![0u64; props.len()];
        let mut prop_hashes_pairs = vec![(0u64, 0u64); props.len()];

        for idx in (0..props.len()).rev() {
            let (children, sample_hash) = std::mem::take(&mut prop_states[idx]);
            let (pos, h1, h2) = self.finalize_property_group(&props[idx], children, sample_hash)?;
            prop_positions[idx] = pos;
            prop_hashes_pairs[idx] = (h1, h2);
        }

        let mut prop_hashes: Vec<u64> = Vec::new();
        for (h1, h2) in &prop_hashes_pairs {
            prop_hashes.push(*h1);
            prop_hashes.push(*h2);
        }

        let (data_h1, data_h2) = {
            let hash_bytes: Vec<u8> = prop_hashes.iter().flat_map(|h| h.to_le_bytes()).collect();
            let mut hasher = SpookyHash::new(0, 0);
            hasher.update(&hash_bytes);
            hasher.finalize()
        };

        let obj_headers_pos = if let Some(ctx) = obj_ctx {
            let obj_headers = self.serialize_object_headers_with_hash(
                ctx.children,
                data_h1,
                data_h2,
                ctx.child_hash1,
                ctx.child_hash2,
            );
            self.write_data(&obj_headers)?
        } else {
            0
        };

        let headers_data = self.serialize_property_headers(props);
        let headers_pos = self.write_data(&headers_data)?;

        let mut children = Vec::new();
        for pos in prop_positions {
            if Self::is_deferred_placeholder(pos) {
                children.push(pos);
            } else {
                children.push(make_group_offset(pos));
            }
        }
        children.push(make_data_offset(headers_pos));

        // Property compound groups must be written inline to match destructor order.
        let pos = self.write_group(&children)?;

        Ok((pos, data_h1, data_h2, obj_headers_pos, prop_hashes))
    }

    fn collect_property_sample_data(
        &mut self,
        prop: &OProperty,
    ) -> Result<(Vec<u64>, Option<(u64, u64)>)> {
        let ts_idx = prop.time_sampling_index;

        match &prop.data {
            OPropertyData::Scalar(samples) => {
                self.update_max_samples(ts_idx, samples.len() as u32);

                let mut sample_hash: Option<(u64, u64)> = None;
                let mut children = Vec::new();

                for sample in samples {
                    let encoded = encode_sample_for_pod(&sample.data, prop.data_type.pod);
                    
                    // Use pre-computed digest if available, otherwise compute
                    let digest: [u8; 16] = if let Some(d) = &sample.digest {
                        *d
                    } else {
                        let seed = pod_seed(prop.data_type.pod);
                        let content_key = ArraySampleContentKey::from_data(&encoded, seed);
                        *content_key.digest()
                    };
                    
                    let d0 = u64::from_le_bytes(digest[0..8].try_into().unwrap());
                    let d1 = u64::from_le_bytes(digest[8..16].try_into().unwrap());

                    sample_hash = match sample_hash {
                        None => Some((d0, d1)),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d0, d1)),
                    };

                    // Use write_keyed_data_with_key for pre-computed digests
                    let pos = if sample.digest.is_some() {
                        self.write_keyed_data_with_key(&encoded, &digest)?
                    } else {
                        self.write_keyed_data(&encoded, prop.data_type.pod)?
                    };
                    children.push(make_data_offset(pos));
                }
                Ok((children, sample_hash))
            }
            OPropertyData::Array(samples) => {
                self.update_max_samples(ts_idx, samples.len() as u32);

                let mut sample_hash: Option<(u64, u64)> = None;
                let mut children = Vec::new();

                for sample in samples {
                    let encoded = encode_sample_for_pod(&sample.data, prop.data_type.pod);
                    
                    // Use pre-computed digest if available, otherwise compute
                    let digest: [u8; 16] = if let Some(d) = &sample.digest {
                        *d
                    } else {
                        let seed = pod_seed(prop.data_type.pod);
                        let content_key = ArraySampleContentKey::from_data(&encoded, seed);
                        *content_key.digest()
                    };
                    
                    let mut d = (
                        u64::from_le_bytes(digest[0..8].try_into().unwrap()),
                        u64::from_le_bytes(digest[8..16].try_into().unwrap()),
                    );
                    hash_dimensions(&sample.dims, &mut d);

                    sample_hash = match sample_hash {
                        None => Some(d),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d.0, d.1)),
                    };

                    // Use write_keyed_data_with_key for pre-computed digests
                    let data_pos = if sample.digest.is_some() {
                        self.write_keyed_data_with_key(&encoded, &digest)?
                    } else {
                        self.write_keyed_data(&encoded, prop.data_type.pod)?
                    };

                    let dims_offset = if sample.dims.len() <= 1
                        && !matches!(prop.data_type.pod, PlainOldDataType::String | PlainOldDataType::Wstring)
                    {
                        EMPTY_DATA
                    } else {
                        let dims_data: Vec<u8> = sample.dims
                            .iter()
                            .flat_map(|dim| (*dim as u64).to_le_bytes())
                            .collect();
                        make_data_offset(self.write_data(&dims_data)?)
                    };

                    children.push(make_data_offset(data_pos));
                    children.push(dims_offset);
                }
                Ok((children, sample_hash))
            }
            OPropertyData::Compound(_) => Ok((Vec::new(), None)),
        }
    }

    fn finalize_property_group(
        &mut self,
        prop: &OProperty,
        children: Vec<u64>,
        sample_hash: Option<(u64, u64)>,
    ) -> Result<(u64, u64, u64)> {
        let ts_idx = prop.time_sampling_index;
        let time_sampling = self.time_samplings.get(ts_idx as usize)
            .cloned()
            .unwrap_or_else(TimeSampling::identity);

        match &prop.data {
            OPropertyData::Scalar(_) | OPropertyData::Array(_) => {
                let pos = self.write_group(&children)?;

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
                let (pos, _, _, raw_prop_hashes) = self.write_properties_with_data(sub_props)?;

                let mut hasher = SpookyHash::new(0, 0);
                let hash_bytes: Vec<u8> = raw_prop_hashes.iter().flat_map(|h| h.to_le_bytes()).collect();
                hasher.update(&hash_bytes);
                hash_property_header(&mut hasher, prop, &time_sampling);

                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
        }
    }

    /// Serialize object headers for children with 32-byte SpookyHash suffix.
    fn serialize_object_headers_with_hash(
        &mut self,
        children: &[OObject],
        data_hash1: u64,
        data_hash2: u64,
        child_hash1: u64,
        child_hash2: u64,
    ) -> Vec<u8> {
        let mut buf = Vec::new();

        for child in children {
            let name_bytes = child.name.as_bytes();
            write_with_hint(&mut buf, name_bytes.len() as u32, 2);
            buf.extend_from_slice(name_bytes);

            let meta_idx = self.add_indexed_metadata(&child.meta_data);
            eprintln!("  [serialize_object_headers] child='{}' meta_idx={} meta='{}'", 
                      child.name, meta_idx, child.meta_data.serialize());
            write_with_hint(&mut buf, meta_idx as u32, 0);

            if meta_idx == 0xff {
                let meta_str = child.meta_data.serialize();
                write_with_hint(&mut buf, meta_str.len() as u32, 2);
                buf.extend_from_slice(meta_str.as_bytes());
            }
        }

        buf.extend_from_slice(&data_hash1.to_le_bytes());
        buf.extend_from_slice(&data_hash2.to_le_bytes());
        buf.extend_from_slice(&child_hash1.to_le_bytes());
        buf.extend_from_slice(&child_hash2.to_le_bytes());

        buf
    }

    fn serialize_property_headers(&mut self, props: &[OProperty]) -> Vec<u8> {
        let mut buf = Vec::new();

        for prop in props {
            let info = self.build_property_info(prop);
            buf.extend_from_slice(&info.to_le_bytes());

            let size_hint = ((info >> 2) & 0x03) as u8;

            if !matches!(prop.data, OPropertyData::Compound(_)) {
                let num_samples = prop.getNumSamples() as u32;
                write_with_hint(&mut buf, num_samples, size_hint);

                if (info & 0x0200) != 0 {
                    write_with_hint(&mut buf, prop.first_changed_index, size_hint);
                    write_with_hint(&mut buf, prop.last_changed_index, size_hint);
                }

                if (info & 0x0100) != 0 {
                    write_with_hint(&mut buf, prop.time_sampling_index, size_hint);
                }
            }

            let name_bytes = prop.name.as_bytes();
            write_with_hint(&mut buf, name_bytes.len() as u32, size_hint);
            buf.extend_from_slice(name_bytes);

            let meta_idx = self.add_indexed_metadata(&prop.meta_data);
            if meta_idx == 0xff {
                let meta_str = prop.meta_data.serialize();
                write_with_hint(&mut buf, meta_str.len() as u32, size_hint);
                buf.extend_from_slice(meta_str.as_bytes());
            }
        }

        buf
    }

    fn build_property_info(&mut self, prop: &OProperty) -> u32 {
        let mut info: u32 = 0;

        let name_size = prop.name.len() as u32;
        let meta_data_size = prop.meta_data.serialize().len() as u32;
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

        info |= (size_hint & 0x03) << 2;

        match &prop.data {
            OPropertyData::Compound(_) => {
                info |= 0;
            }
            OPropertyData::Scalar(_) => {
                info |= 1;
            }
            OPropertyData::Array(_) => {
                if prop.is_scalar_like {
                    info |= 3;
                } else {
                    info |= 2;
                }
            }
        }

        if !matches!(prop.data, OPropertyData::Compound(_)) {
            let pod = pod_to_u8(prop.data_type.pod) as u32;
            info |= (pod & 0x0f) << 4;

            info |= (prop.data_type.extent as u32 & 0xff) << 12;

            let is_homogenous = match &prop.data {
                OPropertyData::Array(samples) => {
                    if samples.is_empty() {
                        true
                    } else {
                        let first = samples[0].dims.iter().product::<usize>();
                        samples.iter().all(|s| s.dims.iter().product::<usize>() == first)
                    }
                }
                _ => true,
            };
            if is_homogenous {
                info |= 0x400;
            }

            if prop.time_sampling_index != 0 {
                info |= 0x0100;
            }

            let num_samples = prop.getNumSamples() as u32;
            if prop.first_changed_index == 0 && prop.last_changed_index == 0 {
                info |= 0x800;
            } else if prop.first_changed_index != 1 || prop.last_changed_index != num_samples.saturating_sub(1) {
                info |= 0x0200;
            }
        }

        let meta_idx = self.add_indexed_metadata(&prop.meta_data);
        info |= (meta_idx as u32) << 20;
        
        eprintln!("  [build_property_info] prop='{}' info=0x{:08x} meta_idx={} (in bits 20-27: {})",
                  prop.name, info, meta_idx, (info >> 20) & 0xff);

        info
    }

    /// Finalize and close the archive.
    pub fn close(mut self) -> Result<()> {
        if !self.frozen {
            let empty_root = OObject::new("");
            self.write_archive(&empty_root)?;
        }
        self.stream.flush()?;
        Ok(())
    }
}
