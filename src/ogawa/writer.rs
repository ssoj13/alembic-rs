//! Ogawa format writer implementation.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};

use super::format::*;
use crate::util::{Error, Result};

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

/// Ogawa archive writer.
pub struct OArchive {
    stream: OStream,
    root_pos: u64,
    frozen: bool,
}

impl OArchive {
    /// Create a new Alembic file for writing.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let mut stream = OStream::create(path)?;

        // Write header with placeholder for root position
        stream.write_bytes(OGAWA_MAGIC)?;
        stream.write_u8(NOT_FROZEN_FLAG)?;
        stream.write_u16(CURRENT_VERSION)?;
        stream.write_u64(0)?; // Root position placeholder

        Ok(Self {
            stream,
            root_pos: 0,
            frozen: false,
        })
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

    /// Write data and return its position.
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

    /// Create a new group writer.
    pub fn group(&mut self) -> OGroup {
        OGroup::new()
    }

    /// Write a group and return its position.
    pub fn write_group(&mut self, group: &OGroup) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        if group.children.is_empty() {
            return Ok(0); // Empty group marker
        }

        let pos = self.stream.pos();

        // Write number of children
        self.stream.write_u64(group.children.len() as u64)?;

        // Write child offsets
        for &child_offset in &group.children {
            self.stream.write_u64(child_offset)?;
        }

        Ok(pos)
    }

    /// Set the root group and freeze the archive.
    pub fn set_root(&mut self, root_pos: u64) -> Result<()> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        self.root_pos = root_pos;
        self.frozen = true;

        // Seek back to header and update root position and frozen flag
        self.stream.seek(FROZEN_OFFSET as u64)?;
        self.stream.write_u8(FROZEN_FLAG)?;
        // Skip version (already at position 7)
        self.stream.seek(ROOT_POS_OFFSET as u64)?;
        self.stream.write_u64(root_pos)?;

        // Seek to end
        self.stream.seek_end()?;
        self.stream.flush()?;

        Ok(())
    }

    /// Finalize and close the archive.
    pub fn close(mut self) -> Result<()> {
        if !self.frozen {
            // If no root was set, create an empty root
            let empty_group = self.group();
            let root_pos = self.write_group(&empty_group)?;
            self.set_root(root_pos)?;
        }
        self.stream.flush()?;
        Ok(())
    }
}

/// Group builder for writing.
#[derive(Default)]
pub struct OGroup {
    children: Vec<u64>,
}

impl OGroup {
    /// Create a new empty group.
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    /// Add a child data reference.
    pub fn add_data(&mut self, pos: u64) {
        self.children.push(make_data_offset(pos));
    }

    /// Add a child group reference.
    pub fn add_group(&mut self, pos: u64) {
        self.children.push(make_group_offset(pos));
    }

    /// Get the number of children.
    pub fn num_children(&self) -> usize {
        self.children.len()
    }

    /// Check if this group is empty.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Clear all children.
    pub fn clear(&mut self) {
        self.children.clear();
    }
}

/// Data builder for convenient data writing.
pub struct OData {
    buffer: Vec<u8>,
}

impl OData {
    /// Create a new empty data builder.
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Create from existing bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self { buffer: bytes.into() }
    }

    /// Write bytes to the buffer.
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Write a u64 value.
    pub fn write_u64(&mut self, value: u64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a u32 value.
    pub fn write_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a u16 value.
    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a u8 value.
    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    /// Write a string with null terminator.
    pub fn write_string(&mut self, s: &str) {
        self.buffer.extend_from_slice(s.as_bytes());
        self.buffer.push(0);
    }

    /// Write an f32 value.
    pub fn write_f32(&mut self, value: f32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Write an f64 value.
    pub fn write_f64(&mut self, value: f64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Get the buffer length.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get the buffer as a slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume and return the buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }
}

impl Default for OData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_empty_archive() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let path = temp.path();

        // Create and close archive
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
    fn test_write_data() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let path = temp.path();

        let mut archive = OArchive::create(path)?;

        // Write some data
        let data_pos = archive.write_data(b"Hello, Alembic!")?;
        assert!(data_pos >= HEADER_SIZE as u64);

        // Create root group with the data
        let mut root = archive.group();
        root.add_data(data_pos);
        let root_pos = archive.write_group(&root)?;

        archive.set_root(root_pos)?;
        archive.close()?;

        // Read back and verify
        let reader = super::super::IArchive::open(path)?;
        assert!(reader.is_valid());
        assert!(reader.is_frozen());
        assert_eq!(reader.root().num_children(), 1);

        Ok(())
    }

    #[test]
    fn test_odata_builder() {
        let mut data = OData::new();
        data.write_u32(42);
        data.write_f32(3.14);
        data.write_string("test");

        assert_eq!(data.len(), 4 + 4 + 5); // u32 + f32 + "test\0"
    }
}
