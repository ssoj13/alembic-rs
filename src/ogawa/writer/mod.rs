//! Ogawa format writer implementation.
//!
//! This module is split for easier debugging and parity with the C++ reference.
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/*`
//! - `_ref/alembic/lib/Alembic/AbcGeom/*`

mod constants;
mod stream;
mod write_util;
mod object;
mod property;
mod archive;

pub mod schema;

pub use archive::OArchive;
pub use object::OObject;
pub use property::{OProperty, OPropertyData};

// Re-export schema writers for API compatibility.
pub use schema::{
    OPolyMesh, OPolyMeshSample,
    OXform, OXformSample,
    OCurves, OCurvesSample,
    OPoints, OPointsSample,
    OSubD, OSubDSample,
    OCamera,
    ONuPatch, ONuPatchSample,
    OLight,
    OFaceSet, OFaceSetSample,
    OMaterial, OMaterialSample,
    OCollections, OCollectionsSample,
};

#[cfg(test)]
mod tests;
