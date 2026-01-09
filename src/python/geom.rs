//! Python bindings for geometry schemas.

#![allow(non_snake_case)]

use pyo3::prelude::*;

use crate::geom::{
    PolyMeshSample, SubDSample, CurvesSample, PointsSample, CameraSample,
    XformSample, LightSample, NuPatchSample,
};

// ============================================================================
// PolyMesh
// ============================================================================

/// Python wrapper for PolyMesh sample data.
#[pyclass(name = "PolyMeshSample")]
pub struct PyPolyMeshSample {
    pub positions: Vec<[f32; 3]>,
    pub face_indices: Vec<i32>,
    pub face_counts: Vec<i32>,
    pub velocities: Option<Vec<[f32; 3]>>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub self_bounds: Option<([f64; 3], [f64; 3])>,
}

#[pymethods]
impl PyPolyMeshSample {
    /// Vertex positions as list of [x, y, z].
    #[getter]
    pub fn positions(&self) -> Vec<[f32; 3]> {
        self.positions.clone()
    }
    
    /// Face vertex indices.
    #[getter]
    pub fn faceIndices(&self) -> Vec<i32> {
        self.face_indices.clone()
    }
    
    /// Number of vertices per face.
    #[getter]
    pub fn faceCounts(&self) -> Vec<i32> {
        self.face_counts.clone()
    }
    
    /// Vertex velocities (optional).
    #[getter]
    pub fn velocities(&self) -> Option<Vec<[f32; 3]>> {
        self.velocities.clone()
    }
    
    /// Vertex normals (optional).
    #[getter]
    pub fn normals(&self) -> Option<Vec<[f32; 3]>> {
        self.normals.clone()
    }
    
    /// UV coordinates (optional).
    #[getter]
    pub fn uvs(&self) -> Option<Vec<[f32; 2]>> {
        self.uvs.clone()
    }
    
    /// Bounding box as (min, max).
    #[getter]
    pub fn selfBounds(&self) -> Option<([f64; 3], [f64; 3])> {
        self.self_bounds
    }
    
    /// Number of vertices.
    pub fn getNumVertices(&self) -> usize {
        self.positions.len()
    }
    
    /// Number of faces.
    pub fn getNumFaces(&self) -> usize {
        self.face_counts.len()
    }
    
    fn __len__(&self) -> usize {
        self.positions.len()
    }
    
    fn __repr__(&self) -> String {
        format!("<PolyMeshSample {} verts, {} faces>", 
            self.positions.len(), self.face_counts.len())
    }
}

impl From<PolyMeshSample> for PyPolyMeshSample {
    fn from(s: PolyMeshSample) -> Self {
        Self {
            positions: s.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            face_indices: s.face_indices,
            face_counts: s.face_counts,
            velocities: s.velocities.map(|v| v.iter().map(|p| [p.x, p.y, p.z]).collect()),
            normals: s.normals.map(|v| v.iter().map(|p| [p.x, p.y, p.z]).collect()),
            uvs: s.uvs.map(|v| v.iter().map(|p| [p.x, p.y]).collect()),
            self_bounds: s.self_bounds.map(|b| (
                [b.min.x, b.min.y, b.min.z],
                [b.max.x, b.max.y, b.max.z]
            )),
        }
    }
}

// ============================================================================
// SubD
// ============================================================================

/// Python wrapper for SubD sample data.
#[pyclass(name = "SubDSample")]
pub struct PySubDSample {
    pub positions: Vec<[f32; 3]>,
    pub face_indices: Vec<i32>,
    pub face_counts: Vec<i32>,
    pub scheme: String,
    pub crease_indices: Vec<i32>,
    pub crease_lengths: Vec<i32>,
    pub crease_sharpnesses: Vec<f32>,
    pub corner_indices: Vec<i32>,
    pub corner_sharpnesses: Vec<f32>,
    pub holes: Vec<i32>,
}

#[pymethods]
impl PySubDSample {
    #[getter]
    pub fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    pub fn faceIndices(&self) -> Vec<i32> { self.face_indices.clone() }
    #[getter]
    pub fn faceCounts(&self) -> Vec<i32> { self.face_counts.clone() }
    #[getter]
    pub fn scheme(&self) -> String { self.scheme.clone() }
    #[getter]
    pub fn creaseIndices(&self) -> Vec<i32> { self.crease_indices.clone() }
    #[getter]
    pub fn creaseLengths(&self) -> Vec<i32> { self.crease_lengths.clone() }
    #[getter]
    pub fn creaseSharpnesses(&self) -> Vec<f32> { self.crease_sharpnesses.clone() }
    #[getter]
    pub fn cornerIndices(&self) -> Vec<i32> { self.corner_indices.clone() }
    #[getter]
    pub fn cornerSharpnesses(&self) -> Vec<f32> { self.corner_sharpnesses.clone() }
    #[getter]
    pub fn holes(&self) -> Vec<i32> { self.holes.clone() }
    
