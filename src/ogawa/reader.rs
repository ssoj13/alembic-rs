//! Ogawa format reader implementation.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

// use byteorder::{LittleEndian, ReadBytesExt};
use memmap2::Mmap;
use parking_lot::RwLock;

use super::format::*;
use crate::util::{Error, Result};

/// Input streams for reading Ogawa data.
/// Supports both memory-mapped and buffered I/O modes.
pub struct IStreams {
    inner: StreamsInner,
    version: u16,
    frozen: bool,
    size: u64,
}

enum StreamsInner {
    /// Memory-mapped file (preferred for large files)
    Mmap(Mmap),
    /// Buffered file access (fallback)
    File(Arc<RwLock<File>>),
}

impl IStreams {
    /// Open a file for reading with memory mapping.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_opts(path, true)
    }

    /// Open a file with optional memory mapping.
    pub fn open_opts(path: impl AsRef<Path>, use_mmap: bool) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::FileNotFound(path.to_path_buf())
            } else {
                Error::Io(e)
            }
        })?;

        let metadata = file.metadata()?;
        let size = metadata.len();

        if size < HEADER_SIZE as u64 {
            return Err(Error::UnexpectedEof(size));
        }

        let inner = if use_mmap && size > 0 {
            // Safety: File is opened read-only and we handle potential issues
            let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
                Error::MmapFailed(e.to_string())
            })?;
            StreamsInner::Mmap(mmap)
        } else {
            StreamsInner::File(Arc::new(RwLock::new(file)))
        };

        // Read and validate header
        let (version, frozen) = match &inner {
            StreamsInner::Mmap(mmap) => Self::parse_header(mmap)?,
            StreamsInner::File(file) => {
                let mut f = file.write();
                let mut header = [0u8; HEADER_SIZE];
                f.seek(SeekFrom::Start(0))?;
                f.read_exact(&mut header)?;
                Self::parse_header(&header)?
            }
        };

        Ok(Self { inner, version, frozen, size })
    }

    /// Parse and validate the Ogawa header.
    fn parse_header(data: &[u8]) -> Result<(u16, bool)> {
        if data.len() < HEADER_SIZE {
            return Err(Error::UnexpectedEof(data.len() as u64));
        }

        // Check magic bytes
        if &data[0..5] != OGAWA_MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Read frozen flag
        let frozen = data[FROZEN_OFFSET] == FROZEN_FLAG;

        // Read version (little-endian u16)
        let version = u16::from_le_bytes([data[VERSION_OFFSET], data[VERSION_OFFSET + 1]]);

        Ok((version, frozen))
    }

    /// Check if the file is valid.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.size >= HEADER_SIZE as u64
    }

    /// Check if the archive is frozen (finalized).
    #[inline]
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Get the format version.
    #[inline]
    pub fn version(&self) -> u16 {
        self.version
    }

    /// Get the total file size.
    #[inline]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get the root group position from the header.
    pub fn root_pos(&self) -> Result<u64> {
        self.read_u64(ROOT_POS_OFFSET as u64)
    }

    /// Read bytes at a specific position.
    pub fn read_bytes(&self, pos: u64, len: usize) -> Result<Vec<u8>> {
        if pos + len as u64 > self.size {
            return Err(Error::UnexpectedEof(pos + len as u64));
        }

        match &self.inner {
            StreamsInner::Mmap(mmap) => {
                Ok(mmap[pos as usize..(pos as usize + len)].to_vec())
            }
            StreamsInner::File(file) => {
                let mut f = file.write();
                f.seek(SeekFrom::Start(pos))?;
                let mut buf = vec![0u8; len];
                f.read_exact(&mut buf)?;
                Ok(buf)
            }
        }
    }

    /// Read bytes into an existing buffer.
    pub fn read_into(&self, pos: u64, buf: &mut [u8]) -> Result<()> {
        if pos + buf.len() as u64 > self.size {
            return Err(Error::UnexpectedEof(pos + buf.len() as u64));
        }

        match &self.inner {
            StreamsInner::Mmap(mmap) => {
                buf.copy_from_slice(&mmap[pos as usize..(pos as usize + buf.len())]);
                Ok(())
            }
            StreamsInner::File(file) => {
                let mut f = file.write();
                f.seek(SeekFrom::Start(pos))?;
                f.read_exact(buf)?;
                Ok(())
            }
        }
    }

    /// Get a slice of the memory-mapped data (only works with mmap).
    pub fn slice(&self, pos: u64, len: usize) -> Result<&[u8]> {
        if pos + len as u64 > self.size {
            return Err(Error::UnexpectedEof(pos + len as u64));
        }

        match &self.inner {
            StreamsInner::Mmap(mmap) => {
                Ok(&mmap[pos as usize..(pos as usize + len)])
            }
            StreamsInner::File(_) => {
                Err(Error::other("slice() requires memory-mapped mode"))
            }
        }
    }

    /// Read a u64 value at the given position.
    pub fn read_u64(&self, pos: u64) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.read_into(pos, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    /// Read a u32 value at the given position.
    pub fn read_u32(&self, pos: u64) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_into(pos, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Read a u16 value at the given position.
    pub fn read_u16(&self, pos: u64) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_into(pos, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }
}

/// Ogawa archive reader.
pub struct IArchive {
    streams: Arc<IStreams>,
    root: IGroup,
}

impl IArchive {
    /// Open an Alembic file for reading.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let streams = Arc::new(IStreams::open(path)?);
        let root_pos = streams.root_pos()?;
        let root = IGroup::new(streams.clone(), root_pos, false)?;
        Ok(Self { streams, root })
    }

    /// Check if the archive is valid.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.streams.is_valid()
    }

    /// Check if the archive is frozen (finalized).
    #[inline]
    pub fn is_frozen(&self) -> bool {
        self.streams.is_frozen()
    }

    /// Get the format version.
    #[inline]
    pub fn version(&self) -> u16 {
        self.streams.version()
    }

    /// Get the root group.
    #[inline]
    pub fn root(&self) -> &IGroup {
        &self.root
    }

    /// Get access to the underlying streams.
    #[inline]
    pub fn streams(&self) -> &Arc<IStreams> {
        &self.streams
    }
}

