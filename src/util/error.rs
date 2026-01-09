//! Error types for the Alembic library.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Alembic operations.
#[derive(Error, Debug)]
pub enum Error {
    /// File does not exist or cannot be accessed
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Invalid magic bytes at start of file
    #[error("Invalid Alembic file: expected Ogawa magic bytes")]
    InvalidMagic,

    /// Unsupported file format version
    #[error("Unsupported Alembic version: {0}")]
    UnsupportedVersion(u16),

    /// File is truncated or corrupted
    #[error("Unexpected end of file at position {0}")]
    UnexpectedEof(u64),

    /// Invalid data structure in file
    #[error("Invalid file structure: {0}")]
    InvalidStructure(String),

    /// Property not found by name
    #[error("Property not found: {0}")]
    PropertyNotFound(String),

    /// Object not found by name or path
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    /// Type mismatch when reading data
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// Sample index out of bounds
    #[error("Sample index {index} out of bounds (count: {count})")]
    SampleOutOfBounds { index: usize, count: usize },

    /// Child index out of bounds
    #[error("Child index {index} out of bounds (count: {count})")]
    ChildOutOfBounds { index: usize, count: usize },

    /// Invalid metadata format
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    /// Schema mismatch
    #[error("Schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: String, actual: String },

    /// Write operation failed
    #[error("Write failed: {0}")]
    WriteFailed(String),

    /// Archive is not writable (opened read-only)
    #[error("Archive is read-only")]
    ReadOnly,

    /// Archive is frozen (finalized)
    #[error("Archive is frozen and cannot be modified")]
    Frozen,

    /// Memory mapping failed
    #[error("Memory mapping failed: {0}")]
    MmapFailed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 conversion error
    #[error("Invalid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create an "other" error from a string.
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Create an invalid structure error.
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::InvalidStructure(msg.into())
    }
}

/// Result type alias for Alembic operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = Error::InvalidMagic;
        assert!(e.to_string().contains("magic"));

        let e = Error::SampleOutOfBounds { index: 5, count: 3 };
        assert!(e.to_string().contains("5"));
        assert!(e.to_string().contains("3"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }
}