    fn __repr__(&self) -> String {
        format!("<SubDSample {} verts, {} faces, scheme={}>", 
            self.positions.len(), self.face_counts.len(), self.scheme)
    }
}

impl From<SubDSample> for PySubDSample {
    fn from(s: SubDSample) -> Self {
        Self {
            positions: s.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            face_indices: s.face_indices,
            face_counts: s.face_counts,
            scheme: format!("{:?}", s.scheme),
            crease_indices: s.crease_indices,
            crease_lengths: s.crease_lengths,
            crease_sharpnesses: s.crease_sharpnesses,
            corner_indices: s.corner_indices,
            corner_sharpnesses: s.corner_sharpnesses,
            holes: s.holes,
        }
    }
}

// ============================================================================
// Curves
// ============================================================================

/// Python wrapper for Curves sample data.
#[pyclass(name = "CurvesSample")]
pub struct PyCurvesSample {
    pub positions: Vec<[f32; 3]>,
    pub num_vertices: Vec<i32>,
    pub curve_type: String,
    pub basis: String,
    pub wrap: String,
    pub widths: Vec<f32>,
    pub orders: Vec<i32>,
    pub knots: Vec<f32>,
}

#[pymethods]
impl PyCurvesSample {
    #[getter]
    pub fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    pub fn numVertices(&self) -> Vec<i32> { self.num_vertices.clone() }
    #[getter]
    pub fn curveType(&self) -> String { self.curve_type.clone() }
    #[getter]
    pub fn basis(&self) -> String { self.basis.clone() }
    #[getter]
    pub fn wrap(&self) -> String { self.wrap.clone() }
    #[getter]
    pub fn widths(&self) -> Vec<f32> { self.widths.clone() }
    #[getter]
    pub fn orders(&self) -> Vec<i32> { self.orders.clone() }
    #[getter]
    pub fn knots(&self) -> Vec<f32> { self.knots.clone() }
    
    /// Number of curves.
    pub fn getNumCurves(&self) -> usize { self.num_vertices.len() }
    
    fn __repr__(&self) -> String {
        format!("<CurvesSample {} curves, type={}>", self.num_vertices.len(), self.curve_type)
    }
}

impl From<CurvesSample> for PyCurvesSample {
    fn from(s: CurvesSample) -> Self {
        Self {
            positions: s.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            num_vertices: s.num_vertices,
            curve_type: format!("{:?}", s.curve_type),
            basis: format!("{:?}", s.basis),
            wrap: format!("{:?}", s.wrap),
            widths: s.widths,
            orders: s.orders,
            knots: s.knots,
        }
    }
}

// ============================================================================
// Points
// ============================================================================

/// Python wrapper for Points sample data.
#[pyclass(name = "PointsSample")]
pub struct PyPointsSample {
    pub positions: Vec<[f32; 3]>,
    pub ids: Vec<u64>,
    pub velocities: Vec<[f32; 3]>,
    pub widths: Vec<f32>,
}

#[pymethods]
impl PyPointsSample {
    #[getter]
    pub fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    pub fn ids(&self) -> Vec<u64> { self.ids.clone() }
    #[getter]
    pub fn velocities(&self) -> Vec<[f32; 3]> { self.velocities.clone() }
    #[getter]
    pub fn widths(&self) -> Vec<f32> { self.widths.clone() }
    
    fn __len__(&self) -> usize { self.positions.len() }
    
    fn __repr__(&self) -> String {
        format!("<PointsSample {} points>", self.positions.len())
    }
}

impl From<PointsSample> for PyPointsSample {
    fn from(s: PointsSample) -> Self {
        Self {
            positions: s.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            ids: s.ids,
            velocities: s.velocities.iter().map(|p| [p.x, p.y, p.z]).collect(),
            widths: s.widths,
        }
    }
}

// ============================================================================
// Camera
// ============================================================================

/// Python wrapper for Camera sample data.
#[pyclass(name = "CameraSample")]
pub struct PyCameraSample {
    pub focal_length: f64,
    pub horizontal_aperture: f64,
    pub vertical_aperture: f64,
    pub horizontal_film_offset: f64,
    pub vertical_film_offset: f64,
    pub lens_squeeze_ratio: f64,
    pub near_clipping_plane: f64,
    pub far_clipping_plane: f64,
    pub f_stop: f64,
    pub focus_distance: f64,
    pub shutter_open: f64,
    pub shutter_close: f64,
}

