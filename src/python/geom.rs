//! Python bindings for geometry schemas.

#![allow(non_snake_case)]

use pyo3::prelude::*;

use crate::geom::{
    PolyMeshSample, SubDSample, CurvesSample, PointsSample, CameraSample,
    XformSample, LightSample, NuPatchSample, FaceSetSample, GeomParamSample,
    IFaceSet, IGeomParam,
    visibility::{ObjectVisibility, OVisibilityProperty},
};
use std::sync::Arc;

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
// FaceSet
// ============================================================================

/// Python wrapper for FaceSet sample data.
#[pyclass(name = "FaceSetSample")]
pub struct PyFaceSetSample {
    pub faces: Vec<i32>,
    pub self_bounds: Option<([f64; 3], [f64; 3])>,
}

#[pymethods]
impl PyFaceSetSample {
    /// Face indices in this face set.
    #[getter]
    pub fn faces(&self) -> Vec<i32> {
        self.faces.clone()
    }
    
    /// Bounding box (optional).
    #[getter]
    pub fn selfBounds(&self) -> Option<([f64; 3], [f64; 3])> {
        self.self_bounds
    }
    
    /// Number of faces in this set.
    pub fn getNumFaces(&self) -> usize {
        self.faces.len()
    }
    
    /// Check if face index is in this set.
    pub fn contains(&self, face_index: i32) -> bool {
        self.faces.contains(&face_index)
    }
    
    fn __len__(&self) -> usize {
        self.faces.len()
    }
    
    fn __repr__(&self) -> String {
        format!("<FaceSetSample {} faces>", self.faces.len())
    }
}

impl From<FaceSetSample> for PyFaceSetSample {
    fn from(s: FaceSetSample) -> Self {
        Self {
            faces: s.faces,
            self_bounds: s.self_bounds.map(|b| (
                [b.min.x, b.min.y, b.min.z],
                [b.max.x, b.max.y, b.max.z]
            )),
        }
    }
}

/// Python wrapper for IFaceSet schema reader.
#[pyclass(name = "IFaceSet")]
pub struct PyIFaceSet {
    archive: Arc<crate::abc::IArchive>,
    path: String,
}

#[pymethods]
impl PyIFaceSet {
    /// Get object name.
    fn getName(&self) -> String {
        self.path.rsplit('/').next().unwrap_or("").to_string()
    }
    
    /// Get full path.
    fn getFullName(&self) -> &str {
        &self.path
    }
    
    /// Get number of samples.
    fn getNumSamples(&self) -> usize {
        self.with_faceset(|fs| fs.getNumSamples()).unwrap_or(1)
    }
    
    /// Check if constant (single sample).
    fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Get face exclusivity setting.
    fn getFaceExclusivity(&self) -> String {
        self.with_faceset(|fs| format!("{:?}", fs.face_exclusivity()))
            .unwrap_or_else(|| "NonExclusive".to_string())
    }
    
    /// Read a sample at given index.
    #[pyo3(signature = (index=0))]
    fn getSample(&self, index: usize) -> PyResult<PyFaceSetSample> {
        self.with_faceset(|fs| {
            fs.getSample(index).ok().map(|s| s.into())
        })
        .flatten()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to read sample"))
    }
    
    fn __repr__(&self) -> String {
        format!("<IFaceSet '{}'>", self.getName())
    }
}

impl PyIFaceSet {
    /// Create from archive and path.
    pub fn new(archive: Arc<crate::abc::IArchive>, path: String) -> Self {
        Self { archive, path }
    }
    
    /// Helper to work with IFaceSet using recursive traversal.
    fn with_faceset<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IFaceSet<'_>) -> T,
    {
        let root = self.archive.getTop();
        let parts: Vec<&str> = self.path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[&str],
            f: impl FnOnce(&IFaceSet<'_>) -> T,
        ) -> Option<T> {
            if path.is_empty() {
                let fs = IFaceSet::new(&obj)?;
                Some(f(&fs))
            } else {
                let child = obj.getChildByName(path[0])?;
                traverse(child, &path[1..], f)
            }
        }
        
        traverse(root, &parts, f)
    }
}

// ============================================================================
// GeomParam
// ============================================================================

/// Python wrapper for GeomParam sample data.
#[pyclass(name = "GeomParamSample")]
pub struct PyGeomParamSample {
    pub values: Vec<f32>,
    pub indices: Option<Vec<u32>>,
    pub scope: String,
    pub is_indexed: bool,
}

