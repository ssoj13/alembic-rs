//! Python bindings for geometry schemas.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use crate::abc::IObject;
use crate::geom::{
    IPolyMesh, ISubD, ICurves, IPoints, ICamera, ILight, IXform, INuPatch,
    PolyMeshSample, SubDSample, CurvesSample, PointsSample, CameraSample,
    XformSample, NuPatchSample, LightSample,
};

// ============================================================================
// PolyMesh
// ============================================================================

/// Python wrapper for PolyMesh sample data.
#[pyclass(name = "PolyMeshSample")]
pub struct PyPolyMeshSample {
    positions: Vec<[f32; 3]>,
    face_indices: Vec<i32>,
    face_counts: Vec<i32>,
    velocities: Option<Vec<[f32; 3]>>,
    normals: Option<Vec<[f32; 3]>>,
    uvs: Option<Vec<[f32; 2]>>,
    self_bounds: Option<([f32; 3], [f32; 3])>,
}

#[pymethods]
impl PyPolyMeshSample {
    /// Vertex positions as list of [x, y, z].
    #[getter]
    fn positions(&self) -> Vec<[f32; 3]> {
        self.positions.clone()
    }
    
    /// Face vertex indices.
    #[getter]
    fn faceIndices(&self) -> Vec<i32> {
        self.face_indices.clone()
    }
    
    /// Number of vertices per face.
    #[getter]
    fn faceCounts(&self) -> Vec<i32> {
        self.face_counts.clone()
    }
    
    /// Vertex velocities (optional).
    #[getter]
    fn velocities(&self) -> Option<Vec<[f32; 3]>> {
        self.velocities.clone()
    }
    
    /// Vertex normals (optional).
    #[getter]
    fn normals(&self) -> Option<Vec<[f32; 3]>> {
        self.normals.clone()
    }
    
    /// UV coordinates (optional).
    #[getter]
    fn uvs(&self) -> Option<Vec<[f32; 2]>> {
        self.uvs.clone()
    }
    
    /// Bounding box as (min, max).
    #[getter]
    fn selfBounds(&self) -> Option<([f32; 3], [f32; 3])> {
        self.self_bounds
    }
    
    /// Number of vertices.
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
    positions: Vec<[f32; 3]>,
    face_indices: Vec<i32>,
    face_counts: Vec<i32>,
    subdivision_scheme: String,
    crease_indices: Option<Vec<i32>>,
    crease_lengths: Option<Vec<i32>>,
    crease_sharpnesses: Option<Vec<f32>>,
    corner_indices: Option<Vec<i32>>,
    corner_sharpnesses: Option<Vec<f32>>,
}

#[pymethods]
impl PySubDSample {
    #[getter]
    fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    fn faceIndices(&self) -> Vec<i32> { self.face_indices.clone() }
    #[getter]
    fn faceCounts(&self) -> Vec<i32> { self.face_counts.clone() }
    #[getter]
    fn subdivisionScheme(&self) -> String { self.subdivision_scheme.clone() }
    #[getter]
    fn creaseIndices(&self) -> Option<Vec<i32>> { self.crease_indices.clone() }
    #[getter]
    fn creaseLengths(&self) -> Option<Vec<i32>> { self.crease_lengths.clone() }
    #[getter]
    fn creaseSharpnesses(&self) -> Option<Vec<f32>> { self.crease_sharpnesses.clone() }
    #[getter]
    fn cornerIndices(&self) -> Option<Vec<i32>> { self.corner_indices.clone() }
    #[getter]
    fn cornerSharpnesses(&self) -> Option<Vec<f32>> { self.corner_sharpnesses.clone() }
    
    fn __repr__(&self) -> String {
        format!("<SubDSample {} verts, {} faces, scheme={}>", 
            self.positions.len(), self.face_counts.len(), self.subdivision_scheme)
    }
}

