//! Ogawa writer stream.
//!
//! Reference: `_ref/alembic/lib/Alembic/AbcCoreOgawa/AwImpl.cpp` (stream writes).

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};

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

    /// Return a Frozen error if stream is already finalized.
    #[inline]
    pub fn frozen_error() -> Error {
        Error::Frozen
    }
}
