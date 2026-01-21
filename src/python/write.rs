//! Python bindings for Alembic write API.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};
use std::sync::{Arc, Mutex};


use crate::ogawa::writer::{
    OArchive, OObject, OPolyMesh, OXform, OPolyMeshSample, OXformSample,
    OCurves, OCurvesSample, OPoints, OPointsSample, OSubD, OSubDSample,
    OCamera, ONuPatch, ONuPatchSample, OLight, OFaceSet, OFaceSetSample,
    OMaterial, OMaterialSample, OCollections, OProperty,
};
use crate::util::DataType;
use crate::core::TimeSampling;
use crate::geom::{CurveType, CurvePeriodicity, BasisType, CameraSample};
use crate::material::{ShaderParam, ShaderParamValue};

// ============================================================================
// OArchive wrapper
// ============================================================================

/// Python wrapper for OArchive (write-only archive).
#[pyclass(name = "OArchive")]
pub struct PyOArchive {
    archive: Arc<Mutex<Option<OArchive>>>,
}

#[pymethods]
impl PyOArchive {
    /// Create a new Alembic archive for writing.
    #[staticmethod]
    fn create(path: &str) -> PyResult<Self> {
        let archive = OArchive::create(path)
            .map_err(|e| PyIOError::new_err(format!("Failed to create archive: {}", e)))?;
        
        Ok(Self {
            archive: Arc::new(Mutex::new(Some(archive))),
        })
    }
    
    /// Get archive name/path.
    fn getName(&self) -> PyResult<String> {
        let guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in getName(): {}", e)))?;
        let archive = guard.as_ref().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        Ok(archive.getName().to_string())
    }
    
    /// Set application writer string.
    fn setApplicationWriter(&self, writer: &str) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setApplicationWriter(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.set_application_writer(writer);
        Ok(())
    }
    
    /// Set compression hint (-1 = no compression, 0-9 = compression level).
    fn setCompressionHint(&self, hint: i32) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setCompressionHint(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.setCompressionHint(hint);
        Ok(())
    }
    
    /// Enable/disable deduplication.
    fn setDedupEnabled(&self, enabled: bool) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setDedupEnabled(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.setDedupEnabled(enabled);
        Ok(())
    }
    
    /// Set the application name (stored as _ai_Application in metadata).
    fn setAppName(&self, name: &str) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setAppName(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.setAppName(name);
        Ok(())
    }
    
    /// Set the date written (stored as _ai_DateWritten in metadata).
    fn setDateWritten(&self, date: &str) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setDateWritten(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.setDateWritten(date);
        Ok(())
    }
    
    /// Set the user description (stored as _ai_Description in metadata).
    fn setDescription(&self, desc: &str) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setDescription(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.setUserDescription(desc);
        Ok(())
    }
    
    /// Set the DCC FPS (stored as _ai_DCC_FPS in metadata).
    fn setDccFps(&self, fps: f64) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in setDccFps(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        archive.setDccFps(fps);
        Ok(())
    }
    
    /// Add uniform time sampling (fps-based). Returns time sampling index.
    #[pyo3(signature = (fps, start_time=0.0))]
    fn addUniformTimeSampling(&self, fps: f64, start_time: f64) -> PyResult<u32> {
        let time_per_cycle = 1.0 / fps;
        let ts = TimeSampling::uniform(time_per_cycle, start_time);
        
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in addUniformTimeSampling(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        Ok(archive.addTimeSampling(ts))
    }
    
    /// Add acyclic time sampling with explicit frame times. Returns time sampling index.
    fn addAcyclicTimeSampling(&self, times: Vec<f64>) -> PyResult<u32> {
        let ts = TimeSampling::acyclic(times);
        
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in addAcyclicTimeSampling(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        Ok(archive.addTimeSampling(ts))
    }
    
    /// Add cyclic time sampling. Returns time sampling index.
    fn addCyclicTimeSampling(&self, time_per_cycle: f64, times: Vec<f64>) -> PyResult<u32> {
        let ts = TimeSampling::cyclic(time_per_cycle, times);
        
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in addCyclicTimeSampling(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        Ok(archive.addTimeSampling(ts))
    }
    
    /// Write the root object hierarchy and finalize.
    fn writeArchive(&self, root: &PyOObject) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in writeArchive(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        
        archive.write_archive(&root.inner)
            .map_err(|e| PyIOError::new_err(format!("Failed to write archive: {}", e)))?;
        
        Ok(())
    }
    
    /// Write PolyMesh hierarchy and finalize.
    fn writePolyMesh(&self, mesh: &mut PyOPolyMesh) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in writePolyMesh(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        
        let obj = mesh.take()?.build();
        archive.write_archive(&obj)
            .map_err(|e| PyIOError::new_err(format!("Failed to write archive: {}", e)))?;
        
        Ok(())
    }
    
    /// Write Xform hierarchy and finalize.
    fn writeXform(&self, xform: &mut PyOXform) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in writeXform(): {}", e)))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive already closed"))?;
        
        let obj = xform.take()?.build();
        archive.write_archive(&obj)
            .map_err(|e| PyIOError::new_err(format!("Failed to write archive: {}", e)))?;
        
        Ok(())
    }
    
