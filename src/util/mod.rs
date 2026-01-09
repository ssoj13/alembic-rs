//! Utility types and functions for Alembic.
//!
//! This module contains fundamental types used throughout the library:
//! - [`PlainOldDataType`] - Enum of basic data types
//! - [`DataType`] - POD + extent (dimensionality)
//! - [`Error`] / [`Result`] - Error handling
//! - Math type re-exports from glam

mod pod;
mod data_type;
mod error;
mod math;
mod dimensions;

pub use pod::*;
pub use data_type::*;
pub use error::*;
pub use math::*;
pub use dimensions::*;
