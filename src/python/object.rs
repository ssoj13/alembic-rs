//! Python bindings for Alembic objects.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

use crate::abc::IArchive;
use crate::geom::{IPolyMesh, ISubD, ICurves, IPoints, ICamera, ILight, IXform, INuPatch, IFaceSet, IGeomParam};
use crate::geom::visibility::{get_visibility, is_visible};
use super::geom::{
    PyPolyMeshSample, PySubDSample, PyCurvesSample, PyPointsSample,
    PyCameraSample, PyXformSample, PyLightSample, PyNuPatchSample,
    PyFaceSetSample, PyIFaceSet, PyIGeomParam, PyObjectVisibility,
};
use super::properties::PyICompoundProperty;

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
    
    /// Execute closure with resolved object (recursive traversal).
    fn with_object<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&crate::abc::IObject) -> Option<T>,
    {
        let root = self.archive.getTop();
        
        if self.path.is_empty() {
            return f(&root);
        }
        
        // Use recursive helper to avoid borrow checker issues
        fn traverse<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&crate::abc::IObject) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                f(&obj)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse(child, &path[1..], f)
            }
        }
        
        traverse(root, &self.path, f)
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
        self.with_object(|obj| Some(obj.getNumChildren())).unwrap_or(0)
    }
    
    /// Get child by index.
    fn getChild(&self, index: usize) -> PyResult<PyIObject> {
        let child_name = self.with_object(|obj| {
            obj.getChild(index).map(|c| c.getName().to_string())
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
            Some(obj.getChildByName(name).is_some())
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
            Some(obj.getMetaData().get("schema").unwrap_or_default().to_string())
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
            if let Some(m) = IPolyMesh::new(obj) { return Some(m.getNumSamples()); }
            if let Some(x) = IXform::new(obj) { return Some(x.getNumSamples()); }
            if let Some(c) = ICamera::new(obj) { return Some(c.getNumSamples()); }
            if let Some(p) = IPoints::new(obj) { return Some(p.getNumSamples()); }
            if let Some(c) = ICurves::new(obj) { return Some(c.getNumSamples()); }
            if let Some(s) = ISubD::new(obj) { return Some(s.getNumSamples()); }
            Some(0)
        }).unwrap_or(0)
    }
    
    // ========================================================================
    // Type checks
    // ========================================================================
    
    pub fn isPolyMesh(&self) -> bool {
        self.with_object(|obj| Some(IPolyMesh::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isSubD(&self) -> bool {
        self.with_object(|obj| Some(ISubD::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isCurves(&self) -> bool {
        self.with_object(|obj| Some(ICurves::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isPoints(&self) -> bool {
        self.with_object(|obj| Some(IPoints::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isCamera(&self) -> bool {
        self.with_object(|obj| Some(ICamera::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isLight(&self) -> bool {
        self.with_object(|obj| Some(ILight::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isXform(&self) -> bool {
        self.with_object(|obj| Some(IXform::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isNuPatch(&self) -> bool {
        self.with_object(|obj| Some(INuPatch::new(obj).is_some())).unwrap_or(false)
    }
    
    pub fn isFaceSet(&self) -> bool {
        self.with_object(|obj| Some(IFaceSet::new(obj).is_some())).unwrap_or(false)
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
    
    /// Get Light sample at index.
    #[pyo3(signature = (index=0))]
    fn getLightSample(&self, index: usize) -> PyResult<PyLightSample> {
        self.with_object(|obj| {
            let light = ILight::new(obj)?;
            light.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a Light or failed to get sample"))
    }
    
    /// Get NuPatch sample at index.
    #[pyo3(signature = (index=0))]
    fn getNuPatchSample(&self, index: usize) -> PyResult<PyNuPatchSample> {
        self.with_object(|obj| {
            let nupatch = INuPatch::new(obj)?;
            nupatch.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a NuPatch or failed to get sample"))
    }
    
    /// Get FaceSet sample at index.
    #[pyo3(signature = (index=0))]
    fn getFaceSetSample(&self, index: usize) -> PyResult<PyFaceSetSample> {
        self.with_object(|obj| {
            let faceset = IFaceSet::new(obj)?;
            faceset.get_sample(index).ok().map(|s| s.into())
        }).ok_or_else(|| PyValueError::new_err("Not a FaceSet or failed to get sample"))
    }
    
    /// Get list of FaceSet child names (for meshes).
    fn getFaceSetNames(&self) -> Vec<String> {
        self.with_object(|obj| {
            let mut names = Vec::new();
            for i in 0..obj.getNumChildren() {
                if let Some(child) = obj.getChild(i) {
                    if IFaceSet::new(&child).is_some() {
                        names.push(child.getName().to_string());
                    }
                }
            }
            Some(names)
        }).unwrap_or_default()
    }
    
    /// Get FaceSet child by name.
    fn getFaceSet(&self, name: &str) -> PyResult<PyIFaceSet> {
        let exists = self.with_object(|obj| {
            if let Some(child) = obj.getChildByName(name) {
                if IFaceSet::new(&child).is_some() {
                    return Some(true);
                }
            }
            None
        });
        
        if !exists.unwrap_or(false) {
            return Err(PyValueError::new_err(format!("FaceSet '{}' not found", name)));
        }
        
        let path = if self.path.is_empty() {
            format!("/{}", name)
        } else {
            format!("/{}/{}", self.path.join("/"), name)
        };
        
        Ok(PyIFaceSet::new(self.archive.clone(), path))
    }
    
    /// Get geometry parameter by name (for meshes, etc).
    fn getGeomParam(&self, name: &str) -> PyResult<PyIGeomParam> {
        let exists = self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            if IGeomParam::new(&geom, name).is_some() {
                Some(true)
            } else {
                None
            }
        });
        
        if !exists.unwrap_or(false) {
            return Err(PyValueError::new_err(format!("GeomParam '{}' not found", name)));
        }
        
        let obj_path = if self.path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.path.join("/"))
        };
        
        Ok(PyIGeomParam::new(self.archive.clone(), obj_path, name.to_string()))
    }
    
    /// List available geometry parameter names.
    fn getGeomParamNames(&self) -> Vec<String> {
        self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            let mut names = Vec::new();
            for i in 0..geom.getNumProperties() {
                if let Some(prop) = geom.property(i) {
                    let name = prop.getName();
                    // Filter out standard properties
                    if !name.starts_with('.') && !name.is_empty() {
                        names.push(name.to_string());
                    }
                }
            }
            Some(names)
        }).unwrap_or_default()
    }
    
    // ========================================================================
    // Visibility
    // ========================================================================
    
    /// Get visibility at sample index.
    #[pyo3(signature = (index=0))]
    fn getVisibility(&self, index: usize) -> PyObjectVisibility {
        self.with_object(|obj| {
            Some(get_visibility(obj, index).into())
        }).unwrap_or_else(|| PyObjectVisibility::from(crate::geom::visibility::ObjectVisibility::Deferred))
    }
    
    /// Check if object is visible at sample index.
    #[pyo3(signature = (index=0))]
    fn isVisible(&self, index: usize) -> bool {
        self.with_object(|obj| {
            Some(is_visible(obj, index))
        }).unwrap_or(true)
    }
    
    /// Check if object is hidden at sample index.
    #[pyo3(signature = (index=0))]
    fn isHidden(&self, index: usize) -> bool {
        !self.isVisible(index)
    }
    
    // ========================================================================
    // Time Sampling
    // ========================================================================
    
    /// Get the time sampling index for this object's primary data.
    /// Returns 0 (identity/static) if no time sampling found.
    fn getTimeSamplingIndex(&self) -> u32 {
        self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            
            // Try common property names in priority order
            for name in &["P", ".vals", ".xform", ".camera", ".light", ".userProperties"] {
                if let Some(prop) = geom.getPropertyByName(name) {
                    return Some(prop.getHeader().time_sampling_index);
                }
            }
            
            // Fallback: first property
            if geom.getNumProperties() > 0 {
                if let Some(prop) = geom.property(0) {
                    return Some(prop.getHeader().time_sampling_index);
                }
            }
            
            None
        }).unwrap_or(0)
    }
    
    // ========================================================================
    // Bounds
    // ========================================================================
    
    /// Get self bounds at sample index.
    /// Returns ((min_x, min_y, min_z), (max_x, max_y, max_z)) or None.
    #[pyo3(signature = (index=0))]
    fn getSelfBounds(&self, index: usize) -> Option<([f64; 3], [f64; 3])> {
        self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            let bnds_prop = geom.getPropertyByName(".selfBnds")?;
            let scalar = bnds_prop.asScalar()?;
            
            let mut buf = [0u8; 48]; // 6 x f64
            scalar.getSample(index, &mut buf).ok()?;
            let doubles: &[f64] = bytemuck::try_cast_slice(&buf).ok()?;
            
            if doubles.len() >= 6 {
                Some((
                    [doubles[0], doubles[1], doubles[2]],
                    [doubles[3], doubles[4], doubles[5]],
                ))
            } else {
                None
            }
        })
    }
    
    /// Get child bounds at sample index.
    /// Returns ((min_x, min_y, min_z), (max_x, max_y, max_z)) or None.
    #[pyo3(signature = (index=0))]
    fn getChildBounds(&self, index: usize) -> Option<([f64; 3], [f64; 3])> {
        self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            let bnds_prop = geom.getPropertyByName(".childBnds")?;
            let scalar = bnds_prop.asScalar()?;
            
            let mut buf = [0u8; 48]; // 6 x f64
            scalar.getSample(index, &mut buf).ok()?;
            let doubles: &[f64] = bytemuck::try_cast_slice(&buf).ok()?;
            
            if doubles.len() >= 6 {
                Some((
                    [doubles[0], doubles[1], doubles[2]],
                    [doubles[3], doubles[4], doubles[5]],
                ))
            } else {
                None
            }
        })
    }
    
    /// Check if object has self bounds property.
    fn hasSelfBounds(&self) -> bool {
        self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            Some(geom.hasProperty(".selfBnds"))
        }).unwrap_or(false)
    }
    
    /// Check if object has child bounds property.
    fn hasChildBounds(&self) -> bool {
        self.with_object(|obj| {
            let props = obj.getProperties();
            let geom_box = props.getPropertyByName(".geom")?;
            let geom = geom_box.asCompound()?;
            Some(geom.hasProperty(".childBnds"))
        }).unwrap_or(false)
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
    
    // ========================================================================
    // Properties access
    // ========================================================================
    
    /// Get compound property for this object.
    fn getProperties(&self) -> PyICompoundProperty {
        PyICompoundProperty {
            archive: self.archive.clone(),
            object_path: self.path.clone(),
            property_path: Vec::new(),
        }
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
