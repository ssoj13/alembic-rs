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

// ============================================================================
// Safe casting helpers
// ============================================================================

/// Safely cast a byte slice to a slice of type T.
/// Returns None if the data is misaligned or has wrong size.
#[inline]
pub fn safe_cast_slice<T: bytemuck::Pod>(data: &[u8]) -> Option<&[T]> {
    bytemuck::try_cast_slice(data).ok()
}

/// Safely cast a byte slice to a Vec of type T.
/// Returns empty Vec if cast fails.
#[inline]
pub fn safe_cast_vec<T: bytemuck::Pod + Clone>(data: &[u8]) -> Vec<T> {
    bytemuck::try_cast_slice(data)
        .map(|s: &[T]| s.to_vec())
        .unwrap_or_default()
}

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
pub use camera::{ICamera, CameraSample, CAMERA_SCHEMA, FilmBackXformOp, FilmBackXformOpType};

// Re-export visibility types
pub use visibility::{
    ObjectVisibility, VISIBILITY_PROPERTY_NAME, 
    get_visibility, is_visible, get_visibility_property,
    is_ancestor_invisible, is_ancestor_invisible_in_archive,
    // Output support
    OVisibilityProperty, create_visibility_property, add_visibility_sample,
};

// Re-export geom_param types
pub use geom_param::{
    // Input (ITypedGeomParam<T>)
    IGeomParam, GeomParamSample,
    IV2fGeomParam, IV3fGeomParam, IN3fGeomParam,
    IC3fGeomParam, IC4fGeomParam,
    IInt32GeomParam, IUInt32GeomParam, IFloatGeomParam,
    // Output (OTypedGeomParam<T>)
    OGeomParam, OGeomParamSample,
    OV2fGeomParam, OV3fGeomParam, ON3fGeomParam,
    OC3fGeomParam, OC4fGeomParam,
    OInt32GeomParam, OUInt32GeomParam, OFloatGeomParam,
    // Constants
    GEOM_SCOPE_KEY, VALS_PROPERTY_NAME, INDICES_PROPERTY_NAME,
};

// Re-export faceset types
pub use faceset::{IFaceSet, FaceSetSample, FaceSetExclusivity, FACESET_SCHEMA};

// Re-export nupatch types
pub use nupatch::{INuPatch, NuPatchSample, TrimCurveData, TrimCurve, NUPATCH_SCHEMA};

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
