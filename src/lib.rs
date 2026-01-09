//! # Alembic
//!
//! Rust implementation of the Alembic (.abc) 3D interchange format.
//!
//! Alembic is an open computer graphics interchange framework developed by
//! Sony Pictures Imageworks and Industrial Light & Magic. It distills complex,
//! animated scenes into baked geometric results.
//!
//! ## Modules
//!
//! - [`util`] - Basic types (POD, DataType, errors)
//! - [`ogawa`] - Low-level Ogawa binary format
//! - [`core`] - Abstract traits and core implementations
//! - [`abc`] - High-level API (IArchive, OArchive, Objects, Properties)
//! - [`geom`] - Geometry schemas (PolyMesh, Xform, Curves, etc.)
//!
//! ## Example
//!
//! ```ignore
//! use alembic::abc::IArchive;
//!
//! let archive = IArchive::open("animation.abc")?;
//! let root = archive.root();
//!
//! for child in root.children() {
//!     println!("{}", child.name());
//! }
//! ```

pub mod util;
pub mod ogawa;
pub mod core;
pub mod abc;
pub mod geom;

// Re-export commonly used types
pub use util::{DataType, PlainOldDataType, Error, Result};
pub use ogawa::{IArchive as OgawaIArchive, OArchive as OgawaOArchive};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::util::{DataType, PlainOldDataType, Error, Result};
    pub use crate::abc::{IArchive, OArchive, IObject, OObject};
    pub use crate::geom::*;
}