/// A group in the Ogawa hierarchy.
/// Groups contain children which can be either data or other groups.
#[derive(Clone)]
pub struct IGroup {
    streams: Arc<IStreams>,
    pos: u64,
    num_children: u64,
    /// Cached child offsets (loaded on demand)
    child_offsets: Vec<u64>,
    /// Light mode - don't cache child offsets
    light: bool,
}

impl IGroup {
    /// Create a new group reader at the given position.
    pub fn new(streams: Arc<IStreams>, pos: u64, light: bool) -> Result<Self> {
        // Read number of children
        let num_children = if pos == 0 {
            0 // Empty group
        } else {
            streams.read_u64(pos)?
        };

        // Load child offsets (unless in light mode)
        let child_offsets = if light || num_children == 0 {
            Vec::new()
        } else {
            let mut offsets = Vec::with_capacity(num_children as usize);
            for i in 0..num_children {
                let offset_pos = pos + 8 + i * 8;
                offsets.push(streams.read_u64(offset_pos)?);
            }
            offsets
        };

        Ok(Self {
            streams,
            pos,
            num_children,
            child_offsets,
            light,
        })
    }

    /// Get the position of this group in the file.
    #[inline]
    pub fn pos(&self) -> u64 {
        self.pos
    }

    /// Get the number of children.
    #[inline]
    pub fn num_children(&self) -> u64 {
        self.num_children
    }

    /// Check if this group is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.num_children == 0
    }

    /// Check if this group is in light mode.
    #[inline]
    pub fn is_light(&self) -> bool {
        self.light
    }

    /// Get the raw offset for a child (with group/data flag).
    pub fn child_offset(&self, index: u64) -> Result<u64> {
        if index >= self.num_children {
            return Err(Error::ChildOutOfBounds {
                index: index as usize,
                count: self.num_children as usize,
            });
        }

        if !self.light && !self.child_offsets.is_empty() {
            Ok(self.child_offsets[index as usize])
        } else {
            let offset_pos = self.pos + 8 + index * 8;
            self.streams.read_u64(offset_pos)
        }
    }

    /// Check if child at index is a group.
    pub fn is_child_group(&self, index: u64) -> Result<bool> {
        Ok(is_group_offset(self.child_offset(index)?))
    }

    /// Check if child at index is data.
    pub fn is_child_data(&self, index: u64) -> Result<bool> {
        Ok(is_data_offset(self.child_offset(index)?))
    }

    /// Check if child at index is an empty group.
    pub fn is_empty_child_group(&self, index: u64) -> Result<bool> {
        let offset = self.child_offset(index)?;
        Ok(is_group_offset(offset) && is_empty_offset(offset))
    }

    /// Check if child at index is empty data.
    pub fn is_empty_child_data(&self, index: u64) -> Result<bool> {
        let offset = self.child_offset(index)?;
        Ok(is_data_offset(offset) && is_empty_offset(offset))
    }

    /// Get a child group.
    pub fn group(&self, index: u64) -> Result<IGroup> {
        let offset = self.child_offset(index)?;
        if !is_group_offset(offset) {
            return Err(Error::TypeMismatch {
                expected: "group".to_string(),
                actual: "data".to_string(),
            });
        }
        IGroup::new(self.streams.clone(), extract_offset(offset), self.light)
    }

    /// Get child data.
    pub fn data(&self, index: u64) -> Result<IData> {
        let offset = self.child_offset(index)?;
        if !is_data_offset(offset) {
            return Err(Error::TypeMismatch {
                expected: "data".to_string(),
                actual: "group".to_string(),
            });
        }
        IData::new(self.streams.clone(), extract_offset(offset))
    }

    /// Iterate over all children, returning either Group or Data.
    pub fn children(&self) -> impl Iterator<Item = Result<IChild>> + '_ {
        (0..self.num_children).map(move |i| {
            let offset = self.child_offset(i)?;
            let pos = extract_offset(offset);
            if is_group_offset(offset) {
                Ok(IChild::Group(IGroup::new(self.streams.clone(), pos, self.light)?))
            } else {
                Ok(IChild::Data(IData::new(self.streams.clone(), pos)?))
            }
        })
    }
}

