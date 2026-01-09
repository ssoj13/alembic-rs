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

// Re-export xform types
pub use xform::{IXform, XformSample, XformOp, XformOpType, XFORM_SCHEMA};

// Re-export polymesh types
pub use polymesh::{IPolyMesh, PolyMeshSample, MeshTopologyVariance, POLYMESH_SCHEMA};

// Re-export curves types
pub use curves::{ICurves, CurvesSample, CurveType, CurvePeriodicity, BasisType, CURVES_SCHEMA};

// Re-export points types
pub use points::{IPoints, PointsSample, POINTS_SCHEMA};

// Re-export subd types
pub use subd::{ISubD, SubDSample, SubDScheme, SUBD_SCHEMA};

// Re-export camera types
pub use camera::{ICamera, CameraSample, CAMERA_SCHEMA};

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

/// Input face set schema.
pub struct IFaceSet {
    _phantom: PhantomData<()>,
}

/// Output face set schema.
pub struct OFaceSet {
    _phantom: PhantomData<()>,
}

/// Face set sample data.
#[derive(Clone, Debug, Default)]
pub struct FaceSetSample {
    pub faces: Vec<i32>,
}

// ============================================================================
// NuPatch (NURBS)
// ============================================================================

/// Input NURBS patch schema.
pub struct INuPatch {
    _phantom: PhantomData<()>,
}

/// Output NURBS patch schema.
pub struct ONuPatch {
    _phantom: PhantomData<()>,
}

/// NURBS patch sample data.
#[derive(Clone, Debug, Default)]
pub struct NuPatchSample {
    pub positions: Vec<glam::Vec3>,
    pub num_u: i32,
    pub num_v: i32,
    pub u_order: i32,
    pub v_order: i32,
    pub u_knots: Vec<f32>,
    pub v_knots: Vec<f32>,
    pub weights: Option<Vec<f32>>,
}
