//! Python bindings for Alembic objects.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

use crate::abc::IArchive;
use crate::geom::{IPolyMesh, ISubD, ICurves, IPoints, ICamera, ILight, IXform, INuPatch};
use super::geom::{
    PyPolyMeshSample, PySubDSample, PyCurvesSample, PyPointsSample,
    PyCameraSample, PyXformSample,
};
use super::time_sampling::PyTimeSampling;

/// Python wrapper for IObject.
/// 
/// Stores path and traverses fresh each time (Rust ownership workaround).
#[pyclass(name = "IObject")]
pub struct PyIObject {
    pub(crate) archive: Arc<IArchive>,
    pub(crate) path: Vec<String>,
}

impl PyIObject {
    /// Create from archive root.
    pub fn from_archive(archive: Arc<IArchive>) -> Self {
        Self {
            archive,
            path: Vec::new(),
        }
    }
    
    /// Execute closure with resolved object.
    fn with_object<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&crate::abc::IObject) -> Option<T>,
    {
        let root = self.archive.root();
        
        if self.path.is_empty() {
            return f(&root);
        }
        
        // Traverse path
        let mut current = root;
        for name in &self.path {
            current = current.child_by_name(name)?;
        }
        f(&current)
    }
}

#[pymethods]
impl PyIObject {
    /// Get object name.
    fn getName(&self) -> String {
        self.path.last().cloned().unwrap_or_else(|| "ABC".to_string())
    }
    
