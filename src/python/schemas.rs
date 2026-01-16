//! Python bindings for Alembic schema readers (IPolyMesh, IXform, etc.)
//!
//! Provides original Alembic-style API:
//! ```python
//! mesh = AbcGeom.IPolyMesh(obj)
//! schema = mesh.getSchema()
//! sample = schema.getValue()
//! ```

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

use crate::abc::IArchive;
use crate::geom::{IPolyMesh, ISubD, ICurves, IPoints, ICamera, ILight, IXform, INuPatch, IFaceSet};
use super::geom::{
    PyPolyMeshSample, PySubDSample, PyCurvesSample, PyPointsSample,
    PyCameraSample, PyXformSample, PyLightSample, PyNuPatchSample, PyFaceSetSample,
};
use super::object::PyIObject;

// ============================================================================
// IPolyMesh
// ============================================================================

/// Python wrapper for IPolyMesh schema.
#[pyclass(name = "IPolyMesh")]
pub struct PyIPolyMesh {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIPolyMesh {
    /// Create IPolyMesh from IObject.
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        // Verify it's actually a PolyMesh
        if !obj.isPolyMesh() {
            return Err(PyValueError::new_err("Object is not a PolyMesh"));
        }
        Ok(Self {
            archive: obj.archive.clone(),
            path: obj.path.clone(),
        })
    }
    
    /// Get the schema for this PolyMesh.
    fn getSchema(&self) -> PyIPolyMeshSchema {
        PyIPolyMeshSchema {
            archive: self.archive.clone(),
            path: self.path.clone(),
        }
    }
    
    /// Check if valid.
    fn valid(&self) -> bool {
        self.with_mesh(|_| Some(true)).unwrap_or(false)
    }
    
    /// Get object name.
    fn getName(&self) -> String {
        self.path.last().cloned().unwrap_or_default()
    }
    
    /// Get full path.
    fn getFullName(&self) -> String {
        if self.path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.path.join("/"))
        }
    }
    
    fn __repr__(&self) -> String {
        format!("<IPolyMesh '{}'>", self.getName())
    }
}

impl PyIPolyMesh {
    fn with_mesh<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IPolyMesh<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&IPolyMesh<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                let mesh = IPolyMesh::new(&obj)?;
                f(&mesh)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse(child, &path[1..], f)
            }
        }
        
        traverse(root, &self.path, f)
    }
}

/// IPolyMesh schema accessor.
#[pyclass(name = "IPolyMeshSchema")]
pub struct PyIPolyMeshSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIPolyMeshSchema {
    /// Get number of samples.
    fn getNumSamples(&self) -> usize {
        self.with_mesh(|m| Some(m.getNumSamples())).unwrap_or(0)
    }
    
    /// Check if constant (single sample).
    fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Get sample at index (default 0).
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyPolyMeshSample> {
        self.with_mesh(|m| m.getSample(index).ok().map(|s| s.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    /// Get time sampling index.
    fn getTimeSamplingIndex(&self) -> u32 {
        self.with_mesh(|m| Some(m.getTimeSamplingIndex())).unwrap_or(0)
    }
    
    fn __repr__(&self) -> String {
        format!("<IPolyMeshSchema {} samples>", self.getNumSamples())
    }
}

impl PyIPolyMeshSchema {
    fn with_mesh<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IPolyMesh<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&IPolyMesh<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                let mesh = IPolyMesh::new(&obj)?;
                f(&mesh)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse(child, &path[1..], f)
            }
        }
        
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// IXform
// ============================================================================

/// Python wrapper for IXform schema.
#[pyclass(name = "IXform")]
pub struct PyIXform {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIXform {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isXform() {
            return Err(PyValueError::new_err("Object is not an Xform"));
        }
        Ok(Self {
            archive: obj.archive.clone(),
            path: obj.path.clone(),
        })
    }
    
    fn getSchema(&self) -> PyIXformSchema {
        PyIXformSchema {
            archive: self.archive.clone(),
            path: self.path.clone(),
        }
    }
    
    fn valid(&self) -> bool {
        self.with_xform(|_| Some(true)).unwrap_or(false)
    }
    
    fn getName(&self) -> String {
        self.path.last().cloned().unwrap_or_default()
    }
    
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    
    fn __repr__(&self) -> String {
        format!("<IXform '{}'>", self.getName())
    }
}

impl PyIXform {
    fn with_xform<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IXform<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&IXform<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                let x = IXform::new(&obj)?;
                f(&x)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse(child, &path[1..], f)
            }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "IXformSchema")]
pub struct PyIXformSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIXformSchema {
    fn getNumSamples(&self) -> usize {
        self.with_xform(|x| Some(x.getNumSamples())).unwrap_or(0)
    }
    
    fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyXformSample> {
        self.with_xform(|x| x.getSample(index).ok().map(|s| s.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 {
        self.with_xform(|x| Some(x.getTimeSamplingIndex())).unwrap_or(0)
    }
    
    /// Check if this transform inherits from parent.
    #[pyo3(signature = (index=0))]
    fn getInheritsXforms(&self, index: usize) -> bool {
        self.with_xform(|x| x.getSample(index).ok().map(|s| s.inherits)).unwrap_or(true)
    }
    
    fn __repr__(&self) -> String {
        format!("<IXformSchema {} samples>", self.getNumSamples())
    }
}

impl PyIXformSchema {
    fn with_xform<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IXform<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&IXform<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                let x = IXform::new(&obj)?;
                f(&x)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse(child, &path[1..], f)
            }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// ISubD
// ============================================================================

#[pyclass(name = "ISubD")]
pub struct PyISubD {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyISubD {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isSubD() {
            return Err(PyValueError::new_err("Object is not a SubD"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyISubDSchema {
        PyISubDSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_subd(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<ISubD '{}'>", self.getName()) }
}

impl PyISubD {
    fn with_subd<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ISubD<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ISubD<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ISubD::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "ISubDSchema")]
pub struct PyISubDSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyISubDSchema {
    fn getNumSamples(&self) -> usize { self.with_subd(|s| Some(s.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PySubDSample> {
        self.with_subd(|s| s.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_subd(|s| Some(s.getTimeSamplingIndex())).unwrap_or(0) }
    fn __repr__(&self) -> String { format!("<ISubDSchema {} samples>", self.getNumSamples()) }
}

impl PyISubDSchema {
    fn with_subd<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ISubD<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ISubD<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ISubD::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// ICurves
// ============================================================================

#[pyclass(name = "ICurves")]
pub struct PyICurves {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyICurves {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isCurves() {
            return Err(PyValueError::new_err("Object is not Curves"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyICurvesSchema {
        PyICurvesSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_curves(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<ICurves '{}'>", self.getName()) }
}

impl PyICurves {
    fn with_curves<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ICurves<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ICurves<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ICurves::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "ICurvesSchema")]
pub struct PyICurvesSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyICurvesSchema {
    fn getNumSamples(&self) -> usize { self.with_curves(|c| Some(c.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyCurvesSample> {
        self.with_curves(|c| c.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_curves(|c| Some(c.getTimeSamplingIndex())).unwrap_or(0) }
    fn __repr__(&self) -> String { format!("<ICurvesSchema {} samples>", self.getNumSamples()) }
}

impl PyICurvesSchema {
    fn with_curves<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ICurves<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ICurves<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ICurves::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// IPoints
// ============================================================================

#[pyclass(name = "IPoints")]
pub struct PyIPoints {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIPoints {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isPoints() {
            return Err(PyValueError::new_err("Object is not Points"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyIPointsSchema {
        PyIPointsSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_points(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<IPoints '{}'>", self.getName()) }
}

impl PyIPoints {
    fn with_points<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&IPoints<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&IPoints<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&IPoints::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "IPointsSchema")]
pub struct PyIPointsSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIPointsSchema {
    fn getNumSamples(&self) -> usize { self.with_points(|p| Some(p.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyPointsSample> {
        self.with_points(|p| p.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_points(|p| Some(p.getTimeSamplingIndex())).unwrap_or(0) }
    fn __repr__(&self) -> String { format!("<IPointsSchema {} samples>", self.getNumSamples()) }
}

impl PyIPointsSchema {
    fn with_points<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&IPoints<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&IPoints<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&IPoints::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// ICamera
// ============================================================================

#[pyclass(name = "ICamera")]
pub struct PyICamera {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyICamera {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isCamera() {
            return Err(PyValueError::new_err("Object is not a Camera"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyICameraSchema {
        PyICameraSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_camera(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<ICamera '{}'>", self.getName()) }
}

impl PyICamera {
    fn with_camera<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ICamera<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ICamera<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ICamera::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "ICameraSchema")]
pub struct PyICameraSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyICameraSchema {
    fn getNumSamples(&self) -> usize { self.with_camera(|c| Some(c.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyCameraSample> {
        self.with_camera(|c| c.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_camera(|c| Some(c.getTimeSamplingIndex())).unwrap_or(0) }
    fn __repr__(&self) -> String { format!("<ICameraSchema {} samples>", self.getNumSamples()) }
}

impl PyICameraSchema {
    fn with_camera<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ICamera<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ICamera<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ICamera::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// ILight
// ============================================================================

#[pyclass(name = "ILight")]
pub struct PyILight {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyILight {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isLight() {
            return Err(PyValueError::new_err("Object is not a Light"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyILightSchema {
        PyILightSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_light(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<ILight '{}'>", self.getName()) }
}

impl PyILight {
    fn with_light<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ILight<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ILight<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ILight::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "ILightSchema")]
pub struct PyILightSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyILightSchema {
    fn getNumSamples(&self) -> usize { self.with_light(|l| Some(l.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyLightSample> {
        self.with_light(|l| l.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_light(|l| Some(l.getTimeSamplingIndex())).unwrap_or(0) }
    fn __repr__(&self) -> String { format!("<ILightSchema {} samples>", self.getNumSamples()) }
}

impl PyILightSchema {
    fn with_light<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&ILight<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&ILight<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&ILight::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// INuPatch
// ============================================================================

#[pyclass(name = "INuPatch")]
pub struct PyINuPatch {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyINuPatch {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isNuPatch() {
            return Err(PyValueError::new_err("Object is not a NuPatch"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyINuPatchSchema {
        PyINuPatchSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_nupatch(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<INuPatch '{}'>", self.getName()) }
}

impl PyINuPatch {
    fn with_nupatch<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&INuPatch<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&INuPatch<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&INuPatch::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

#[pyclass(name = "INuPatchSchema")]
pub struct PyINuPatchSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyINuPatchSchema {
    fn getNumSamples(&self) -> usize { self.with_nupatch(|n| Some(n.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyNuPatchSample> {
        self.with_nupatch(|n| n.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_nupatch(|n| Some(n.getTimeSamplingIndex())).unwrap_or(0) }
    fn __repr__(&self) -> String { format!("<INuPatchSchema {} samples>", self.getNumSamples()) }
}

impl PyINuPatchSchema {
    fn with_nupatch<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&INuPatch<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&INuPatch<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&INuPatch::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

// ============================================================================
// IFaceSet
// ============================================================================

#[pyclass(name = "IFaceSetSchema")]
pub struct PyIFaceSetSchema {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIFaceSetSchema {
    fn getNumSamples(&self) -> usize { self.with_faceset(|f| Some(f.getNumSamples())).unwrap_or(0) }
    fn isConstant(&self) -> bool { self.getNumSamples() <= 1 }
    
    #[pyo3(signature = (index=0))]
    fn getValue(&self, index: usize) -> PyResult<PyFaceSetSample> {
        self.with_faceset(|f| f.getSample(index).ok().map(|v| v.into()))
            .ok_or_else(|| PyValueError::new_err("Failed to get sample"))
    }
    
    fn getTimeSamplingIndex(&self) -> u32 { self.with_faceset(|f| Some(f.getTimeSamplingIndex())).unwrap_or(0) }
    
    /// Get face exclusivity setting.
    fn getFaceExclusivity(&self) -> String {
        self.with_faceset(|f| Some(format!("{:?}", f.face_exclusivity())))
            .unwrap_or_else(|| "NonExclusive".to_string())
    }
    
    fn __repr__(&self) -> String { format!("<IFaceSetSchema {} samples>", self.getNumSamples()) }
}

impl PyIFaceSetSchema {
    fn with_faceset<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&IFaceSet<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&IFaceSet<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&IFaceSet::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}

/// Schema-style wrapper for IFaceSet (matches original Alembic API).
#[pyclass(name = "IFaceSetTyped")]
pub struct PyIFaceSetTyped {
    archive: Arc<IArchive>,
    path: Vec<String>,
}

#[pymethods]
impl PyIFaceSetTyped {
    #[new]
    fn new(obj: &PyIObject) -> PyResult<Self> {
        if !obj.isFaceSet() {
            return Err(PyValueError::new_err("Object is not a FaceSet"));
        }
        Ok(Self { archive: obj.archive.clone(), path: obj.path.clone() })
    }
    
    fn getSchema(&self) -> PyIFaceSetSchema {
        PyIFaceSetSchema { archive: self.archive.clone(), path: self.path.clone() }
    }
    
    fn valid(&self) -> bool { self.with_faceset(|_| Some(true)).unwrap_or(false) }
    fn getName(&self) -> String { self.path.last().cloned().unwrap_or_default() }
    fn getFullName(&self) -> String {
        if self.path.is_empty() { "/".to_string() }
        else { format!("/{}", self.path.join("/")) }
    }
    fn __repr__(&self) -> String { format!("<IFaceSetTyped '{}'>", self.getName()) }
}

impl PyIFaceSetTyped {
    fn with_faceset<T, F>(&self, f: F) -> Option<T>
    where F: FnOnce(&IFaceSet<'_>) -> Option<T> {
        let root = self.archive.getTop();
        fn traverse<'a, T>(obj: crate::abc::IObject<'a>, path: &[String], f: impl FnOnce(&IFaceSet<'_>) -> Option<T>) -> Option<T> {
            if path.is_empty() { f(&IFaceSet::new(&obj)?) }
            else { traverse(obj.getChildByName(&path[0])?, &path[1..], f) }
        }
        traverse(root, &self.path, f)
    }
}
