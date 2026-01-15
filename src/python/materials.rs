//! Python bindings for Materials and Collections.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;

use crate::abc::IArchive;
use crate::material::IMaterial;
use crate::collection::{ICollections, Collection};

// ============================================================================
// Collection bindings
// ============================================================================

/// Python wrapper for a collection.
#[pyclass(name = "Collection")]
#[derive(Clone)]
pub struct PyCollection {
    name: String,
    paths: Vec<String>,
}

#[pymethods]
impl PyCollection {
    /// Get collection name.
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }
    
    /// Get all paths in collection.
    #[getter]
    fn paths(&self) -> Vec<String> {
        self.paths.clone()
    }
    
    /// Check if path is in collection.
    fn contains(&self, path: &str) -> bool {
        self.paths.iter().any(|p| p == path)
    }
    
    /// Get number of paths.
    fn __len__(&self) -> usize {
        self.paths.len()
    }
    
    /// Iterate paths.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyCollectionIter>> {
        let paths = slf.paths.clone();
        Py::new(slf.py(), PyCollectionIter { paths, index: 0 })
    }
    
    fn __repr__(&self) -> String {
        format!("<Collection '{}' ({} paths)>", self.name, self.paths.len())
    }
}

impl From<Collection> for PyCollection {
    fn from(c: Collection) -> Self {
        Self {
            name: c.name,
            paths: c.paths,
        }
    }
}

/// Iterator for collection paths.
#[pyclass]
pub struct PyCollectionIter {
    paths: Vec<String>,
    index: usize,
}

#[pymethods]
impl PyCollectionIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }
    
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<String> {
        if slf.index < slf.paths.len() {
            let path = slf.paths[slf.index].clone();
            slf.index += 1;
            Some(path)
        } else {
            None
        }
    }
}

/// Python wrapper for ICollections schema.
#[pyclass(name = "ICollections")]
pub struct PyICollections {
    archive: Arc<IArchive>,
    object_path: Vec<String>,
}

impl PyICollections {
    /// Create from object path.
    pub fn new(archive: Arc<IArchive>, object_path: Vec<String>) -> Self {
        Self { archive, object_path }
    }
    
    /// Execute with collections context using recursive traversal.
    fn with_collections<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&ICollections<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        
        fn traverse_and_execute<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&ICollections<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                let collections = ICollections::new(&obj)?;
                f(&collections)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse_and_execute(child, &path[1..], f)
            }
        }
        
        traverse_and_execute(root, &self.object_path, f)
    }
}

#[pymethods]
impl PyICollections {
    /// Get number of collections.
    fn getNumCollections(&self) -> usize {
        self.with_collections(|c: &ICollections<'_>| Some(c.num_collections())).unwrap_or(0)
    }
    
    /// Get collection names.
    fn getCollectionNames(&self) -> Vec<String> {
        self.with_collections(|c: &ICollections<'_>| Some(c.collection_names())).unwrap_or_default()
    }
    
    /// Get collection by name.
    fn getCollection(&self, name: &str) -> Option<PyCollection> {
        self.with_collections(|c: &ICollections<'_>| {
            c.get(name).map(|col: Collection| col.into())
        })
    }
    
    /// Get collection by index.
    fn getCollectionByIndex(&self, index: usize) -> Option<PyCollection> {
        self.with_collections(|c: &ICollections<'_>| {
            c.collection(index).map(|col: Collection| col.into())
        })
    }
    
    fn __len__(&self) -> usize {
        self.getNumCollections()
    }
    
    fn __repr__(&self) -> String {
        format!("<ICollections ({} collections)>", self.getNumCollections())
    }
}

// ============================================================================
// Material bindings
// ============================================================================

/// Python wrapper for IMaterial schema.
#[pyclass(name = "IMaterial")]
pub struct PyIMaterial {
    archive: Arc<IArchive>,
    object_path: Vec<String>,
}

