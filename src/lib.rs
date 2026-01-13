//! # Alembic
//!
//! Rust implementation of the Alembic (.abc) 3D interchange format.
//!
//! Original Alembic format and C++ library developed by Sony Pictures Imageworks
//! and Industrial Light & Magic. All rights to the original belong to the authors.
//! This is an independent Rust implementation aiming to match the original as closely
//! as possible for binary compatibility.
//!
//! ## Modules
//!
//! - [`util`] - Basic types (POD, DataType, errors)
//! - [`ogawa`] - Low-level Ogawa binary format
//! - [`core`] - Abstract traits and core implementations
//! - [`abc`] - High-level API (IArchive, OArchive, Objects, Properties)
//! - [`geom`] - Geometry schemas (PolyMesh, Xform, Curves, etc.)
//! - [`material`] - Material and shader network support
//! - [`collection`] - Collection/grouping support
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
pub mod material;
pub mod collection;

// Python bindings (optional, enabled with "python" feature)
#[cfg(feature = "python")]
pub mod python;

// 3D Viewer (optional, enabled with "viewer" feature)
#[cfg(feature = "viewer")]
pub mod viewer;

// Re-export commonly used types
pub use util::{DataType, PlainOldDataType, Error, Result};
pub use ogawa::{IArchive as OgawaIArchive, OArchive as OgawaOArchive};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::util::{DataType, PlainOldDataType, Error, Result};
    pub use crate::abc::{IArchive, OArchive, IObject, OObject};
    pub use crate::ogawa::{IArchive as OgawaIArchive, OArchive as OgawaOArchive};
    pub use crate::core::{TimeSampling, SampleSelector};
    pub use crate::geom::*;
}