impl From<SubDSample> for PySubDSample {
    fn from(s: SubDSample) -> Self {
        Self {
            positions: s.positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
            face_indices: s.face_indices,
            face_counts: s.face_counts,
            subdivision_scheme: s.subdivision_scheme,
            crease_indices: if s.crease_indices.is_empty() { None } else { Some(s.crease_indices) },
            crease_lengths: if s.crease_lengths.is_empty() { None } else { Some(s.crease_lengths) },
            crease_sharpnesses: if s.crease_sharpnesses.is_empty() { None } else { Some(s.crease_sharpnesses) },
            corner_indices: if s.corner_indices.is_empty() { None } else { Some(s.corner_indices) },
            corner_sharpnesses: if s.corner_sharpnesses.is_empty() { None } else { Some(s.corner_sharpnesses) },
        }
    }
}

// ============================================================================
// Curves
// ============================================================================

/// Python wrapper for Curves sample data.
#[pyclass(name = "CurvesSample")]
pub struct PyCurvesSample {
    positions: Vec<[f32; 3]>,
    num_vertices: Vec<i32>,
    curve_type: String,
    basis: String,
    wrap: String,
    widths: Option<Vec<f32>>,
    orders: Option<Vec<u8>>,
    knots: Option<Vec<f32>>,
}

#[pymethods]
impl PyCurvesSample {
    #[getter]
    fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    fn numVertices(&self) -> Vec<i32> { self.num_vertices.clone() }
    #[getter]
    fn curveType(&self) -> String { self.curve_type.clone() }
    #[getter]
    fn basis(&self) -> String { self.basis.clone() }
    #[getter]
    fn wrap(&self) -> String { self.wrap.clone() }
    #[getter]
    fn widths(&self) -> Option<Vec<f32>> { self.widths.clone() }
    #[getter]
    fn orders(&self) -> Option<Vec<u8>> { self.orders.clone() }
    #[getter]
    fn knots(&self) -> Option<Vec<f32>> { self.knots.clone() }
    
    /// Number of curves.
    fn getNumCurves(&self) -> usize { self.num_vertices.len() }
    
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
            wrap: format!("{:?}", s.periodicity),
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
    positions: Vec<[f32; 3]>,
    ids: Vec<u64>,
    velocities: Option<Vec<[f32; 3]>>,
    widths: Option<Vec<f32>>,
}

#[pymethods]
impl PyPointsSample {
    #[getter]
    fn positions(&self) -> Vec<[f32; 3]> { self.positions.clone() }
    #[getter]
    fn ids(&self) -> Vec<u64> { self.ids.clone() }
    #[getter]
    fn velocities(&self) -> Option<Vec<[f32; 3]>> { self.velocities.clone() }
    #[getter]
    fn widths(&self) -> Option<Vec<f32>> { self.widths.clone() }
    
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
            velocities: s.velocities.map(|v| v.iter().map(|p| [p.x, p.y, p.z]).collect()),
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
    focal_length: f64,
    horizontal_aperture: f64,
    vertical_aperture: f64,
    horizontal_film_offset: f64,
    vertical_film_offset: f64,
    lens_squeeze_ratio: f64,
    near_clipping_plane: f64,
    far_clipping_plane: f64,
    f_stop: f64,
    focus_distance: f64,
    shutter_open: f64,
    shutter_close: f64,
}

#[pymethods]
impl PyCameraSample {
    #[getter]
    fn focalLength(&self) -> f64 { self.focal_length }
    #[getter]
    fn horizontalAperture(&self) -> f64 { self.horizontal_aperture }
    #[getter]
    fn verticalAperture(&self) -> f64 { self.vertical_aperture }
    #[getter]
    fn horizontalFilmOffset(&self) -> f64 { self.horizontal_film_offset }
    #[getter]
    fn verticalFilmOffset(&self) -> f64 { self.vertical_film_offset }
    #[getter]
    fn lensSqueezeRatio(&self) -> f64 { self.lens_squeeze_ratio }
    #[getter]
    fn nearClippingPlane(&self) -> f64 { self.near_clipping_plane }
    #[getter]
    fn farClippingPlane(&self) -> f64 { self.far_clipping_plane }
    #[getter]
    fn fStop(&self) -> f64 { self.f_stop }
    #[getter]
    fn focusDistance(&self) -> f64 { self.focus_distance }
    #[getter]
    fn shutterOpen(&self) -> f64 { self.shutter_open }
    #[getter]
    fn shutterClose(&self) -> f64 { self.shutter_close }
    
