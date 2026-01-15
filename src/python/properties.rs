//! Python bindings for property access.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

use crate::abc::IArchive;
use crate::util::{DataType, PlainOldDataType};
use crate::core::ScalarPropertyReader;

/// Property info returned to Python (owns the data).
#[pyclass(name = "PropertyInfo")]
#[derive(Clone)]
pub struct PyPropertyInfo {
    pub name: String,
    pub is_scalar: bool,
    pub is_array: bool,
    pub is_compound: bool,
    pub data_type: String,
    pub extent: u8,
    pub num_samples: usize,
    pub time_sampling_index: u32,
}

#[pymethods]
impl PyPropertyInfo {
    #[getter]
    fn name(&self) -> &str { &self.name }
    #[getter]
    fn isScalar(&self) -> bool { self.is_scalar }
    #[getter]
    fn isArray(&self) -> bool { self.is_array }
    #[getter]
    fn isCompound(&self) -> bool { self.is_compound }
    #[getter]
    fn dataType(&self) -> &str { &self.data_type }
    #[getter]
    fn extent(&self) -> u8 { self.extent }
    #[getter]
    fn numSamples(&self) -> usize { self.num_samples }
    #[getter]
    fn timeSamplingIndex(&self) -> u32 { self.time_sampling_index }
    
    fn __repr__(&self) -> String {
        let kind = if self.is_compound { "compound" }
            else if self.is_scalar { "scalar" }
            else { "array" };
        format!("<PropertyInfo '{}' {} {}>", self.name, kind, self.data_type)
    }
}

/// Python wrapper for compound property access.
/// Stores path for lazy traversal (Rust ownership workaround).
#[pyclass(name = "ICompoundProperty")]
pub struct PyICompoundProperty {
    pub(crate) archive: Arc<IArchive>,
    pub(crate) object_path: Vec<String>,
    pub(crate) property_path: Vec<String>,
}

impl PyICompoundProperty {
    /// Traverse to object, then to compound, execute closure.
    fn with_compound<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&crate::abc::ICompoundProperty<'_>) -> Option<T>,
    {
        let root = self.archive.getTop();
        
        // Recursive object traversal with closure
        fn traverse_obj<'a, T>(
            obj: crate::abc::IObject<'a>,
            obj_path: &[String],
            prop_path: &[String],
            f: impl FnOnce(&crate::abc::ICompoundProperty<'_>) -> Option<T>,
        ) -> Option<T> {
            if obj_path.is_empty() {
                // Reached target object, now traverse properties
                let props = obj.getProperties();
                traverse_prop(props, prop_path, f)
            } else {
                let child = obj.getChildByName(&obj_path[0])?;
                traverse_obj(child, &obj_path[1..], prop_path, f)
            }
        }
        
        // Recursive property traversal with closure
        fn traverse_prop<'a, T>(
            compound: crate::abc::ICompoundProperty<'a>,
            path: &[String],
            f: impl FnOnce(&crate::abc::ICompoundProperty<'_>) -> Option<T>,
        ) -> Option<T> {
            if path.is_empty() {
                f(&compound)
            } else {
                let prop = compound.property_by_name(&path[0])?;
                let child_compound = prop.as_compound()?;
                traverse_prop(child_compound, &path[1..], f)
            }
        }
        
        traverse_obj(root, &self.object_path, &self.property_path, f)
    }
}

#[pymethods]
impl PyICompoundProperty {
    /// Get number of sub-properties.
    fn getNumProperties(&self) -> usize {
        self.with_compound(|c| Some(c.getNumProperties())).unwrap_or(0)
    }
    
    /// Get property names.
    fn getPropertyNames(&self) -> Vec<String> {
        self.with_compound(|c| Some(c.property_names())).unwrap_or_default()
    }
    
    /// Check if property exists.
    fn hasProperty(&self, name: &str) -> bool {
        self.with_compound(|c| Some(c.has_property(name))).unwrap_or(false)
    }
    