    /// Close the archive.
    fn close(&self) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|e| PyValueError::new_err(format!("Archive lock poisoned in close(): {}", e)))?;
        if let Some(archive) = guard.take() {
            archive.close()
                .map_err(|e| PyIOError::new_err(format!("Failed to close archive: {}", e)))?;
        }
        Ok(())
    }
    
    /// Context manager enter.
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    
    /// Context manager exit - close archive.
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.close()?;
        Ok(false)
    }
    
    fn __repr__(&self) -> String {
        let guard = self.archive.lock();
        if let Ok(g) = guard {
            if let Some(archive) = g.as_ref() {
                return format!("<OArchive '{}'>", archive.getName());
            }
        }
        "<OArchive (closed)>".to_string()
    }
}

// ============================================================================
// OObject wrapper
// ============================================================================

/// Python wrapper for generic output object.
#[pyclass(name = "OObject")]
pub struct PyOObject {
    pub(crate) inner: OObject,
}

#[pymethods]
impl PyOObject {
    /// Create a new object with name.
    #[new]
    fn new(name: &str) -> Self {
        Self { inner: OObject::new(name) }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.inner.name
    }
    
    /// Add a child object.
    fn addChild(&mut self, child: &PyOObject) {
        self.inner.add_child(child.inner.clone());
    }
    
