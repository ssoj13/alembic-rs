//! Geometry schemas for Alembic.
//!
//! This module provides typed schemas for common geometry types:
//! - [`IPolyMesh`] / [`OPolyMesh`] - Polygon meshes
//! - [`IXform`] / [`OXform`] - Transform nodes
//! - [`ICurves`] / [`OCurves`] - NURBS/Bezier curves
//! - [`IPoints`] / [`OPoints`] - Point clouds
//! - [`ISubD`] / [`OSubD`] - Subdivision surfaces
//! - [`ICamera`] / [`OCamera`] - Cameras
//! - [`IFaceSet`] / [`OFaceSet`] - Face groupings
//! - [`INuPatch`] / [`ONuPatch`] - NURBS patches

use std::marker::PhantomData;

pub mod xform;
pub mod polymesh;
pub mod curves;
pub mod points;
pub mod subd;
pub mod camera;
pub mod visibility;
pub mod geom_param;
pub mod faceset;
pub mod nupatch;
pub mod light;

// Re-export xform types
pub use xform::{IXform, XformSample, XformOp, XformOpType, XFORM_SCHEMA};

// Re-export polymesh types
pub use polymesh::{IPolyMesh, PolyMeshSample, POLYMESH_SCHEMA};

// Re-export curves types
pub use curves::{ICurves, CurvesSample, CurveType, CurvePeriodicity, BasisType, CURVES_SCHEMA};

// Re-export points types
pub use points::{IPoints, PointsSample, POINTS_SCHEMA};

// Re-export subd types
pub use subd::{ISubD, SubDSample, SubDScheme, SUBD_SCHEMA};

// Re-export camera types
pub use camera::{ICamera, CameraSample, CAMERA_SCHEMA};

// Re-export visibility types
pub use visibility::{ObjectVisibility, VISIBILITY_PROPERTY_NAME, get_visibility, is_visible, get_visibility_property};

// Re-export geom_param types
pub use geom_param::{
    IGeomParam, GeomParamSample,
    IV2fGeomParam, IV3fGeomParam, IN3fGeomParam,
    IC3fGeomParam, IC4fGeomParam,
    IInt32GeomParam, IUInt32GeomParam, IFloatGeomParam,
    GEOM_SCOPE_KEY, VALS_PROPERTY_NAME, INDICES_PROPERTY_NAME,
};

// Re-export faceset types
pub use faceset::{IFaceSet, FaceSetSample, FaceSetExclusivity, FACESET_SCHEMA};

// Re-export nupatch types
pub use nupatch::{INuPatch, NuPatchSample, TrimCurveData, NUPATCH_SCHEMA};

// Re-export light types
pub use light::{ILight, LightSample, LIGHT_SCHEMA};

// ============================================================================
// PolyMesh
// ============================================================================

/// Output polygon mesh schema.
pub struct OPolyMesh {
    _phantom: PhantomData<()>,
}

// ============================================================================
// Xform
// ============================================================================

/// Output transform schema.
pub struct OXform {
    _phantom: PhantomData<()>,
}

// ============================================================================
// Curves
// ============================================================================

/// Output curves schema.
pub struct OCurves {
    _phantom: PhantomData<()>,
}

// ============================================================================
// Points
// ============================================================================

/// Output points schema.
pub struct OPoints {
    _phantom: PhantomData<()>,
}

// ============================================================================
// SubD (Subdivision Surface)
// ============================================================================

/// Output subdivision surface schema.
pub struct OSubD {
    _phantom: PhantomData<()>,
}

// ============================================================================
// Camera
// ============================================================================

/// Output camera schema.
pub struct OCamera {
    _phantom: PhantomData<()>,
}

// ============================================================================
// FaceSet
// ============================================================================

/// Output face set schema.
pub struct OFaceSet {
    _phantom: PhantomData<()>,
}

// ============================================================================
// NuPatch (NURBS)
// ============================================================================

/// Output NURBS patch schema.
pub struct ONuPatch {
    _phantom: PhantomData<()>,
}