#[pymethods]
impl PyGeomParamSample {
    /// Raw values as float array.
    #[getter]
    pub fn values(&self) -> Vec<f32> {
        self.values.clone()
    }
    
    /// Indices for indexed params (None if not indexed).
    #[getter]
    pub fn indices(&self) -> Option<Vec<u32>> {
        self.indices.clone()
    }
    
    /// Geometry scope ("vertex", "facevarying", etc).
    #[getter]
    pub fn scope(&self) -> &str {
        &self.scope
    }
    
    /// Whether this param is indexed.
    #[getter]
    pub fn isIndexed(&self) -> bool {
        self.is_indexed
    }
    
    /// Get values as Vec2 array.
    pub fn asVec2(&self) -> Vec<[f32; 2]> {
        self.values.chunks_exact(2)
            .map(|c| [c[0], c[1]])
            .collect()
    }
    
    /// Get values as Vec3 array.
    pub fn asVec3(&self) -> Vec<[f32; 3]> {
        self.values.chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect()
    }
    
    /// Get values as Vec4 array.
    pub fn asVec4(&self) -> Vec<[f32; 4]> {
        self.values.chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect()
    }
    
    fn __repr__(&self) -> String {
        let count = self.values.len();
        format!("<GeomParamSample {} values, scope={}, indexed={}>", 
            count, self.scope, self.is_indexed)
    }
}

impl From<GeomParamSample> for PyGeomParamSample {
    fn from(s: GeomParamSample) -> Self {
        Self {
            values: s.values_as_f32().to_vec(),
            indices: s.indices,
            scope: format!("{:?}", s.scope),
            is_indexed: s.is_indexed,
        }
    }
}

/// Python wrapper for IGeomParam schema reader.
#[pyclass(name = "IGeomParam")]
pub struct PyIGeomParam {
    archive: Arc<crate::abc::IArchive>,
    object_path: String,
    param_name: String,
}

#[pymethods]
impl PyIGeomParam {
    /// Get parameter name.
    fn getName(&self) -> &str {
        &self.param_name
    }
    
    /// Get number of samples.
    fn getNumSamples(&self) -> usize {
        self.with_geomparam(|gp| gp.getNumSamples()).unwrap_or(0)
    }
    
    /// Check if constant.
    fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Check if indexed.
    fn isIndexed(&self) -> bool {
        self.with_geomparam(|gp| gp.is_indexed()).unwrap_or(false)
    }
    
    /// Get geometry scope.
    fn getScope(&self) -> String {
        self.with_geomparam(|gp| format!("{:?}", gp.scope()))
            .unwrap_or_else(|| "Unknown".to_string())
    }
    
    /// Read a sample at given index.
    #[pyo3(signature = (index=0))]
    fn getSample(&self, index: usize) -> PyResult<PyGeomParamSample> {
        self.with_geomparam(|gp| {
            gp.getSample(index).ok().map(|s| s.into())
        })
        .flatten()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to read sample"))
    }
    
    /// Read expanded sample (indices applied).
    #[pyo3(signature = (index=0))]
    fn getExpandedSample(&self, index: usize) -> PyResult<PyGeomParamSample> {
        self.with_geomparam(|gp| {
            gp.get_expanded_sample(index).ok().map(|s| s.into())
        })
        .flatten()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to read sample"))
    }
    
    /// Get UVs directly as Vec2 array.
    #[pyo3(signature = (index=0))]
    fn getUVs(&self, index: usize) -> PyResult<Vec<[f32; 2]>> {
        self.with_geomparam(|gp| {
            gp.get_uvs(index).ok().map(|v| v.iter().map(|u| [u.x, u.y]).collect())
        })
        .flatten()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to read UVs"))
    }
    
    /// Get normals directly as Vec3 array.
    #[pyo3(signature = (index=0))]
    fn getNormals(&self, index: usize) -> PyResult<Vec<[f32; 3]>> {
        self.with_geomparam(|gp| {
            gp.get_normals(index).ok().map(|v| v.iter().map(|n| [n.x, n.y, n.z]).collect())
        })
        .flatten()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to read normals"))
    }
    
    /// Get colors as Vec3 array.
    #[pyo3(signature = (index=0))]
    fn getColors3(&self, index: usize) -> PyResult<Vec<[f32; 3]>> {
        self.with_geomparam(|gp| {
            gp.get_colors3(index).ok().map(|v| v.iter().map(|c| [c.x, c.y, c.z]).collect())
        })
        .flatten()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to read colors"))
    }
    
