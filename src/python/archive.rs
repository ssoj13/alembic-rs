//! Python bindings for Alembic archives.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;
use std::sync::Arc;

use crate::abc::IArchive;

/// Python wrapper for IArchive (read-only archive).
#[pyclass(name = "IArchive")]
pub struct PyIArchive {
    pub(crate) inner: Arc<IArchive>,
}

#[pymethods]
impl PyIArchive {
    /// Open an Alembic archive for reading.
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let archive = IArchive::open(path)
            .map_err(|e| PyIOError::new_err(format!("Failed to open archive: {}", e)))?;
        Ok(Self { inner: Arc::new(archive) })
    }
    
    /// Get the archive name.
    fn getName(&self) -> String {
        self.inner.name().to_string()
    }
    
    /// Get the top-level object.
    fn getTop(&self) -> super::object::PyIObject {
        super::object::PyIObject::from_archive(self.inner.clone())
    }
    
    /// Get the number of time samplings.
    fn getNumTimeSamplings(&self) -> usize {
        self.inner.num_time_samplings()
    }
    
    /// Check if the archive is valid.
    fn valid(&self) -> bool {
        true
    }
    
    fn __repr__(&self) -> String {
        format!("<IArchive '{}'>", self.inner.name())
    }
}
