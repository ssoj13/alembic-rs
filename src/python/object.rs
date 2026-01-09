//! Python bindings for Alembic objects.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

use crate::abc::IArchive;

/// Python wrapper for IObject.
/// 
/// Note: Due to Rust's borrowing rules, we store path as a vector of names
/// and traverse fresh each time we need to access the underlying object.
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
}

/// Macro to traverse path and execute code with the final object.
/// This works around Rust's borrowing limitations by using a macro
/// that generates the nested match statements.
macro_rules! with_object {
    ($self:expr, $obj:ident => $body:expr) => {{
        let root = $self.archive.root();
        match $self.path.len() {
            0 => {
                let $obj = root;
                $body
            }
            1 => {
                if let Some($obj) = root.child_by_name(&$self.path[0]) {
                    $body
                } else {
                    None
                }
            }
            2 => {
                if let Some(c1) = root.child_by_name(&$self.path[0]) {
                    if let Some($obj) = c1.child_by_name(&$self.path[1]) {
                        $body
                    } else { None }
                } else { None }
            }
            3 => {
                if let Some(c1) = root.child_by_name(&$self.path[0]) {
                    if let Some(c2) = c1.child_by_name(&$self.path[1]) {
                        if let Some($obj) = c2.child_by_name(&$self.path[2]) {
                            $body
                        } else { None }
                    } else { None }
                } else { None }
            }
            4 => {
                if let Some(c1) = root.child_by_name(&$self.path[0]) {
                    if let Some(c2) = c1.child_by_name(&$self.path[1]) {
                        if let Some(c3) = c2.child_by_name(&$self.path[2]) {
                            if let Some($obj) = c3.child_by_name(&$self.path[3]) {
                                $body
                            } else { None }
                        } else { None }
                    } else { None }
                } else { None }
            }
            5 => {
                if let Some(c1) = root.child_by_name(&$self.path[0]) {
                    if let Some(c2) = c1.child_by_name(&$self.path[1]) {
                        if let Some(c3) = c2.child_by_name(&$self.path[2]) {
                            if let Some(c4) = c3.child_by_name(&$self.path[3]) {
                                if let Some($obj) = c4.child_by_name(&$self.path[4]) {
                                    $body
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None }
                } else { None }
            }
            _ => {
                // For deeper paths (rare), we'll just return None
                // A proper implementation would need a recursive approach
                // that doesn't have the borrow checker issues
                None
            }
        }
    }};
}

#[pymethods]
impl PyIObject {
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
    
    /// Get number of children.
    fn getNumChildren(&self) -> usize {
        with_object!(self, obj => Some(obj.num_children())).unwrap_or(0)
    }
    
    /// Get child by index.
    fn getChild(&self, index: usize) -> PyResult<PyIObject> {
        let child_name: Option<String> = with_object!(self, obj => {
            obj.child(index).map(|c| c.name().to_string())
        });
        
        let name = child_name
            .ok_or_else(|| PyValueError::new_err("Child index out of range"))?;
        
        let mut new_path = self.path.clone();
        new_path.push(name);
        
        Ok(PyIObject {
            archive: self.archive.clone(),
            path: new_path,
        })
    }
    
    /// Get child by name.
    #[pyo3(signature = (name))]
    fn getChildByName(&self, name: &str) -> PyResult<PyIObject> {
        // Verify child exists
        let exists: bool = with_object!(self, obj => {
            let has_child = obj.child_by_name(name).is_some();
            Some(has_child)
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
        with_object!(self, _obj => Some(true)).unwrap_or(false)
    }
    
    /// Children as list.
    #[getter]
    fn children(&self) -> Vec<PyIObject> {
        let num = self.getNumChildren();
        (0..num)
            .filter_map(|i| self.getChild(i).ok())
            .collect()
    }
    
    /// Get positions if this is a PolyMesh.
    #[pyo3(signature = (index=None))]
    fn getPositions(&self, index: Option<usize>) -> Option<Vec<[f32; 3]>> {
        use crate::geom::IPolyMesh;
        
        with_object!(self, obj => {
            IPolyMesh::new(&obj).and_then(|mesh| {
                mesh.get_sample(index.unwrap_or(0)).ok().map(|s| {
                    s.positions.iter().map(|p| [p.x, p.y, p.z]).collect()
                })
            })
        })
    }
    
    /// Get face counts if this is a PolyMesh.
    #[pyo3(signature = (index=None))]
    fn getFaceCounts(&self, index: Option<usize>) -> Option<Vec<i32>> {
        use crate::geom::IPolyMesh;
        
        with_object!(self, obj => {
            IPolyMesh::new(&obj).and_then(|mesh| {
                mesh.get_sample(index.unwrap_or(0)).ok().map(|s| s.face_counts)
            })
        })
    }
    
    /// Get face indices if this is a PolyMesh.
    #[pyo3(signature = (index=None))]
    fn getFaceIndices(&self, index: Option<usize>) -> Option<Vec<i32>> {
        use crate::geom::IPolyMesh;
        
        with_object!(self, obj => {
            IPolyMesh::new(&obj).and_then(|mesh| {
                mesh.get_sample(index.unwrap_or(0)).ok().map(|s| s.face_indices)
            })
        })
    }
    
    /// Get 4x4 transformation matrix if this is an Xform.
    #[pyo3(signature = (index=None))]
    fn getMatrix(&self, index: Option<usize>) -> Option<[[f64; 4]; 4]> {
        use crate::geom::IXform;
        
        with_object!(self, obj => {
            IXform::new(&obj).and_then(|xform| {
                xform.get_sample(index.unwrap_or(0)).ok().map(|s| {
                    let m = s.matrix();
                    [
                        [m.x_axis.x as f64, m.x_axis.y as f64, m.x_axis.z as f64, m.x_axis.w as f64],
                        [m.y_axis.x as f64, m.y_axis.y as f64, m.y_axis.z as f64, m.y_axis.w as f64],
                        [m.z_axis.x as f64, m.z_axis.y as f64, m.z_axis.z as f64, m.z_axis.w as f64],
                        [m.w_axis.x as f64, m.w_axis.y as f64, m.w_axis.z as f64, m.w_axis.w as f64],
                    ]
                })
            })
        })
    }
    
    /// Get number of samples for this object's schema.
    fn getNumSamples(&self) -> usize {
        use crate::geom::{IPolyMesh, IXform, ICamera, IPoints, ICurves, ISubD};
        
        with_object!(self, obj => {
            if let Some(m) = IPolyMesh::new(&obj) { Some(m.num_samples()) }
            else if let Some(x) = IXform::new(&obj) { Some(x.num_samples()) }
            else if let Some(c) = ICamera::new(&obj) { Some(c.num_samples()) }
            else if let Some(p) = IPoints::new(&obj) { Some(p.num_samples()) }
            else if let Some(c) = ICurves::new(&obj) { Some(c.num_samples()) }
            else if let Some(s) = ISubD::new(&obj) { Some(s.num_samples()) }
            else { Some(0) }
        }).unwrap_or(0)
    }
    
    /// Check if this is a PolyMesh.
    fn isPolyMesh(&self) -> bool {
        use crate::geom::IPolyMesh;
        with_object!(self, obj => Some(IPolyMesh::new(&obj).is_some())).unwrap_or(false)
    }
    
    /// Check if this is an Xform.
    fn isXform(&self) -> bool {
        use crate::geom::IXform;
        with_object!(self, obj => Some(IXform::new(&obj).is_some())).unwrap_or(false)
    }
    
    /// Check if this is a Camera.
    fn isCamera(&self) -> bool {
        use crate::geom::ICamera;
        with_object!(self, obj => Some(ICamera::new(&obj).is_some())).unwrap_or(false)
    }
    
    fn __repr__(&self) -> String {
        format!("<IObject '{}'>", self.getFullName())
    }
    
    fn __len__(&self) -> usize {
        self.getNumChildren()
    }
}