    /// Get property info by name.
    fn getPropertyInfo(&self, name: &str) -> Option<PyPropertyInfo> {
        self.with_compound(|c| {
            let prop = c.property_by_name(name)?;
            let hdr = prop.getHeader();
            Some(PyPropertyInfo {
                name: hdr.name.clone(),
                is_scalar: prop.is_scalar(),
                is_array: prop.is_array(),
                is_compound: prop.is_compound(),
                data_type: format!("{:?}", hdr.data_type),
                extent: hdr.data_type.extent,
                num_samples: if prop.is_scalar() {
                    prop.as_scalar().map(|s| s.getNumSamples()).unwrap_or(1)
                } else if prop.is_array() {
                    prop.as_array().map(|a| a.getNumSamples()).unwrap_or(1)
                } else { 1 },
                time_sampling_index: hdr.time_sampling_index,
            })
        })
    }
    
    /// Get sub-compound property by name.
    fn getCompoundProperty(&self, name: &str) -> PyResult<PyICompoundProperty> {
        // Check existence via closure - returns owned bool
        let exists = self.with_compound(|c| {
            let prop = c.property_by_name(name)?;
            if prop.is_compound() { Some(true) } else { None }
        }).unwrap_or(false);
        
        if !exists {
            return Err(PyValueError::new_err(format!("No compound property '{}'", name)));
        }
        
        let mut new_path = self.property_path.clone();
        new_path.push(name.to_string());
        
        Ok(PyICompoundProperty {
            archive: self.archive.clone(),
            object_path: self.object_path.clone(),
            property_path: new_path,
        })
    }
    
    /// Read scalar property value at sample index.
    /// Returns Python types: int, float, str, list, etc.
    #[pyo3(signature = (name, index=0))]
    fn getScalarValue(&self, name: &str, index: usize) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            self.with_compound(|c| {
                let prop = c.property_by_name(name)?;
                let scalar = prop.as_scalar()?;
                let hdr = scalar.getHeader();
                
                read_scalar_as_python(py, scalar, index, hdr.data_type)
            })
            .ok_or_else(|| PyValueError::new_err(format!("Failed to read scalar '{}'", name)))
        })
    }
    
    /// Read array property values at sample index.
    /// Returns list of Python values.
    #[pyo3(signature = (name, index=0))]
    fn getArrayValue(&self, name: &str, index: usize) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            self.with_compound(|c| {
                let prop = c.property_by_name(name)?;
                let array = prop.as_array()?;
                let hdr = array.getHeader();
                let data = array.read_sample_vec(index).ok()?;
                
                read_array_as_python(py, &data, hdr.data_type)
            })
            .ok_or_else(|| PyValueError::new_err(format!("Failed to read array '{}'", name)))
        })
    }
    
    /// Iterate property names.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyPropertyNameIter>> {
        let names = slf.getPropertyNames();
        Py::new(slf.py(), PyPropertyNameIter { names, index: 0 })
    }
    
    fn __len__(&self) -> usize {
        self.getNumProperties()
    }
    
    fn __repr__(&self) -> String {
        let path = if self.property_path.is_empty() {
            ".props".to_string()
        } else {
            format!(".props/{}", self.property_path.join("/"))
        };
        format!("<ICompoundProperty '{}'>", path)
    }
}

/// Iterator for property names.
#[pyclass]
pub struct PyPropertyNameIter {
    names: Vec<String>,
    index: usize,
}

#[pymethods]
impl PyPropertyNameIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }
    
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<String> {
        if slf.index < slf.names.len() {
            let name = slf.names[slf.index].clone();
            slf.index += 1;
            Some(name)
        } else {
            None
        }
    }
}

// ============================================================================
// Helper functions for reading data as Python types
// ============================================================================

