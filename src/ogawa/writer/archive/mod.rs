//! Ogawa archive writer (core write path).
//!
//! This module is split into focused submodules to keep the write pipeline
//! debuggable and aligned with C++ AbcCoreOgawa.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/AwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/OwData.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/CpwData.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/SpwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp`

mod data;
mod deferred;
mod metadata;
mod objects;
mod properties;
mod types;

use std::collections::HashMap;
use std::path::Path;

use super::constants::{ALEMBIC_LIBRARY_VERSION, OGAWA_FILE_VERSION};
use super::object::OObject;
use super::stream::OStream;
use super::write_util::format_alembic_version;
use crate::core::{ArraySampleContentKey, MetaData, TimeSampling};
use crate::ogawa::format::*;
use crate::util::{Error, Result};
use types::DeferredGroup;

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
    /// Preserve existing archive metadata keys when copying.
    preserve_archive_metadata: bool,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    ///
    /// Mirrors `AwImpl::init()` header setup.
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
            preserve_archive_metadata: false,
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
        self.preserve_archive_metadata = true;
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

    /// Write the complete archive with given root object.
    ///
    /// Mirrors `AwImpl::init()` ordering and metadata placement.
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
        if !self.preserve_archive_metadata
            || archive_meta.get("_ai_AlembicVersion").is_none()
        {
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