    fn __repr__(&self) -> String {
        format!("<IGeomParam '{}'>", self.param_name)
    }
}

impl PyIGeomParam {
    /// Create from archive, object path and param name.
    pub fn new(archive: Arc<crate::abc::IArchive>, object_path: String, param_name: String) -> Self {
        Self { archive, object_path, param_name }
    }
    
    /// Helper to work with IGeomParam using recursive traversal.
    fn with_geomparam<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IGeomParam<'_>) -> T,
    {
        let root = self.archive.getTop();
        let parts: Vec<&str> = self.object_path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        let param_name = self.param_name.clone();
        
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[&str],
            param_name: &str,
            f: impl FnOnce(&IGeomParam<'_>) -> T,
        ) -> Option<T> {
            if path.is_empty() {
                let props = obj.getProperties();
                let geom_box = props.getPropertyByName(".geom")?;
                let geom_prop = geom_box.asCompound()?;
                let gp = IGeomParam::new(&geom_prop, param_name)?;
                Some(f(&gp))
            } else {
                let child = obj.getChildByName(path[0])?;
                traverse(child, &path[1..], param_name, f)
            }
        }
        
        traverse(root, &parts, &param_name, f)
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
    
    /// Get rotation as Euler angles (XYZ order) in degrees.
    /// Assumes matrix has no shear; scale is removed before extraction.
    pub fn getRotation(&self) -> [f64; 3] {
        // Get scale to normalize rotation matrix
        let scale = self.getScale();
        let sx = if scale[0].abs() > 1e-10 { scale[0] } else { 1.0 };
        let sy = if scale[1].abs() > 1e-10 { scale[1] } else { 1.0 };
        let sz = if scale[2].abs() > 1e-10 { scale[2] } else { 1.0 };
        
        // Normalized rotation matrix (column vectors)
        let m00 = self.matrix[0][0] / sx;
        let m01 = self.matrix[0][1] / sx;
        let m02 = self.matrix[0][2] / sx;
        let m10 = self.matrix[1][0] / sy;
        let m11 = self.matrix[1][1] / sy;
        let m12 = self.matrix[1][2] / sy;
        let _m20 = self.matrix[2][0] / sz;
        let _m21 = self.matrix[2][1] / sz;
        let m22 = self.matrix[2][2] / sz;
        
        // Extract Euler angles (XYZ order)
        let (rx, ry, rz);
        
        if m02.abs() < 0.9999 {
            ry = (-m02).asin();
            let cos_ry = ry.cos();
            rx = (m12 / cos_ry).atan2(m22 / cos_ry);
            rz = (m01 / cos_ry).atan2(m00 / cos_ry);
        } else {
            // Gimbal lock
            rz = 0.0;
            if m02 < 0.0 {
                ry = std::f64::consts::FRAC_PI_2;
                rx = m10.atan2(m11);
            } else {
                ry = -std::f64::consts::FRAC_PI_2;
                rx = (-m10).atan2(m11);
            }
        }
        
        // Convert to degrees
        [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()]
    }
    
    /// Get rotation as quaternion [x, y, z, w].
    pub fn getRotationQuaternion(&self) -> [f64; 4] {
        // Get scale to normalize
        let scale = self.getScale();
        let sx = if scale[0].abs() > 1e-10 { scale[0] } else { 1.0 };
        let sy = if scale[1].abs() > 1e-10 { scale[1] } else { 1.0 };
        let sz = if scale[2].abs() > 1e-10 { scale[2] } else { 1.0 };
        
        // Normalized rotation matrix
        let m00 = self.matrix[0][0] / sx;
        let m01 = self.matrix[0][1] / sx;
        let m02 = self.matrix[0][2] / sx;
        let m10 = self.matrix[1][0] / sy;
        let m11 = self.matrix[1][1] / sy;
        let m12 = self.matrix[1][2] / sy;
        let m20 = self.matrix[2][0] / sz;
        let m21 = self.matrix[2][1] / sz;
        let m22 = self.matrix[2][2] / sz;
        
        // Convert rotation matrix to quaternion
        let trace = m00 + m11 + m22;
        
        let (x, y, z, w);
        
        if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            w = 0.25 / s;
            x = (m12 - m21) * s;
            y = (m20 - m02) * s;
            z = (m01 - m10) * s;
        } else if m00 > m11 && m00 > m22 {
            let s = 2.0 * (1.0 + m00 - m11 - m22).sqrt();
            w = (m12 - m21) / s;
            x = 0.25 * s;
            y = (m10 + m01) / s;
            z = (m20 + m02) / s;
        } else if m11 > m22 {
            let s = 2.0 * (1.0 + m11 - m00 - m22).sqrt();
            w = (m20 - m02) / s;
            x = (m10 + m01) / s;
            y = 0.25 * s;
            z = (m21 + m12) / s;
        } else {
            let s = 2.0 * (1.0 + m22 - m00 - m11).sqrt();
            w = (m01 - m10) / s;
            x = (m20 + m02) / s;
            y = (m21 + m12) / s;
            z = 0.25 * s;
        }
        
        [x, y, z, w]
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

// ============================================================================
// Visibility
// ============================================================================

/// Object visibility state.
/// 
/// Controls whether an object should be visible in the scene:
/// - Deferred (-1): inherit from parent
/// - Hidden (0): explicitly hidden
/// - Visible (1): explicitly visible
#[pyclass(name = "ObjectVisibility")]
#[derive(Clone)]
pub struct PyObjectVisibility {
    inner: ObjectVisibility,
}

#[pymethods]
impl PyObjectVisibility {
    /// Create Deferred visibility (inherit from parent).
    #[staticmethod]
    fn deferred() -> Self {
        Self { inner: ObjectVisibility::Deferred }
    }
    
    /// Create Hidden visibility.
    #[staticmethod]
    fn hidden() -> Self {
        Self { inner: ObjectVisibility::Hidden }
    }
    
    /// Create Visible visibility.
    #[staticmethod]
    fn visible() -> Self {
        Self { inner: ObjectVisibility::Visible }
    }
    
    /// Create from integer value (-1=deferred, 0=hidden, 1=visible).
    #[staticmethod]
    fn fromValue(value: i8) -> Self {
        Self { inner: ObjectVisibility::from_i8(value) }
    }
    
    /// Get integer value (-1, 0, or 1).
    #[getter]
    fn value(&self) -> i8 {
        self.inner.to_i8()
    }
    
    /// Check if deferred (inherit from parent).
    fn isDeferred(&self) -> bool {
        self.inner.is_deferred()
    }
    
    /// Check if explicitly hidden.
    fn isHidden(&self) -> bool {
        self.inner.is_hidden()
    }
    
    /// Check if explicitly visible.
    fn isVisible(&self) -> bool {
        self.inner.is_visible()
    }
    
    fn __repr__(&self) -> String {
        match self.inner {
            ObjectVisibility::Deferred => "<ObjectVisibility.Deferred>".to_string(),
            ObjectVisibility::Hidden => "<ObjectVisibility.Hidden>".to_string(),
            ObjectVisibility::Visible => "<ObjectVisibility.Visible>".to_string(),
        }
    }
    
    fn __eq__(&self, other: &PyObjectVisibility) -> bool {
        self.inner == other.inner
    }
}

impl From<ObjectVisibility> for PyObjectVisibility {
    fn from(vis: ObjectVisibility) -> Self {
        Self { inner: vis }
    }
}

/// Output visibility property for writing.
#[pyclass(name = "OVisibilityProperty")]
pub struct PyOVisibilityProperty {
    inner: OVisibilityProperty,
}

#[pymethods]
impl PyOVisibilityProperty {
    /// Create a new visibility property.
    #[new]
    fn new() -> Self {
        Self { inner: OVisibilityProperty::new() }
    }
    
    /// Set visibility to visible.
    fn setVisible(&mut self) {
        self.inner.set_visible();
    }
    
    /// Set visibility to hidden.
    fn setHidden(&mut self) {
        self.inner.set_hidden();
    }
    
    /// Set visibility to deferred (inherit from parent).
    fn setDeferred(&mut self) {
        self.inner.set_deferred();
    }
    
    /// Set visibility from PyObjectVisibility.
    fn set(&mut self, vis: &PyObjectVisibility) {
        self.inner.set(vis.inner);
    }
    
    fn __repr__(&self) -> String {
        "<OVisibilityProperty>".to_string()
    }
}

impl PyOVisibilityProperty {
    /// Create a new visibility property (for Rust code).
    pub fn create() -> Self {
        Self { inner: OVisibilityProperty::new() }
    }
    
    /// Get the underlying OProperty for adding to objects.
    pub fn into_property(self) -> crate::ogawa::writer::OProperty {
        self.inner.into_property()
    }
}