impl PyIMaterial {
    /// Create from object path.
    pub fn new(archive: Arc<IArchive>, object_path: Vec<String>) -> Self {
        Self { archive, object_path }
    }
    
    /// Execute with material context using recursive traversal.
    fn with_material<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&IMaterial<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        
        fn traverse_and_execute<'a, T>(
            obj: crate::abc::IObject<'a>,
            path: &[String],
            f: impl FnOnce(&IMaterial<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                let material = IMaterial::new(&obj)?;
                f(&material)
            } else {
                let child = obj.getChildByName(&path[0])?;
                traverse_and_execute(child, &path[1..], f)
            }
        }
        
        traverse_and_execute(root, &self.object_path, f)
    }
}

#[pymethods]
impl PyIMaterial {
    /// Get target names (e.g., "arnold", "renderman").
    fn getTargetNames(&self) -> Vec<String> {
        self.with_material(|m: &IMaterial<'_>| {
            Some(m.target_names())
        }).unwrap_or_default()
    }
    
    /// Get shader type names for a target.
    fn getShaderTypeNames(&self, target: &str) -> Vec<String> {
        self.with_material(|m: &IMaterial<'_>| {
            Some(m.shader_type_names(target))
        }).unwrap_or_default()
    }
    
    /// Get shader name for target and type.
    fn getShader(&self, target: &str, shader_type: &str) -> Option<String> {
        self.with_material(|m: &IMaterial<'_>| {
            m.shader(target, shader_type)
        })
    }
    
    /// Check if this material inherits from another.
    fn hasInheritance(&self) -> bool {
        self.with_material(|m: &IMaterial<'_>| Some(m.has_inheritance())).unwrap_or(false)
    }
    
    /// Get inherited material path.
    fn getInheritsPath(&self) -> Option<String> {
        self.with_material(|m: &IMaterial<'_>| m.inherits_path())
    }
    
    /// Get flattened material parameters as dict.
    fn getFlattenedParams(&self) -> HashMap<String, PyObject> {
        Python::with_gil(|py| {
            self.with_material(|m: &IMaterial<'_>| {
                let flat = m.flatten();
                let mut result = HashMap::new();
                
                for (target, network) in &flat.networks {
                    for (node_name, node) in &network.nodes {
                        for param in &node.parameters {
                            let key = format!("{}.{}.{}", target, node_name, param.name);
                            let value = param_to_pyobject(py, &param.value);
                            result.insert(key, value);
                        }
                    }
                }
                
                Some(result)
            }).unwrap_or_default()
        })
    }
    
    /// Check if material is valid.
    fn valid(&self) -> bool {
        self.with_material(|m: &IMaterial<'_>| Some(m.valid())).unwrap_or(false)
    }
    
    fn __repr__(&self) -> String {
        let targets = self.getTargetNames();
        format!("<IMaterial targets={:?}>", targets)
    }
}

/// Convert ShaderParamValue to PyObject.
fn param_to_pyobject(py: Python<'_>, value: &crate::material::ShaderParamValue) -> PyObject {
    use crate::material::ShaderParamValue;
    
    match value {
        ShaderParamValue::Bool(v) => v.into_pyobject(py).unwrap().to_owned().unbind().into_any(),
        ShaderParamValue::Int(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Float(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Double(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::String(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Vec2(v) => vec![v.x, v.y].into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Vec3(v) => vec![v.x, v.y, v.z].into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Vec4(v) => vec![v.x, v.y, v.z, v.w].into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Color3(v) => vec![v.x, v.y, v.z].into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Color4(v) => vec![v.x, v.y, v.z, v.w].into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::Matrix(m) => {
            let arr: [[f32; 4]; 4] = m.to_cols_array_2d();
            arr.into_pyobject(py).unwrap().unbind().into_any()
        }
        ShaderParamValue::FloatArray(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::IntArray(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
        ShaderParamValue::StringArray(v) => v.into_pyobject(py).unwrap().unbind().into_any(),
    }
}