    /// Get full path.
    fn getFullName(&self) -> String {
        if self.path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.path.join("/"))
        }
    }
    
    /// Get number of children.
    fn getNumChildren(&self) -> usize {
        self.with_object(|obj| Some(obj.num_children())).unwrap_or(0)
    }
    
    /// Get child by index.
    fn getChild(&self, index: usize) -> PyResult<PyIObject> {
        let child_name = self.with_object(|obj| {
            obj.child(index).map(|c| c.name().to_string())
        }).ok_or_else(|| PyValueError::new_err("Child index out of range"))?;
        
        let mut new_path = self.path.clone();
        new_path.push(child_name);
        
        Ok(PyIObject {
            archive: self.archive.clone(),
            path: new_path,
        })
    }
    
    /// Get child by name.
    #[pyo3(signature = (name))]
    fn getChildByName(&self, name: &str) -> PyResult<PyIObject> {
        let exists = self.with_object(|obj| {
            Some(obj.child_by_name(name).is_some())
        }).unwrap_or(false);
        
        if !exists {
            return Err(PyValueError::new_err(format!("Child '{}' not found", name)));
        }
        
        let mut new_path = self.path.clone();
        new_path.push(name.to_string());
        
        Ok(PyIObject {
            archive: self.archive.clone(),
            path: new_path,
        })
    }
    
    /// Get parent object.
    fn getParent(&self) -> Option<PyIObject> {
        if self.path.is_empty() {
            return None;
        }
        
        let mut parent_path = self.path.clone();
        parent_path.pop();
        
        Some(PyIObject {
            archive: self.archive.clone(),
            path: parent_path,
        })
    }
    
    /// Check if valid.
    fn valid(&self) -> bool {
        self.with_object(|_| Some(true)).unwrap_or(false)
    }
    
    /// Get schema type string.
    fn getSchemaType(&self) -> String {
        self.with_object(|obj| {
            Some(obj.meta_data().get("schema").unwrap_or_default())
        }).unwrap_or_default()
    }
    
    /// Children as list.
    #[getter]
    fn children(&self) -> Vec<PyIObject> {
        let num = self.getNumChildren();
        (0..num)
            .filter_map(|i| self.getChild(i).ok())
            .collect()
    }
    
    /// Get number of samples.
    fn getNumSamples(&self) -> usize {
        self.with_object(|obj| {
            if let Some(m) = IPolyMesh::new(obj) { return Some(m.num_samples()); }
            if let Some(x) = IXform::new(obj) { return Some(x.num_samples()); }
            if let Some(c) = ICamera::new(obj) { return Some(c.num_samples()); }
            if let Some(p) = IPoints::new(obj) { return Some(p.num_samples()); }
            if let Some(c) = ICurves::new(obj) { return Some(c.num_samples()); }
            if let Some(s) = ISubD::new(obj) { return Some(s.num_samples()); }
            Some(0)
        }).unwrap_or(0)
    }
    
    // ========================================================================
    // Type checks
    // ========================================================================
    
    fn isPolyMesh(&self) -> bool {
        self.with_object(|obj| Some(IPolyMesh::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isSubD(&self) -> bool {
        self.with_object(|obj| Some(ISubD::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isCurves(&self) -> bool {
        self.with_object(|obj| Some(ICurves::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isPoints(&self) -> bool {
        self.with_object(|obj| Some(IPoints::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isCamera(&self) -> bool {
        self.with_object(|obj| Some(ICamera::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isLight(&self) -> bool {
        self.with_object(|obj| Some(ILight::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isXform(&self) -> bool {
        self.with_object(|obj| Some(IXform::new(obj).is_some())).unwrap_or(false)
    }
    
    fn isNuPatch(&self) -> bool {
        self.with_object(|obj| Some(INuPatch::new(obj).is_some())).unwrap_or(false)
    }
    
    // ========================================================================
    // Get samples
    // ========================================================================
    
    /// Get PolyMesh sample at index.
    #[pyo3(signature = (index=0))]
    fn getPolyMeshSample(&self, index: usize) -> PyResult<PyPolyMeshSample> {
        self.with_object(|obj| {
            let mesh = IPolyMesh::new(obj)?;
            mesh.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a PolyMesh or failed to get sample"))
    }
    
    /// Get SubD sample at index.
    #[pyo3(signature = (index=0))]
    fn getSubDSample(&self, index: usize) -> PyResult<PySubDSample> {
        self.with_object(|obj| {
            let subd = ISubD::new(obj)?;
            subd.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a SubD or failed to get sample"))
    }
    
    /// Get Curves sample at index.
    #[pyo3(signature = (index=0))]
    fn getCurvesSample(&self, index: usize) -> PyResult<PyCurvesSample> {
        self.with_object(|obj| {
            let curves = ICurves::new(obj)?;
            curves.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a Curves or failed to get sample"))
    }
    
    /// Get Points sample at index.
    #[pyo3(signature = (index=0))]
    fn getPointsSample(&self, index: usize) -> PyResult<PyPointsSample> {
        self.with_object(|obj| {
            let points = IPoints::new(obj)?;
            points.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a Points or failed to get sample"))
    }
    
    /// Get Camera sample at index.
    #[pyo3(signature = (index=0))]
    fn getCameraSample(&self, index: usize) -> PyResult<PyCameraSample> {
        self.with_object(|obj| {
            let camera = ICamera::new(obj)?;
            camera.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a Camera or failed to get sample"))
    }
    
    /// Get Xform sample at index.
    #[pyo3(signature = (index=0))]
    fn getXformSample(&self, index: usize) -> PyResult<PyXformSample> {
        self.with_object(|obj| {
            let xform = IXform::new(obj)?;
            xform.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not an Xform or failed to get sample"))
    }
    
    // ========================================================================
    // Convenience methods (backward compatible)
    // ========================================================================
    
    /// Get positions if this is a PolyMesh.
    #[pyo3(signature = (index=0))]
    fn getPositions(&self, index: usize) -> Option<Vec<[f32; 3]>> {
        self.getPolyMeshSample(index).ok().map(|s| s.positions())
    }
    
    /// Get face counts if this is a PolyMesh.
    #[pyo3(signature = (index=0))]
    fn getFaceCounts(&self, index: usize) -> Option<Vec<i32>> {
        self.getPolyMeshSample(index).ok().map(|s| s.faceCounts())
    }
    
    /// Get face indices if this is a PolyMesh.
    #[pyo3(signature = (index=0))]
    fn getFaceIndices(&self, index: usize) -> Option<Vec<i32>> {
        self.getPolyMeshSample(index).ok().map(|s| s.faceIndices())
    }
    
    /// Get 4x4 transformation matrix if this is an Xform.
    #[pyo3(signature = (index=0))]
    fn getMatrix(&self, index: usize) -> Option<[[f64; 4]; 4]> {
        self.getXformSample(index).ok().map(|s| s.matrix())
    }
    
    fn __repr__(&self) -> String {
        let schema = self.getSchemaType();
        let type_str = if schema.contains("PolyMesh") { "PolyMesh" }
            else if schema.contains("SubD") { "SubD" }
            else if schema.contains("Curves") { "Curves" }
            else if schema.contains("Points") { "Points" }
            else if schema.contains("Camera") { "Camera" }
            else if schema.contains("Light") { "Light" }
            else if schema.contains("Xform") { "Xform" }
            else { "Object" };
        format!("<IObject '{}' [{}]>", self.getFullName(), type_str)
    }
    
    fn __len__(&self) -> usize {
        self.getNumChildren()
    }
    
    /// Iterator support.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyIObjectIter>> {
        let children: Vec<PyIObject> = slf.children();
        Py::new(slf.py(), PyIObjectIter { children, index: 0 })
    }
}

/// Iterator for IObject children.
#[pyclass]
pub struct PyIObjectIter {
    children: Vec<PyIObject>,
    index: usize,
}

#[pymethods]
impl PyIObjectIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyIObject> {
        if slf.index < slf.children.len() {
            let child = slf.children[slf.index].clone();
            slf.index += 1;
            Some(child)
        } else {
            None
        }
    }
}

impl Clone for PyIObject {
    fn clone(&self) -> Self {
        Self {
            archive: self.archive.clone(),
            path: self.path.clone(),
        }
    }
}