    /// Get horizontal field of view in degrees.
    fn getFovHorizontal(&self) -> f64 {
        2.0 * (self.horizontal_aperture / (2.0 * self.focal_length)).atan().to_degrees()
    }
    
    /// Get vertical field of view in degrees.
    fn getFovVertical(&self) -> f64 {
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
    matrix: [[f64; 4]; 4],
    inherits_xforms: bool,
}

#[pymethods]
impl PyXformSample {
    /// Get 4x4 transformation matrix (column-major).
    #[getter]
    fn matrix(&self) -> [[f64; 4]; 4] {
        self.matrix
    }
    
    /// Whether this transform inherits from parent.
    #[getter]
    fn inheritsXforms(&self) -> bool {
        self.inherits_xforms
    }
    
    /// Get translation component.
    fn getTranslation(&self) -> [f64; 3] {
        [self.matrix[3][0], self.matrix[3][1], self.matrix[3][2]]
    }
    
    /// Get scale component (approximate, assumes no shear).
    fn getScale(&self) -> [f64; 3] {
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
            inherits_xforms: s.inherits_xforms,
        }
    }
}

// ============================================================================
// Helper functions for creating samples from IObject
// ============================================================================

/// Get PolyMesh sample from object.
pub fn get_polymesh_sample(obj: &IObject, index: usize) -> PyResult<PyPolyMeshSample> {
    let mesh = IPolyMesh::new(obj)
        .ok_or_else(|| PyValueError::new_err("Object is not a PolyMesh"))?;
    let sample = mesh.get_sample(index)
        .map_err(|e| PyValueError::new_err(format!("Failed to get sample: {}", e)))?;
    Ok(sample.into())
}

/// Get SubD sample from object.
pub fn get_subd_sample(obj: &IObject, index: usize) -> PyResult<PySubDSample> {
    let subd = ISubD::new(obj)
        .ok_or_else(|| PyValueError::new_err("Object is not a SubD"))?;
    let sample = subd.get_sample(index)
        .map_err(|e| PyValueError::new_err(format!("Failed to get sample: {}", e)))?;
    Ok(sample.into())
}

/// Get Curves sample from object.
pub fn get_curves_sample(obj: &IObject, index: usize) -> PyResult<PyCurvesSample> {
    let curves = ICurves::new(obj)
        .ok_or_else(|| PyValueError::new_err("Object is not a Curves"))?;
    let sample = curves.get_sample(index)
        .map_err(|e| PyValueError::new_err(format!("Failed to get sample: {}", e)))?;
    Ok(sample.into())
}

/// Get Points sample from object.
pub fn get_points_sample(obj: &IObject, index: usize) -> PyResult<PyPointsSample> {
    let points = IPoints::new(obj)
        .ok_or_else(|| PyValueError::new_err("Object is not a Points"))?;
    let sample = points.get_sample(index)
        .map_err(|e| PyValueError::new_err(format!("Failed to get sample: {}", e)))?;
    Ok(sample.into())
}

/// Get Camera sample from object.
pub fn get_camera_sample(obj: &IObject, index: usize) -> PyResult<PyCameraSample> {
    let camera = ICamera::new(obj)
        .ok_or_else(|| PyValueError::new_err("Object is not a Camera"))?;
    let sample = camera.get_sample(index)
        .map_err(|e| PyValueError::new_err(format!("Failed to get sample: {}", e)))?;
    Ok(sample.into())
}

/// Get Xform sample from object.
pub fn get_xform_sample(obj: &IObject, index: usize) -> PyResult<PyXformSample> {
    let xform = IXform::new(obj)
        .ok_or_else(|| PyValueError::new_err("Object is not an Xform"))?;
    let sample = xform.get_sample(index)
        .map_err(|e| PyValueError::new_err(format!("Failed to get sample: {}", e)))?;
    Ok(sample.into())
}
