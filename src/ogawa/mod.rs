//! Low-level Ogawa binary format implementation.
//!
//! Ogawa is the modern binary format used by Alembic files (.abc).
//! This module provides direct read/write access to the format.
//!
//! ## File Structure
//!
//! ```text
//! +------------------+
//! | Magic: "Ogawa"   |  5 bytes
//! +------------------+
//! | Frozen flag      |  1 byte (0x00 or 0xFF)
//! +------------------+
//! | Version          |  2 bytes (u16 LE)
//! +------------------+
//! | Root Group Pos   |  8 bytes (u64 LE)
//! +------------------+
//! | ... Data ...     |
//! +------------------+
//! ```

mod format;
mod reader;
pub mod writer;
mod abc_impl;
mod read_util;

pub use format::*;
pub use reader::*;
pub use writer::*;
pub use abc_impl::*;
pub use read_util::*;
