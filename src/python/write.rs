//! Python bindings for Alembic write API.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};
use std::sync::{Arc, Mutex};

use crate::ogawa::writer::{OArchive, OObject, OPolyMesh, OXform, OPolyMeshSample, OXformSample};
use crate::core::TimeSampling;

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
        let guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_ref().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        Ok(archive.name().to_string())
    }
    
    /// Set application writer string.
    fn setApplicationWriter(&self, writer: &str) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        archive.set_application_writer(writer);
        Ok(())
    }
    
    /// Set compression hint (-1 = no compression, 0-9 = compression level).
    fn setCompressionHint(&self, hint: i32) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        archive.set_compression_hint(hint);
        Ok(())
    }
    
    /// Enable/disable deduplication.
    fn setDedupEnabled(&self, enabled: bool) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        archive.set_dedup_enabled(enabled);
        Ok(())
    }
    
    /// Add uniform time sampling (fps-based).
    /// Returns time sampling index.
    #[pyo3(signature = (fps, start_time=0.0))]
    fn addUniformTimeSampling(&self, fps: f64, start_time: f64) -> PyResult<u32> {
        let time_per_cycle = 1.0 / fps;
        let ts = TimeSampling::uniform(time_per_cycle, start_time);
        
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        Ok(archive.add_time_sampling(ts))
    }
    
    /// Add acyclic time sampling with explicit frame times.
    /// Returns time sampling index.
    fn addAcyclicTimeSampling(&self, times: Vec<f64>) -> PyResult<u32> {
        let ts = TimeSampling::acyclic(times);
        
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        Ok(archive.add_time_sampling(ts))
    }
    
    /// Add cyclic time sampling.
    /// Returns time sampling index.
    fn addCyclicTimeSampling(&self, time_per_cycle: f64, times: Vec<f64>) -> PyResult<u32> {
        let ts = TimeSampling::cyclic(time_per_cycle, times);
        
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        Ok(archive.add_time_sampling(ts))
    }
    
    /// Write the root object hierarchy and finalize.
    fn writeArchive(&self, root: &PyOObject) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        
        archive.write_archive(&root.inner)
            .map_err(|e| PyIOError::new_err(format!("Failed to write archive: {}", e)))?;
        
        Ok(())
    }
    
    /// Write PolyMesh hierarchy and finalize.
    fn writePolyMesh(&self, mesh: &mut PyOPolyMesh) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        
        // Take ownership of inner and build
        let obj = std::mem::replace(&mut mesh.inner, OPolyMesh::new("_empty")).build();
        
        archive.write_archive(&obj)
            .map_err(|e| PyIOError::new_err(format!("Failed to write archive: {}", e)))?;
        
        Ok(())
    }
    
    /// Write Xform hierarchy and finalize.
    fn writeXform(&self, xform: &mut PyOXform) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
        let archive = guard.as_mut().ok_or_else(|| PyValueError::new_err("Archive closed"))?;
        
        // Take ownership and build
        let obj = std::mem::replace(&mut xform.inner, OXform::new("_empty")).build();
        
        archive.write_archive(&obj)
            .map_err(|e| PyIOError::new_err(format!("Failed to write archive: {}", e)))?;
        
        Ok(())
    }
    
    /// Close the archive.
    fn close(&self) -> PyResult<()> {
        let mut guard = self.archive.lock().map_err(|_| PyValueError::new_err("Lock poisoned"))?;
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
        Ok(false) // Don't suppress exceptions
    }
    
    fn __repr__(&self) -> String {
        let guard = self.archive.lock();
        if let Ok(g) = guard {
            if let Some(archive) = g.as_ref() {
                return format!("<OArchive '{}'>", archive.name());
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
        Self {
            inner: OObject::new(name),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.inner.name
    }
    
    /// Add a child object.
    fn addChild(&mut self, child: &PyOObject) {
        self.inner.add_child(child.inner.clone());
    }
    
    /// Add a PolyMesh as child.
    fn addPolyMesh(&mut self, mesh: &mut PyOPolyMesh) {
        let obj = std::mem::replace(&mut mesh.inner, OPolyMesh::new("_empty")).build();
        self.inner.add_child(obj);
    }
    
    /// Add an Xform as child.
    fn addXform(&mut self, xform: &mut PyOXform) {
        let obj = std::mem::replace(&mut xform.inner, OXform::new("_empty")).build();
        self.inner.add_child(obj);
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
    pub(crate) inner: OPolyMesh,
    name: String,
}

#[pymethods]
impl PyOPolyMesh {
    /// Create a new PolyMesh with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: OPolyMesh::new(name),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
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
        // Convert positions to glam::Vec3
        let pos: Vec<glam::Vec3> = positions.iter()
            .map(|p| glam::Vec3::new(p[0], p[1], p[2]))
            .collect();
        
        let mut sample = OPolyMeshSample::new(pos, face_counts, face_indices);
        
        // Add normals if provided
        if let Some(norms) = normals {
            sample.normals = Some(norms.iter()
                .map(|n| glam::Vec3::new(n[0], n[1], n[2]))
                .collect());
        }
        
        // Add UVs if provided
        if let Some(uv_data) = uvs {
            sample.uvs = Some(uv_data.iter()
                .map(|u| glam::Vec2::new(u[0], u[1]))
                .collect());
        }
        
        self.inner.add_sample(&sample);
        Ok(())
    }
    
    /// Add a child object (note: mesh hierarchy is uncommon, consider using Xform).
    fn addChild(&mut self, _child: &PyOObject) {
        // OPolyMesh doesn't expose add_child directly
        // Children should typically be added via Xform hierarchy
    }
    
    fn __repr__(&self) -> String {
        format!("<OPolyMesh '{}'>", self.name)
    }
}

// ============================================================================
// OXform wrapper
// ============================================================================

/// Python wrapper for output Xform (transform).
#[pyclass(name = "OXform")]
pub struct PyOXform {
    pub(crate) inner: OXform,
    name: String,
}

#[pymethods]
impl PyOXform {
    /// Create a new Xform with name.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: OXform::new(name),
            name: name.to_string(),
        }
    }
    
    /// Get object name.
    fn getName(&self) -> &str {
        &self.name
    }
    
    /// Add identity sample.
    fn addIdentitySample(&mut self) {
        self.inner.add_sample(OXformSample::identity());
    }
    
    /// Add sample from 4x4 matrix (row-major, f32).
    #[pyo3(signature = (matrix, inherits=true))]
    fn addMatrixSample(&mut self, matrix: [[f32; 4]; 4], inherits: bool) {
        let m = glam::Mat4::from_cols_array_2d(&matrix);
        self.inner.add_sample(OXformSample::from_matrix(m, inherits));
    }
    
    /// Add sample from translation.
    fn addTranslationSample(&mut self, x: f32, y: f32, z: f32) {
        let m = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
        self.inner.add_sample(OXformSample::from_matrix(m, true));
    }
    
    /// Add sample from scale.
    fn addScaleSample(&mut self, x: f32, y: f32, z: f32) {
        let m = glam::Mat4::from_scale(glam::Vec3::new(x, y, z));
        self.inner.add_sample(OXformSample::from_matrix(m, true));
    }
    
    /// Add a child object.
    fn addChild(&mut self, child: &PyOObject) {
        self.inner.add_child(child.inner.clone());
    }
    
    /// Add a PolyMesh as child.
    fn addPolyMesh(&mut self, mesh: &mut PyOPolyMesh) {
        let obj = std::mem::replace(&mut mesh.inner, OPolyMesh::new("_empty")).build();
        self.inner.add_child(obj);
    }
    
    /// Add another Xform as child.
    fn addXformChild(&mut self, xform: &mut PyOXform) {
        let obj = std::mem::replace(&mut xform.inner, OXform::new("_empty")).build();
        self.inner.add_child(obj);
    }
    
    fn __repr__(&self) -> String {
        format!("<OXform '{}'>", self.name)
    }
}