#[pymethods]
impl PyCameraSample {
    #[getter]
    pub fn focalLength(&self) -> f64 { self.focal_length }
    #[getter]
    pub fn horizontalAperture(&self) -> f64 { self.horizontal_aperture }
    #[getter]
    pub fn verticalAperture(&self) -> f64 { self.vertical_aperture }
    #[getter]
    pub fn horizontalFilmOffset(&self) -> f64 { self.horizontal_film_offset }
    #[getter]
    pub fn verticalFilmOffset(&self) -> f64 { self.vertical_film_offset }
    #[getter]
    pub fn lensSqueezeRatio(&self) -> f64 { self.lens_squeeze_ratio }
    #[getter]
    pub fn nearClippingPlane(&self) -> f64 { self.near_clipping_plane }
    #[getter]
    pub fn farClippingPlane(&self) -> f64 { self.far_clipping_plane }
    #[getter]
    pub fn fStop(&self) -> f64 { self.f_stop }
    #[getter]
    pub fn focusDistance(&self) -> f64 { self.focus_distance }
    #[getter]
    pub fn shutterOpen(&self) -> f64 { self.shutter_open }
    #[getter]
    pub fn shutterClose(&self) -> f64 { self.shutter_close }
    
    /// Get horizontal field of view in degrees.
    pub fn getFovHorizontal(&self) -> f64 {
        2.0 * (self.horizontal_aperture / (2.0 * self.focal_length)).atan().to_degrees()
    }
    
    /// Get vertical field of view in degrees.
    pub fn getFovVertical(&self) -> f64 {
        2.0 * (self.vertical_aperture / (2.0 * self.focal_length)).atan().to_degrees()
    }
    
    fn __repr__(&self) -> String {
        format!("<CameraSample focal={:.1}mm>", self.focal_length)
    }
}

impl From<CameraSample> for PyCameraSample {
    fn from(s: CameraSample) -> Self {
        Self {
            focal_length: s.focal_length,
            horizontal_aperture: s.horizontal_aperture,
            vertical_aperture: s.vertical_aperture,
            horizontal_film_offset: s.horizontal_film_offset,
            vertical_film_offset: s.vertical_film_offset,
            lens_squeeze_ratio: s.lens_squeeze_ratio,
            near_clipping_plane: s.near_clipping_plane,
            far_clipping_plane: s.far_clipping_plane,
            f_stop: s.f_stop,
            focus_distance: s.focus_distance,
            shutter_open: s.shutter_open,
            shutter_close: s.shutter_close,
        }
    }
}

// ============================================================================
// Xform
// ============================================================================

/// Python wrapper for Xform sample data.
#[pyclass(name = "XformSample")]
pub struct PyXformSample {
    pub matrix: [[f64; 4]; 4],
    pub inherits: bool,
}

#[pymethods]
impl PyXformSample {
    /// Get 4x4 transformation matrix (column-major).
    #[getter]
    pub fn matrix(&self) -> [[f64; 4]; 4] {
        self.matrix
    }
    
    /// Whether this transform inherits from parent.
    #[getter]
    pub fn inherits(&self) -> bool {
        self.inherits
    }
    
    /// Get translation component.
    pub fn getTranslation(&self) -> [f64; 3] {
        [self.matrix[3][0], self.matrix[3][1], self.matrix[3][2]]
    }
    
    /// Get scale component (approximate, assumes no shear).
    pub fn getScale(&self) -> [f64; 3] {
        let sx = (self.matrix[0][0].powi(2) + self.matrix[0][1].powi(2) + self.matrix[0][2].powi(2)).sqrt();
        let sy = (self.matrix[1][0].powi(2) + self.matrix[1][1].powi(2) + self.matrix[1][2].powi(2)).sqrt();
        let sz = (self.matrix[2][0].powi(2) + self.matrix[2][1].powi(2) + self.matrix[2][2].powi(2)).sqrt();
        [sx, sy, sz]
    }
    
    fn __repr__(&self) -> String {
        let t = self.getTranslation();
        format!("<XformSample translate=[{:.2}, {:.2}, {:.2}]>", t[0], t[1], t[2])
    }
}

impl From<XformSample> for PyXformSample {
    fn from(s: XformSample) -> Self {
        let m = s.matrix();
        Self {
            matrix: [
                [m.x_axis.x as f64, m.x_axis.y as f64, m.x_axis.z as f64, m.x_axis.w as f64],
                [m.y_axis.x as f64, m.y_axis.y as f64, m.y_axis.z as f64, m.y_axis.w as f64],
                [m.z_axis.x as f64, m.z_axis.y as f64, m.z_axis.z as f64, m.z_axis.w as f64],
                [m.w_axis.x as f64, m.w_axis.y as f64, m.w_axis.z as f64, m.w_axis.w as f64],
            ],
            inherits: s.inherits,
        }
    }
}

// ============================================================================
// Light
// ============================================================================