/// A child in the Ogawa hierarchy - either a Group or Data.
pub enum IChild {
    Group(IGroup),
    Data(IData),
}

impl IChild {
    /// Check if this is a group.
    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group(_))
    }

    /// Check if this is data.
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Data(_))
    }

    /// Get as group (if it is one).
    pub fn as_group(&self) -> Option<&IGroup> {
        match self {
            Self::Group(g) => Some(g),
            Self::Data(_) => None,
        }
    }

    /// Get as data (if it is one).
    pub fn as_data(&self) -> Option<&IData> {
        match self {
            Self::Data(d) => Some(d),
            Self::Group(_) => None,
        }
    }
}

/// Data block in the Ogawa hierarchy.
pub struct IData {
    streams: Arc<IStreams>,
    pos: u64,
    size: u64,
}

impl IData {
    /// Create a new data reader at the given position.
    pub fn new(streams: Arc<IStreams>, pos: u64) -> Result<Self> {
        // Read size
        let size = if pos == 0 {
            0 // Empty data
        } else {
            streams.read_u64(pos)?
        };

        Ok(Self { streams, pos, size })
    }

    /// Get the position of this data in the file.
    #[inline]
    pub fn pos(&self) -> u64 {
        self.pos
    }

    /// Get the size of the data in bytes.
    #[inline]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Check if this data is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get the position of the actual data bytes (after size field).
    #[inline]
    pub fn data_pos(&self) -> u64 {
        if self.pos == 0 {
            0
        } else {
            self.pos + 8
        }
    }

    /// Read all data as bytes.
    pub fn read_all(&self) -> Result<Vec<u8>> {
        if self.size == 0 {
            return Ok(Vec::new());
        }
        self.streams.read_bytes(self.data_pos(), self.size as usize)
    }

    /// Read data into an existing buffer.
    pub fn read_into(&self, buf: &mut [u8]) -> Result<()> {
        if buf.len() != self.size as usize {
            return Err(Error::other(format!(
                "Buffer size {} doesn't match data size {}",
                buf.len(),
                self.size
            )));
        }
        if self.size == 0 {
            return Ok(());
        }
        self.streams.read_into(self.data_pos(), buf)
    }

    /// Get a slice to the data (only works with mmap).
    pub fn slice(&self) -> Result<&[u8]> {
        if self.size == 0 {
            return Ok(&[]);
        }
        self.streams.slice(self.data_pos(), self.size as usize)
    }

    /// Read data as a string (UTF-8).
    pub fn read_string(&self) -> Result<String> {
        let bytes = self.read_all()?;
        // Remove trailing null if present
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8(bytes[..len].to_vec())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_parsing() {
        let mut header = [0u8; 16];
        header[0..5].copy_from_slice(OGAWA_MAGIC);
        header[FROZEN_OFFSET] = FROZEN_FLAG;
        header[VERSION_OFFSET] = 1;
        header[VERSION_OFFSET + 1] = 0;

        let (version, frozen) = IStreams::parse_header(&header).unwrap();
        assert_eq!(version, 1);
        assert!(frozen);
    }

    #[test]
    fn test_invalid_magic() {
        let header = [0u8; 16]; // All zeros, invalid magic
        let result = IStreams::parse_header(&header);
        assert!(matches!(result, Err(Error::InvalidMagic)));
    }
}
