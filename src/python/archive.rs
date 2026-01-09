//! Python bindings for Alembic archives.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::exceptions::{PyIOError, PyValueError};
use std::sync::Arc;

use crate::abc::IArchive;
use super::time_sampling::PyTimeSampling;

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
    
    /// Get the archive name/path.
    fn getName(&self) -> String {
        self.inner.name().to_string()
    }
    
    /// Get the top-level (root) object.
    fn getTop(&self) -> super::object::PyIObject {
        super::object::PyIObject::from_archive(self.inner.clone())
    }
    
    /// Get the number of time samplings.
    fn getNumTimeSamplings(&self) -> usize {
        self.inner.num_time_samplings()
    }
    
    /// Get a time sampling by index.
    fn getTimeSampling(&self, index: usize) -> PyResult<PyTimeSampling> {
        self.inner.time_sampling(index)
            .map(|ts| ts.into())
            .ok_or_else(|| PyValueError::new_err(format!("Time sampling index {} out of range", index)))
    }
    
    /// Get max number of samples for a time sampling index.
    fn getMaxNumSamplesForTimeSamplingIndex(&self, index: usize) -> Option<usize> {
        self.inner.max_num_samples_for_time_sampling(index)
    }
    
    /// Get archive version (AABBCC format: major.minor.patch).
    fn getArchiveVersion(&self) -> i32 {
        self.inner.archive_version()
    }
    
    /// Get archive version as string (e.g., "1.7.5").
    fn getArchiveVersionString(&self) -> String {
        let v = self.inner.archive_version();
        let major = v / 10000;
        let minor = (v % 10000) / 100;
        let patch = v % 100;
        format!("{}.{}.{}", major, minor, patch)
    }
    
    /// Check if the archive is valid.
    fn valid(&self) -> bool {
        true
    }
    
    /// Check if an object exists at path.
    fn hasObject(&self, path: &str) -> bool {
        self.inner.has_object(path)
    }
    
    fn __repr__(&self) -> String {
        format!("<IArchive '{}' v{}>", self.inner.name(), self.getArchiveVersionString())
    }
}