    /// Add a PolyMesh as child. Consumes the mesh.
    fn addPolyMesh(&mut self, mesh: &mut PyOPolyMesh) -> PyResult<()> {
        let obj = mesh.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add an Xform as child. Consumes the xform.
    fn addXform(&mut self, xform: &mut PyOXform) -> PyResult<()> {
        let obj = xform.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add Curves as child. Consumes the curves.
    fn addCurves(&mut self, curves: &mut PyOCurves) -> PyResult<()> {
        let obj = curves.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add Points as child. Consumes the points.
    fn addPoints(&mut self, points: &mut PyOPoints) -> PyResult<()> {
        let obj = points.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add SubD as child. Consumes the subd.
    fn addSubD(&mut self, subd: &mut PyOSubD) -> PyResult<()> {
        let obj = subd.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add Camera as child. Consumes the camera.
    fn addCamera(&mut self, camera: &mut PyOCamera) -> PyResult<()> {
        let obj = camera.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add NuPatch as child. Consumes the nupatch.
    fn addNuPatch(&mut self, nupatch: &mut PyONuPatch) -> PyResult<()> {
        let obj = nupatch.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add Light as child. Consumes the light.
    fn addLight(&mut self, light: &mut PyOLight) -> PyResult<()> {
        let obj = light.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add FaceSet as child. Consumes the faceset.
    fn addFaceSet(&mut self, faceset: &mut PyOFaceSet) -> PyResult<()> {
        let obj = faceset.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add Material as child. Consumes the material.
    fn addMaterial(&mut self, material: &mut PyOMaterial) -> PyResult<()> {
        let obj = material.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add Collections as child. Consumes the collections.
    fn addCollections(&mut self, collections: &mut PyOCollections) -> PyResult<()> {
        let obj = collections.take()?.build();
        self.inner.add_child(obj);
        Ok(())
    }
    
    /// Add a scalar property to this object.
    fn addScalarProperty(&mut self, prop: &PyOScalarProperty) {
        self.inner.add_property(prop.inner.clone());
    }
    
    /// Add an array property to this object.
    fn addArrayProperty(&mut self, prop: &PyOArrayProperty) {
        self.inner.add_property(prop.inner.clone());
    }
    
    /// Add a compound property to this object.
    fn addCompoundProperty(&mut self, prop: &PyOCompoundProperty) {
        self.inner.add_property(prop.inner.clone());
    }
    
    /// Create and add a visibility property to this object.
    /// Returns the visibility property for writing visibility samples.
    /// Note: Call finalizeVisibility() after setting visibility samples to commit.
    fn addVisibilityProperty(&mut self) -> super::geom::PyOVisibilityProperty {
        super::geom::PyOVisibilityProperty::create()
    }
    
    /// Apply configured visibility property to this object.
    /// Must be called after configuring the visibility property returned by addVisibilityProperty().
    fn applyVisibilityProperty(&mut self, vis: &mut super::geom::PyOVisibilityProperty) {
        let prop = std::mem::replace(vis, super::geom::PyOVisibilityProperty::create()).into_property();
        self.inner.add_property(prop);
    }
    
    fn __repr__(&self) -> String {
        format!("<OObject '{}'>", self.inner.name)
    }
}

// ============================================================================
// OPolyMesh wrapper
// ============================================================================

/// Python wrapper for output PolyMesh.
#[pyclass(name = "OPolyMesh")]
pub struct PyOPolyMesh {
    pub(crate) inner: Option<OPolyMesh>,
    name: String,
}

impl PyOPolyMesh {
    /// Take inner OPolyMesh, consuming it. Returns error if already consumed.
    pub(crate) fn take(&mut self) -> PyResult<OPolyMesh> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("PolyMesh '{}' already consumed (added to parent)", self.name)))
    }
    
    /// Get mutable reference to inner, returns error if consumed.
    fn inner_mut(&mut self) -> PyResult<&mut OPolyMesh> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("PolyMesh '{}' already consumed (added to parent)", self.name)))
    }
}

#[pymethods]
impl PyOPolyMesh {
    /// Create a new PolyMesh with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OPolyMesh::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Check if this object has been consumed (added to a parent).
    fn isConsumed(&self) -> bool {
        self.inner.is_none()
    }
    
    /// Add a sample with positions, face counts, and face indices.
    #[pyo3(signature = (positions, face_counts, face_indices, normals=None, uvs=None))]
    fn addSample(
        &mut self,
        positions: Vec<[f32; 3]>,
        face_counts: Vec<i32>,
        face_indices: Vec<i32>,
        normals: Option<Vec<[f32; 3]>>,
        uvs: Option<Vec<[f32; 2]>>,
    ) -> PyResult<()> {
        let inner = self.inner_mut()?;
        
        let pos: Vec<glam::Vec3> = positions.iter()
            .map(|p| glam::Vec3::new(p[0], p[1], p[2]))
            .collect();
        
        let mut sample = OPolyMeshSample::new(pos, face_counts, face_indices);
        
        if let Some(norms) = normals {
            sample.normals = Some(norms.iter()
                .map(|n| glam::Vec3::new(n[0], n[1], n[2]))
                .collect());
        }
        
        if let Some(uv_data) = uvs {
            sample.uvs = Some(uv_data.iter()
                .map(|u| glam::Vec2::new(u[0], u[1]))
                .collect());
        }
        
        inner.add_sample(&sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OPolyMesh '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OXform wrapper
// ============================================================================

/// Python wrapper for output Xform (transform).
#[pyclass(name = "OXform")]
pub struct PyOXform {
    pub(crate) inner: Option<OXform>,
    name: String,
}

impl PyOXform {
    /// Take inner OXform, consuming it. Returns error if already consumed.
    pub(crate) fn take(&mut self) -> PyResult<OXform> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Xform '{}' already consumed (added to parent)", self.name)))
    }
    
    /// Get mutable reference to inner, returns error if consumed.
    fn inner_mut(&mut self) -> PyResult<&mut OXform> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Xform '{}' already consumed (added to parent)", self.name)))
    }
}

#[pymethods]
impl PyOXform {
    /// Create a new Xform with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OXform::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Check if this object has been consumed (added to a parent).
    fn isConsumed(&self) -> bool {
        self.inner.is_none()
    }
    
    /// Add identity sample.
    fn addIdentitySample(&mut self) -> PyResult<()> {
        self.inner_mut()?.add_sample(OXformSample::identity());
        Ok(())
    }
    
    /// Add sample from 4x4 matrix (row-major, f32).
    ///
    /// Matrix layout matches Python/NumPy convention: `matrix[row][col]`.
    /// This is also consistent with Alembic C++ API.
    #[pyo3(signature = (matrix, inherits=true))]
    fn addMatrixSample(&mut self, matrix: [[f32; 4]; 4], inherits: bool) -> PyResult<()> {
        // Input is row-major (Python/NumPy convention), transpose to column-major for glam
        let m = glam::Mat4::from_cols_array_2d(&matrix).transpose();
        self.inner_mut()?.add_sample(OXformSample::from_matrix(m, inherits));
        Ok(())
    }
    
    /// Add sample from translation.
    fn addTranslationSample(&mut self, x: f32, y: f32, z: f32) -> PyResult<()> {
        let m = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
        self.inner_mut()?.add_sample(OXformSample::from_matrix(m, true));
        Ok(())
    }
    
    /// Add sample from scale.
    fn addScaleSample(&mut self, x: f32, y: f32, z: f32) -> PyResult<()> {
        let m = glam::Mat4::from_scale(glam::Vec3::new(x, y, z));
        self.inner_mut()?.add_sample(OXformSample::from_matrix(m, true));
        Ok(())
    }
    
    /// Add a child object.
    fn addChild(&mut self, child: &PyOObject) -> PyResult<()> {
        self.inner_mut()?.add_child(child.inner.clone());
        Ok(())
    }
    
    /// Add a PolyMesh as child. Consumes the mesh.
    fn addPolyMesh(&mut self, mesh: &mut PyOPolyMesh) -> PyResult<()> {
        let obj = mesh.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add another Xform as child. Consumes the xform.
    fn addXformChild(&mut self, xform: &mut PyOXform) -> PyResult<()> {
        let obj = xform.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add Curves as child. Consumes the curves.
    fn addCurves(&mut self, curves: &mut PyOCurves) -> PyResult<()> {
        let obj = curves.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add Points as child. Consumes the points.
    fn addPoints(&mut self, points: &mut PyOPoints) -> PyResult<()> {
        let obj = points.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add SubD as child. Consumes the subd.
    fn addSubD(&mut self, subd: &mut PyOSubD) -> PyResult<()> {
        let obj = subd.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add Camera as child. Consumes the camera.
    fn addCamera(&mut self, camera: &mut PyOCamera) -> PyResult<()> {
        let obj = camera.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add NuPatch as child. Consumes the nupatch.
    fn addNuPatch(&mut self, nupatch: &mut PyONuPatch) -> PyResult<()> {
        let obj = nupatch.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    /// Add Light as child. Consumes the light.
    fn addLight(&mut self, light: &mut PyOLight) -> PyResult<()> {
        let obj = light.take()?.build();
        self.inner_mut()?.add_child(obj);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OXform '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OCurves wrapper
// ============================================================================

/// Python wrapper for output Curves.
#[pyclass(name = "OCurves")]
pub struct PyOCurves {
    pub(crate) inner: Option<OCurves>,
    name: String,
}

impl PyOCurves {
    /// Take inner OCurves, consuming it. Returns error if already consumed.
    pub(crate) fn take(&mut self) -> PyResult<OCurves> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Curves '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OCurves> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Curves '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOCurves {
    /// Create a new Curves with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OCurves::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a sample.
    /// curve_type: "linear", "cubic", "bezier", "bspline", "catmullrom", "hermite"
    /// wrap: "nonperiodic", "periodic"
    /// basis: "nobasis", "bezier", "bspline", "catmullrom", "hermite", "power"
    #[allow(clippy::too_many_arguments)] // Python API requires many params for full sample data
    #[pyo3(signature = (
        positions, num_vertices, 
        curve_type="linear", wrap="nonperiodic", basis="nobasis",
        velocities=None, widths=None, normals=None, uvs=None, knots=None, orders=None
    ))]
    fn addSample(
        &mut self,
        positions: Vec<[f32; 3]>,
        num_vertices: Vec<i32>,
        curve_type: &str,
        wrap: &str,
        basis: &str,
        velocities: Option<Vec<[f32; 3]>>,
        widths: Option<Vec<f32>>,
        normals: Option<Vec<[f32; 3]>>,
        uvs: Option<Vec<[f32; 2]>>,
        knots: Option<Vec<f32>>,
        orders: Option<Vec<i32>>,
    ) -> PyResult<()> {
        let pos: Vec<glam::Vec3> = positions.iter()
            .map(|p| glam::Vec3::new(p[0], p[1], p[2]))
            .collect();
        
        let ct = match curve_type.to_lowercase().as_str() {
            "cubic" => CurveType::Cubic,
            "bezier" => CurveType::Cubic,
            "bspline" => CurveType::Cubic,
            "catmullrom" => CurveType::Cubic,
            "hermite" => CurveType::Cubic,
            _ => CurveType::Linear,
        };
        
        let w = match wrap.to_lowercase().as_str() {
            "periodic" => CurvePeriodicity::Periodic,
            _ => CurvePeriodicity::NonPeriodic,
        };
        
        let b = match basis.to_lowercase().as_str() {
            "bezier" => BasisType::Bezier,
            "bspline" => BasisType::Bspline,
            "catmullrom" => BasisType::CatmullRom,
            "hermite" => BasisType::Hermite,
            "power" => BasisType::Power,
            _ => BasisType::NoBasis,
        };
        
        let mut sample = OCurvesSample::new(pos, num_vertices)
            .with_curve_type(ct)
            .with_wrap(w)
            .with_basis(b);
        
        if let Some(vels) = velocities {
            sample.velocities = Some(vels.iter()
                .map(|v| glam::Vec3::new(v[0], v[1], v[2]))
                .collect());
        }
        
        sample.widths = widths;
        
        if let Some(norms) = normals {
            sample.normals = Some(norms.iter()
                .map(|n| glam::Vec3::new(n[0], n[1], n[2]))
                .collect());
        }
        
        if let Some(uv_data) = uvs {
            sample.uvs = Some(uv_data.iter()
                .map(|u| glam::Vec2::new(u[0], u[1]))
                .collect());
        }
        
        sample.knots = knots;
        sample.orders = orders;
        
        self.inner_mut()?.add_sample(&sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OCurves '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OPoints wrapper
// ============================================================================

/// Python wrapper for output Points.
#[pyclass(name = "OPoints")]
pub struct PyOPoints {
    pub(crate) inner: Option<OPoints>,
    name: String,
}

impl PyOPoints {
    /// Take inner OPoints, consuming it. Returns error if already consumed.
    pub(crate) fn take(&mut self) -> PyResult<OPoints> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Points '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OPoints> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Points '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOPoints {
    /// Create a new Points with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OPoints::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a sample.
    #[pyo3(signature = (positions, ids, velocities=None, widths=None))]
    fn addSample(
        &mut self,
        positions: Vec<[f32; 3]>,
        ids: Vec<u64>,
        velocities: Option<Vec<[f32; 3]>>,
        widths: Option<Vec<f32>>,
    ) -> PyResult<()> {
        let pos: Vec<glam::Vec3> = positions.iter()
            .map(|p| glam::Vec3::new(p[0], p[1], p[2]))
            .collect();
        
        // Convert u64 to i64 (Alembic uses signed IDs internally)
        let ids_i64: Vec<i64> = ids.iter().map(|&id| id as i64).collect();
        let mut sample = OPointsSample::new(pos, ids_i64);
        
        if let Some(vels) = velocities {
            sample.velocities = Some(vels.iter()
                .map(|v| glam::Vec3::new(v[0], v[1], v[2]))
                .collect());
        }
        
        sample.widths = widths;
        
        self.inner_mut()?.add_sample(&sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OPoints '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OSubD wrapper
// ============================================================================

/// Python wrapper for output SubD (subdivision surface).
#[pyclass(name = "OSubD")]
pub struct PyOSubD {
    pub(crate) inner: Option<OSubD>,
    name: String,
}

impl PyOSubD {
    /// Take inner OSubD, consuming it. Returns error if already consumed.
    pub(crate) fn take(&mut self) -> PyResult<OSubD> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("SubD '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OSubD> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("SubD '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOSubD {
    /// Create a new SubD with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OSubD::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a sample.
    /// scheme: "catmullClark", "loop", "bilinear"
    #[allow(clippy::too_many_arguments)] // Python API requires many params for full SubD sample
    #[pyo3(signature = (
        positions, face_counts, face_indices, scheme="catmullClark",
        velocities=None, crease_indices=None, crease_lengths=None, crease_sharpnesses=None,
        corner_indices=None, corner_sharpnesses=None, holes=None, uvs=None, uv_indices=None
    ))]
    fn addSample(
        &mut self,
        positions: Vec<[f32; 3]>,
        face_counts: Vec<i32>,
        face_indices: Vec<i32>,
        scheme: &str,
        velocities: Option<Vec<[f32; 3]>>,
        crease_indices: Option<Vec<i32>>,
        crease_lengths: Option<Vec<i32>>,
        crease_sharpnesses: Option<Vec<f32>>,
        corner_indices: Option<Vec<i32>>,
        corner_sharpnesses: Option<Vec<f32>>,
        holes: Option<Vec<i32>>,
        uvs: Option<Vec<[f32; 2]>>,
        uv_indices: Option<Vec<i32>>,
    ) -> PyResult<()> {
        let pos: Vec<glam::Vec3> = positions.iter()
            .map(|p| glam::Vec3::new(p[0], p[1], p[2]))
            .collect();
        
        let mut sample = OSubDSample::new(pos, face_counts, face_indices)
            .with_scheme(scheme);
        
        if let Some(vels) = velocities {
            sample.velocities = Some(vels.iter()
                .map(|v| glam::Vec3::new(v[0], v[1], v[2]))
                .collect());
        }
        
        sample.crease_indices = crease_indices;
        sample.crease_lengths = crease_lengths;
        sample.crease_sharpnesses = crease_sharpnesses;
        sample.corner_indices = corner_indices;
        sample.corner_sharpnesses = corner_sharpnesses;
        sample.holes = holes;
        
        if let Some(uv_data) = uvs {
            sample.uvs = Some(uv_data.iter()
                .map(|u| glam::Vec2::new(u[0], u[1]))
                .collect());
        }
        
        sample.uv_indices = uv_indices;
        
        self.inner_mut()?.add_sample(&sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OSubD '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OCamera wrapper
// ============================================================================

/// Python wrapper for output Camera.
#[pyclass(name = "OCamera")]
pub struct PyOCamera {
    pub(crate) inner: Option<OCamera>,
    name: String,
}

impl PyOCamera {
    pub(crate) fn take(&mut self) -> PyResult<OCamera> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Camera '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OCamera> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Camera '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOCamera {
    /// Create a new Camera with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OCamera::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a sample with camera parameters.
    #[allow(clippy::too_many_arguments)] // Camera has 16 standard params per Alembic spec
    #[pyo3(signature = (
        focal_length=35.0, horizontal_aperture=36.0, vertical_aperture=24.0,
        horizontal_film_offset=0.0, vertical_film_offset=0.0, lens_squeeze_ratio=1.0,
        overscan_left=0.0, overscan_right=0.0, overscan_top=0.0, overscan_bottom=0.0,
        f_stop=5.6, focus_distance=5.0, shutter_open=0.0, shutter_close=0.0208333,
        near_clipping_plane=0.1, far_clipping_plane=100000.0
    ))]
    fn addSample(
        &mut self,
        focal_length: f64,
        horizontal_aperture: f64,
        vertical_aperture: f64,
        horizontal_film_offset: f64,
        vertical_film_offset: f64,
        lens_squeeze_ratio: f64,
        overscan_left: f64,
        overscan_right: f64,
        overscan_top: f64,
        overscan_bottom: f64,
        f_stop: f64,
        focus_distance: f64,
        shutter_open: f64,
        shutter_close: f64,
        near_clipping_plane: f64,
        far_clipping_plane: f64,
    ) -> PyResult<()> {
        let sample = CameraSample {
            focal_length,
            horizontal_aperture,
            horizontal_film_offset,
            vertical_aperture,
            vertical_film_offset,
            lens_squeeze_ratio,
            overscan_left,
            overscan_right,
            overscan_top,
            overscan_bottom,
            f_stop,
            focus_distance,
            shutter_open,
            shutter_close,
            near_clipping_plane,
            far_clipping_plane,
            ..Default::default()
        };
        self.inner_mut()?.add_sample(sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OCamera '{}'{}>", self.name, status)
    }
}

// ============================================================================
// ONuPatch wrapper
// ============================================================================

/// Python wrapper for output NuPatch (NURBS patch).
#[pyclass(name = "ONuPatch")]
pub struct PyONuPatch {
    pub(crate) inner: Option<ONuPatch>,
    name: String,
}

impl PyONuPatch {
    pub(crate) fn take(&mut self) -> PyResult<ONuPatch> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("NuPatch '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut ONuPatch> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("NuPatch '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyONuPatch {
    /// Create a new NuPatch with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(ONuPatch::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a sample.
    #[allow(clippy::too_many_arguments)] // NuPatch requires many params per Alembic spec
    #[pyo3(signature = (
        positions, num_u, num_v, u_order, v_order, u_knot, v_knot,
        position_weights=None, velocities=None, uvs=None, normals=None
    ))]
    fn addSample(
        &mut self,
        positions: Vec<[f32; 3]>,
        num_u: i32,
        num_v: i32,
        u_order: i32,
        v_order: i32,
        u_knot: Vec<f32>,
        v_knot: Vec<f32>,
        position_weights: Option<Vec<f32>>,
        velocities: Option<Vec<[f32; 3]>>,
        uvs: Option<Vec<[f32; 2]>>,
        normals: Option<Vec<[f32; 3]>>,
    ) -> PyResult<()> {
        let pos: Vec<glam::Vec3> = positions.iter()
            .map(|p| glam::Vec3::new(p[0], p[1], p[2]))
            .collect();
        
        let mut sample = ONuPatchSample::new(pos, num_u, num_v, u_order, v_order, u_knot, v_knot);
        
        sample.position_weights = position_weights;
        
        if let Some(vels) = velocities {
            sample.velocities = Some(vels.iter()
                .map(|v| glam::Vec3::new(v[0], v[1], v[2]))
                .collect());
        }
        
        if let Some(uv_data) = uvs {
            sample.uvs = Some(uv_data.iter()
                .map(|u| glam::Vec2::new(u[0], u[1]))
                .collect());
        }
        
        if let Some(norms) = normals {
            sample.normals = Some(norms.iter()
                .map(|n| glam::Vec3::new(n[0], n[1], n[2]))
                .collect());
        }
        
        self.inner_mut()?.add_sample(&sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<ONuPatch '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OLight wrapper
// ============================================================================

/// Python wrapper for output Light.
#[pyclass(name = "OLight")]
pub struct PyOLight {
    pub(crate) inner: Option<OLight>,
    name: String,
}

impl PyOLight {
    pub(crate) fn take(&mut self) -> PyResult<OLight> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Light '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OLight> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Light '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOLight {
    /// Create a new Light with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OLight::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a camera sample (light uses camera schema for parameters).
    #[allow(clippy::too_many_arguments)] // Camera has 16 standard params per Alembic spec
    #[pyo3(signature = (
        focal_length=35.0, horizontal_aperture=36.0, vertical_aperture=24.0,
        horizontal_film_offset=0.0, vertical_film_offset=0.0, lens_squeeze_ratio=1.0,
        overscan_left=0.0, overscan_right=0.0, overscan_top=0.0, overscan_bottom=0.0,
        f_stop=5.6, focus_distance=5.0, shutter_open=0.0, shutter_close=0.0208333,
        near_clipping_plane=0.1, far_clipping_plane=100000.0
    ))]
    fn addCameraSample(
        &mut self,
        focal_length: f64,
        horizontal_aperture: f64,
        vertical_aperture: f64,
        horizontal_film_offset: f64,
        vertical_film_offset: f64,
        lens_squeeze_ratio: f64,
        overscan_left: f64,
        overscan_right: f64,
        overscan_top: f64,
        overscan_bottom: f64,
        f_stop: f64,
        focus_distance: f64,
        shutter_open: f64,
        shutter_close: f64,
        near_clipping_plane: f64,
        far_clipping_plane: f64,
    ) -> PyResult<()> {
        let sample = CameraSample {
            focal_length,
            horizontal_aperture,
            horizontal_film_offset,
            vertical_aperture,
            vertical_film_offset,
            lens_squeeze_ratio,
            overscan_left,
            overscan_right,
            overscan_top,
            overscan_bottom,
            f_stop,
            focus_distance,
            shutter_open,
            shutter_close,
            near_clipping_plane,
            far_clipping_plane,
            ..Default::default()
        };
        self.inner_mut()?.add_camera_sample(sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OLight '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OFaceSet wrapper
// ============================================================================

/// Python wrapper for output FaceSet.
#[pyclass(name = "OFaceSet")]
pub struct PyOFaceSet {
    pub(crate) inner: Option<OFaceSet>,
    name: String,
}

impl PyOFaceSet {
    pub(crate) fn take(&mut self) -> PyResult<OFaceSet> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("FaceSet '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OFaceSet> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("FaceSet '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOFaceSet {
    /// Create a new FaceSet with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OFaceSet::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a sample with face indices.
    fn addSample(&mut self, faces: Vec<i32>) -> PyResult<()> {
        let sample = OFaceSetSample::new(faces);
        self.inner_mut()?.add_sample(&sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OFaceSet '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OMaterial wrapper
// ============================================================================

/// Python wrapper for output Material.
#[pyclass(name = "OMaterial")]
pub struct PyOMaterial {
    pub(crate) inner: Option<OMaterial>,
    name: String,
    sample: OMaterialSample,
}

impl PyOMaterial {
    pub(crate) fn take(&mut self) -> PyResult<OMaterial> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Material '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OMaterial> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Material '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOMaterial {
    /// Create a new Material with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OMaterial::new(name)),
            name: name.to_string(),
            sample: OMaterialSample::new(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a shader.
    fn addShader(&mut self, target: &str, shader_type: &str, shader_name: &str) {
        self.sample.add_shader(target, shader_type, shader_name);
    }
    
    /// Add a float parameter for a target/shaderType pair.
    fn addFloatParam(&mut self, target: &str, shader_type: &str, name: &str, value: f32) {
        self.sample.add_param(target, shader_type, ShaderParam {
            name: name.to_string(),
            value: ShaderParamValue::Float(value),
        });
    }
    
    /// Add a vec3/color3 parameter for a target/shaderType pair.
    fn addVec3Param(&mut self, target: &str, shader_type: &str, name: &str, x: f32, y: f32, z: f32) {
        self.sample.add_param(target, shader_type, ShaderParam {
            name: name.to_string(),
            value: ShaderParamValue::Vec3(glam::Vec3::new(x, y, z)),
        });
    }
    
    /// Add an int parameter for a target/shaderType pair.
    fn addIntParam(&mut self, target: &str, shader_type: &str, name: &str, value: i32) {
        self.sample.add_param(target, shader_type, ShaderParam {
            name: name.to_string(),
            value: ShaderParamValue::Int(value),
        });
    }
    
    /// Add a string parameter for a target/shaderType pair.
    fn addStringParam(&mut self, target: &str, shader_type: &str, name: &str, value: &str) {
        self.sample.add_param(target, shader_type, ShaderParam {
            name: name.to_string(),
            value: ShaderParamValue::String(value.to_string()),
        });
    }
    
    /// Finalize and set sample on the material before building.
    fn finalize(&mut self) -> PyResult<()> {
        let sample = std::mem::take(&mut self.sample);
        self.inner_mut()?.set_sample(sample);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OMaterial '{}'{}>", self.name, status)
    }
}

// ============================================================================
// OCollections wrapper
// ============================================================================

/// Python wrapper for output Collections.
#[pyclass(name = "OCollections")]
pub struct PyOCollections {
    pub(crate) inner: Option<OCollections>,
    name: String,
}

impl PyOCollections {
    pub(crate) fn take(&mut self) -> PyResult<OCollections> {
        self.inner.take().ok_or_else(|| 
            PyValueError::new_err(format!("Collections '{}' already consumed", self.name)))
    }
    
    fn inner_mut(&mut self) -> PyResult<&mut OCollections> {
        self.inner.as_mut().ok_or_else(|| 
            PyValueError::new_err(format!("Collections '{}' already consumed", self.name)))
    }
}

#[pymethods]
impl PyOCollections {
    /// Create a new Collections with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: Some(OCollections::new(name)),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add a collection with name and list of object paths.
    fn addCollection(&mut self, name: &str, paths: Vec<String>) -> PyResult<()> {
        self.inner_mut()?.add_collection(name, paths);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        let status = if self.inner.is_none() { " (consumed)" } else { "" };
        format!("<OCollections '{}'{}>", self.name, status)
    }
}

// ============================================================================
// Property Writing Support
// ============================================================================

/// Parse data type string to DataType.
fn parse_data_type(type_str: &str) -> Option<DataType> {
    match type_str.to_lowercase().as_str() {
        // Scalars
        "bool" | "boolean" => Some(DataType::BOOL),
        "uint8" | "u8" => Some(DataType::UINT8),
        "int8" | "i8" => Some(DataType::INT8),
        "uint16" | "u16" => Some(DataType::UINT16),
        "int16" | "i16" => Some(DataType::INT16),
        "uint32" | "u32" | "uint" => Some(DataType::UINT32),
        "int32" | "i32" | "int" => Some(DataType::INT32),
        "uint64" | "u64" => Some(DataType::UINT64),
        "int64" | "i64" => Some(DataType::INT64),
        "float16" | "f16" | "half" => Some(DataType::FLOAT16),
        "float32" | "f32" | "float" => Some(DataType::FLOAT32),
        "float64" | "f64" | "double" => Some(DataType::FLOAT64),
        "string" | "str" => Some(DataType::STRING),
        // Vectors
        "vec2f" | "float2" | "v2f" => Some(DataType::VEC2F),
        "vec3f" | "float3" | "v3f" => Some(DataType::VEC3F),
        "vec4f" | "float4" | "v4f" => Some(DataType::VEC4F),
        "vec2d" | "double2" | "v2d" => Some(DataType::VEC2D),
        "vec3d" | "double3" | "v3d" => Some(DataType::VEC3D),
        "vec4d" | "double4" | "v4d" => Some(DataType::VEC4D),
        "vec2i" | "int2" | "v2i" => Some(DataType::VEC2I),
        "vec3i" | "int3" | "v3i" => Some(DataType::VEC3I),
        // Matrices
        "mat33f" | "matrix3" | "m33f" => Some(DataType::MAT33F),
        "mat44f" | "matrix4" | "m44f" => Some(DataType::MAT44F),
        _ => None,
    }
}

/// Python wrapper for output scalar property.
#[pyclass(name = "OScalarProperty")]
pub struct PyOScalarProperty {
    inner: OProperty,
}

#[pymethods]
impl PyOScalarProperty {
    /// Create a new scalar property.
    /// 
    /// Type can be: "int", "float", "double", "bool", "string",
    /// "vec2f", "vec3f", "vec4f", "vec2d", "vec3d", etc.
    #[new]
    #[pyo3(signature = (name, data_type, time_sampling_index=0))]
    fn new(name: &str, data_type: &str, time_sampling_index: u32) -> PyResult<Self> {
        let dt = parse_data_type(data_type)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown data type: {}", data_type)))?;
        
        let mut prop = OProperty::scalar(name, dt);
        prop.time_sampling_index = time_sampling_index;
        
        Ok(Self { inner: prop })
    }
    
    /// Get property name.
    fn getName(&self) -> &str {
        &self.inner.name
    }
    
    /// Add an integer sample.
    fn addSampleInt(&mut self, value: i32) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a float sample.
    fn addSampleFloat(&mut self, value: f32) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a double sample.
    fn addSampleDouble(&mut self, value: f64) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a boolean sample.
    fn addSampleBool(&mut self, value: bool) {
        let v: u8 = if value { 1 } else { 0 };
        self.inner.add_scalar_pod(&v);
    }
    
    /// Add a Vec2f sample.
    fn addSampleVec2f(&mut self, value: [f32; 2]) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a Vec3f sample.
    fn addSampleVec3f(&mut self, value: [f32; 3]) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a Vec4f sample.
    fn addSampleVec4f(&mut self, value: [f32; 4]) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a Vec3d sample.
    fn addSampleVec3d(&mut self, value: [f64; 3]) {
        self.inner.add_scalar_pod(&value);
    }
    
    /// Add a Mat44f sample (4x4 matrix as flat 16 floats).
    fn addSampleMat44f(&mut self, value: [[f32; 4]; 4]) {
        // Flatten to contiguous array
        let flat: [f32; 16] = [
            value[0][0], value[0][1], value[0][2], value[0][3],
            value[1][0], value[1][1], value[1][2], value[1][3],
            value[2][0], value[2][1], value[2][2], value[2][3],
            value[3][0], value[3][1], value[3][2], value[3][3],
        ];
        self.inner.add_scalar_pod(&flat);
    }
    
    /// Add a string sample.
    fn addSampleString(&mut self, value: &str) {
        // Strings in Alembic are stored as null-terminated bytes
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0); // null terminator
        self.inner.add_scalar_sample(&bytes);
    }
    
    fn __repr__(&self) -> String {
        format!("<OScalarProperty '{}'>", self.inner.name)
    }
}

/// Python wrapper for output array property.
#[pyclass(name = "OArrayProperty")]
pub struct PyOArrayProperty {
    inner: OProperty,
}

#[pymethods]
impl PyOArrayProperty {
    /// Create a new array property.
    #[new]
    #[pyo3(signature = (name, data_type, time_sampling_index=0))]
    fn new(name: &str, data_type: &str, time_sampling_index: u32) -> PyResult<Self> {
        let dt = parse_data_type(data_type)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown data type: {}", data_type)))?;
        
        let mut prop = OProperty::array(name, dt);
        prop.time_sampling_index = time_sampling_index;
        
        Ok(Self { inner: prop })
    }
    
    /// Get property name.
    fn getName(&self) -> &str {
        &self.inner.name
    }
    
    /// Add int array sample.
    fn addSampleInts(&mut self, values: Vec<i32>) {
        self.inner.add_array_pod(&values);
    }
    
    /// Add float array sample.
    fn addSampleFloats(&mut self, values: Vec<f32>) {
        self.inner.add_array_pod(&values);
    }
    
    /// Add double array sample.
    fn addSampleDoubles(&mut self, values: Vec<f64>) {
        self.inner.add_array_pod(&values);
    }
    
    /// Add Vec2f array sample.
    fn addSampleVec2fs(&mut self, values: Vec<[f32; 2]>) {
        let data: &[u8] = bytemuck::cast_slice(&values);
        self.inner.add_array_sample(data, &[values.len()]);
    }
    
    /// Add Vec3f array sample.
    fn addSampleVec3fs(&mut self, values: Vec<[f32; 3]>) {
        let data: &[u8] = bytemuck::cast_slice(&values);
        self.inner.add_array_sample(data, &[values.len()]);
    }
    
    /// Add Vec4f array sample.
    fn addSampleVec4fs(&mut self, values: Vec<[f32; 4]>) {
        let data: &[u8] = bytemuck::cast_slice(&values);
        self.inner.add_array_sample(data, &[values.len()]);
    }
    
    /// Add u32 array sample (for indices).
    fn addSampleUints(&mut self, values: Vec<u32>) {
        self.inner.add_array_pod(&values);
    }
    
    /// Add string array sample.
    fn addSampleStrings(&mut self, values: Vec<String>) {
        // Strings are stored as concatenated null-terminated bytes
        let mut bytes = Vec::new();
        for s in &values {
            bytes.extend_from_slice(s.as_bytes());
            bytes.push(0); // null terminator
        }
        self.inner.add_array_sample(&bytes, &[values.len()]);
    }
    
    fn __repr__(&self) -> String {
        format!("<OArrayProperty '{}'>", self.inner.name)
    }
}

/// Python wrapper for output compound property.
#[pyclass(name = "OCompoundProperty")]
pub struct PyOCompoundProperty {
    inner: OProperty,
}

#[pymethods]
impl PyOCompoundProperty {
    /// Create a new compound property.
    #[new]
    fn new(name: &str) -> Self {
        Self { inner: OProperty::compound(name) }
    }
    
    /// Get property name.
    fn getName(&self) -> &str {
        &self.inner.name
    }
    
    /// Add a scalar property as child.
    fn addScalar(&mut self, prop: &PyOScalarProperty) {
        self.inner.add_child(prop.inner.clone());
    }
    
    /// Add an array property as child.
    fn addArray(&mut self, prop: &PyOArrayProperty) {
        self.inner.add_child(prop.inner.clone());
    }
    
    /// Add a compound property as child.
    fn addCompound(&mut self, prop: &PyOCompoundProperty) {
        self.inner.add_child(prop.inner.clone());
    }
    
    fn __repr__(&self) -> String {
        format!("<OCompoundProperty '{}'>", self.inner.name)
    }
}