/// Python wrapper for Light sample data.
#[pyclass(name = "LightSample")]
pub struct PyLightSample {
    /// Camera-like parameters (lights share camera properties).
    pub camera: PyCameraSample,
    /// Child bounds (optional).
    pub child_bounds: Option<([f64; 3], [f64; 3])>,
}

#[pymethods]
impl PyLightSample {
    /// Get camera-like parameters.
    #[getter]
    pub fn camera(&self) -> PyCameraSample {
        PyCameraSample {
            focal_length: self.camera.focal_length,
            horizontal_aperture: self.camera.horizontal_aperture,
            vertical_aperture: self.camera.vertical_aperture,
            horizontal_film_offset: self.camera.horizontal_film_offset,
            vertical_film_offset: self.camera.vertical_film_offset,
            lens_squeeze_ratio: self.camera.lens_squeeze_ratio,
            near_clipping_plane: self.camera.near_clipping_plane,
            far_clipping_plane: self.camera.far_clipping_plane,
            f_stop: self.camera.f_stop,
            focus_distance: self.camera.focus_distance,
            shutter_open: self.camera.shutter_open,
            shutter_close: self.camera.shutter_close,
        }
    }
    
    /// Child bounds (optional).
    #[getter]
    pub fn childBounds(&self) -> Option<([f64; 3], [f64; 3])> {
        self.child_bounds
    }
    
    fn __repr__(&self) -> String {
        "<LightSample>".to_string()
    }
}

impl From<LightSample> for PyLightSample {
    fn from(s: LightSample) -> Self {
        Self {
            camera: s.camera.into(),
            child_bounds: s.child_bounds.map(|b| (
                [b.min.x, b.min.y, b.min.z],
                [b.max.x, b.max.y, b.max.z]
            )),
        }
    }
}

// ============================================================================
// NuPatch
// ============================================================================

/// Python wrapper for NuPatch (NURBS surface) sample data.
#[pyclass(name = "NuPatchSample")]
pub struct PyNuPatchSample {
    pub positions: Vec<[f32; 3]>,
    pub num_u: i32,
    pub num_v: i32,
    pub u_order: i32,
    pub v_order: i32,
    pub u_knots: Vec<f32>,
    pub v_knots: Vec<f32>,
    pub position_weights: Option<Vec<f32>>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub self_bounds: Option<([f64; 3], [f64; 3])>,
}

#[pymethods]
impl PyNuPatchSample {
    #[getter]
    pub fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    pub fn numU(&self) -> i32 { self.num_u }
    #[getter]
    pub fn numV(&self) -> i32 { self.num_v }
    #[getter]
    pub fn uOrder(&self) -> i32 { self.u_order }
    #[getter]
    pub fn vOrder(&self) -> i32 { self.v_order }
    #[getter]
    pub fn uKnots(&self) -> Vec<f32> { self.u_knots.clone() }
    #[getter]
    pub fn vKnots(&self) -> Vec<f32> { self.v_knots.clone() }
    #[getter]
    pub fn positionWeights(&self) -> Option<Vec<f32>> { self.position_weights.clone() }
    #[getter]
    pub fn normals(&self) -> Option<Vec<[f32; 3]>> { self.normals.clone() }
    #[getter]
    pub fn uvs(&self) -> Option<Vec<[f32; 2]>> { self.uvs.clone() }
    #[getter]
    pub fn selfBounds(&self) -> Option<([f64; 3], [f64; 3])> { self.self_bounds }
    
    /// Get U degree (order - 1).
    pub fn uDegree(&self) -> i32 { self.u_order - 1 }
    
    /// Get V degree (order - 1).
    pub fn vDegree(&self) -> i32 { self.v_order - 1 }
    
    /// Number of control vertices.
    pub fn getNumCVs(&self) -> usize { self.positions.len() }
    
    /// Check if rational (has weights).
    pub fn isRational(&self) -> bool { self.position_weights.is_some() }
    
    fn __repr__(&self) -> String {
        format!("<NuPatchSample {}x{} CVs, order=({}, {})>",
            self.num_u, self.num_v, self.u_order, self.v_order)
    }
}

impl From<NuPatchSample> for PyNuPatchSample {
    fn from(s: NuPatchSample) -> Self {
        Self {
            positions: s.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            num_u: s.num_u,
            num_v: s.num_v,
            u_order: s.u_order,
            v_order: s.v_order,
            u_knots: s.u_knots,
            v_knots: s.v_knots,
            position_weights: s.position_weights,
            normals: s.normals.map(|v| v.iter().map(|p| [p.x, p.y, p.z]).collect()),
            uvs: s.uvs.map(|v| v.iter().map(|p| [p.x, p.y]).collect()),
            self_bounds: s.self_bounds.map(|b| (
                [b.min.x, b.min.y, b.min.z],
                [b.max.x, b.max.y, b.max.z]
            )),
        }
    }
}
