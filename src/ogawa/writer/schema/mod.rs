//! Schema writers (AbcGeom/AbcMaterial/AbcCollection).
//!
//! These are higher-level writers built on top of Ogawa core.
//! References: AbcGeom / AbcMaterial / AbcCollection in `_ref/alembic/lib/Alembic`.

mod util;

pub mod polymesh;
pub mod xform;
pub mod curves;
pub mod points;
pub mod subd;
pub mod camera;
pub mod nupatch;
pub mod light;
pub mod faceset;
pub mod material;
pub mod collections;

pub use polymesh::{OPolyMesh, OPolyMeshSample};
pub use xform::{OXform, OXformSample};
pub use curves::{OCurves, OCurvesSample};
pub use points::{OPoints, OPointsSample};
pub use subd::{OSubD, OSubDSample};
pub use camera::OCamera;
pub use nupatch::{ONuPatch, ONuPatchSample};
pub use light::OLight;
pub use faceset::{OFaceSet, OFaceSetSample};
pub use material::{OMaterial, OMaterialSample};
pub use collections::{OCollections, OCollectionsSample};