fn read_scalar_as_python(
    py: Python<'_>,
    scalar: &dyn ScalarPropertyReader,
    index: usize,
    data_type: DataType,
) -> Option<PyObject> {
    use PlainOldDataType::*;
    
    let num_samples = scalar.getNumSamples();
    let idx = index.min(num_samples.saturating_sub(1));
    let extent = data_type.extent;
    
    // Read based on POD type
    match data_type.pod {
        Boolean => {
            let mut buf = [0u8; 1];
            scalar.read_sample(idx, &mut buf).ok()?;
            let val = buf[0] != 0;
            Some(val.into_pyobject(py).ok()?.to_owned().unbind().into_any())
        }
        Int8 => {
            let mut buf = [0u8; 1];
            scalar.read_sample(idx, &mut buf).ok()?;
            let val = buf[0] as i8;
            Some(val.into_pyobject(py).ok()?.unbind().into_any())
        }
        Uint8 => {
            let mut buf = [0u8; 1];
            scalar.read_sample(idx, &mut buf).ok()?;
            Some(buf[0].into_pyobject(py).ok()?.unbind().into_any())
        }
        Int16 => {
            let mut buf = [0u8; 2];
            scalar.read_sample(idx, &mut buf).ok()?;
            let val = i16::from_le_bytes(buf);
            Some(val.into_pyobject(py).ok()?.unbind().into_any())
        }
        Uint16 => {
            let mut buf = [0u8; 2];
            scalar.read_sample(idx, &mut buf).ok()?;
            let val = u16::from_le_bytes(buf);
            Some(val.into_pyobject(py).ok()?.unbind().into_any())
        }
        Int32 => {
            if extent > 1 {
                let size = 4 * extent as usize;
                let mut buf = vec![0u8; size];
                scalar.read_sample(idx, &mut buf).ok()?;
                let values: Vec<i32> = buf.chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                let mut buf = [0u8; 4];
                scalar.read_sample(idx, &mut buf).ok()?;
                let val = i32::from_le_bytes(buf);
                Some(val.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Uint32 => {
            if extent > 1 {
                let size = 4 * extent as usize;
                let mut buf = vec![0u8; size];
                scalar.read_sample(idx, &mut buf).ok()?;
                let values: Vec<u32> = buf.chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                let mut buf = [0u8; 4];
                scalar.read_sample(idx, &mut buf).ok()?;
                let val = u32::from_le_bytes(buf);
                Some(val.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Int64 => {
            if extent > 1 {
                let size = 8 * extent as usize;
                let mut buf = vec![0u8; size];
                scalar.read_sample(idx, &mut buf).ok()?;
                let values: Vec<i64> = buf.chunks_exact(8)
                    .map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect();
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                let mut buf = [0u8; 8];
                scalar.read_sample(idx, &mut buf).ok()?;
                let val = i64::from_le_bytes(buf);
                Some(val.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Uint64 => {
            if extent > 1 {
                let size = 8 * extent as usize;
                let mut buf = vec![0u8; size];
                scalar.read_sample(idx, &mut buf).ok()?;
                let values: Vec<u64> = buf.chunks_exact(8)
                    .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect();
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                let mut buf = [0u8; 8];
                scalar.read_sample(idx, &mut buf).ok()?;
                let val = u64::from_le_bytes(buf);
                Some(val.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Float16 => {
            let mut buf = [0u8; 2];
            scalar.read_sample(idx, &mut buf).ok()?;
            let f16 = half::f16::from_le_bytes(buf);
            let val = f16.to_f32();
            Some(val.into_pyobject(py).ok()?.unbind().into_any())
        }
        Float32 => {
            if extent > 1 {
                let size = 4 * extent as usize;
                let mut buf = vec![0u8; size];
                scalar.read_sample(idx, &mut buf).ok()?;
                let floats: Vec<f32> = buf.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                Some(floats.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                let mut buf = [0u8; 4];
                scalar.read_sample(idx, &mut buf).ok()?;
                let val = f32::from_le_bytes(buf);
                Some(val.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Float64 => {
            if extent > 1 {
                let size = 8 * extent as usize;
                let mut buf = vec![0u8; size];
                scalar.read_sample(idx, &mut buf).ok()?;
                let floats: Vec<f64> = buf.chunks_exact(8)
                    .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect();
                Some(floats.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                let mut buf = [0u8; 8];
                scalar.read_sample(idx, &mut buf).ok()?;
                let val = f64::from_le_bytes(buf);
                Some(val.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        String | Wstring => {
            // Use read_sample_vec for variable-length string data
            if let Ok(buf) = scalar.read_sample_vec(idx) {
                let s = std::string::String::from_utf8_lossy(&buf);
                Some(s.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some("".into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Unknown => None,
    }
}

fn read_array_as_python(
    py: Python<'_>,
    data: &[u8],
    data_type: DataType,
) -> Option<PyObject> {
    use PlainOldDataType::*;
    
    let extent = data_type.extent;
    
    match data_type.pod {
        Boolean => {
            let values: Vec<bool> = data.iter().map(|&b| b != 0).collect();
            Some(values.into_pyobject(py).ok()?.unbind().into_any())
        }
        Int8 => {
            let values: Vec<i8> = data.iter().map(|&b| b as i8).collect();
            Some(values.into_pyobject(py).ok()?.unbind().into_any())
        }
        Uint8 => {
            Some(data.to_vec().into_pyobject(py).ok()?.unbind().into_any())
        }
        Int16 => {
            let values: Vec<i16> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            Some(values.into_pyobject(py).ok()?.unbind().into_any())
        }
        Uint16 => {
            let values: Vec<u16> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            Some(values.into_pyobject(py).ok()?.unbind().into_any())
        }
        Int32 => {
            let values: Vec<i32> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            if extent > 1 {
                let grouped: Vec<Vec<i32>> = values.chunks(extent as usize)
                    .map(|c| c.to_vec())
                    .collect();
                Some(grouped.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Uint32 => {
            let values: Vec<u32> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            if extent > 1 {
                let grouped: Vec<Vec<u32>> = values.chunks(extent as usize)
                    .map(|c| c.to_vec())
                    .collect();
                Some(grouped.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Int64 => {
            let values: Vec<i64> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            if extent > 1 {
                let grouped: Vec<Vec<i64>> = values.chunks(extent as usize)
                    .map(|c| c.to_vec())
                    .collect();
                Some(grouped.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Uint64 => {
            let values: Vec<u64> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            if extent > 1 {
                let grouped: Vec<Vec<u64>> = values.chunks(extent as usize)
                    .map(|c| c.to_vec())
                    .collect();
                Some(grouped.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Float32 => {
            let values: Vec<f32> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            if extent > 1 {
                let grouped: Vec<Vec<f32>> = values.chunks(extent as usize)
                    .map(|c| c.to_vec())
                    .collect();
                Some(grouped.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Float64 => {
            let values: Vec<f64> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            if extent > 1 {
                let grouped: Vec<Vec<f64>> = values.chunks(extent as usize)
                    .map(|c| c.to_vec())
                    .collect();
                Some(grouped.into_pyobject(py).ok()?.unbind().into_any())
            } else {
                Some(values.into_pyobject(py).ok()?.unbind().into_any())
            }
        }
        Float16 => {
            // Read as raw u16 and convert to f32
            let raw: Vec<u16> = bytemuck::try_cast_slice(data).ok()?.to_vec();
            let values: Vec<f32> = raw.iter()
                .map(|&v| half::f16::from_bits(v).to_f32())
                .collect();
            Some(values.into_pyobject(py).ok()?.unbind().into_any())
        }
        String | Wstring => {
            // String arrays are more complex, return as single string for now
            let s = std::string::String::from_utf8_lossy(data);
            Some(s.into_pyobject(py).ok()?.unbind().into_any())
        }
        Unknown => None,
    }
}
