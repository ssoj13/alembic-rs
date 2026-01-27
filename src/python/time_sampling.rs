//! Python bindings for TimeSampling.

#![allow(non_snake_case)]

use pyo3::prelude::*;
use crate::core::{TimeSampling, TimeSamplingType};

/// Python wrapper for TimeSampling.
#[pyclass(name = "TimeSampling")]
#[derive(Clone)]
pub struct PyTimeSampling {
    pub(crate) inner: TimeSampling,
}

#[pymethods]
impl PyTimeSampling {
    /// Create identity (static) time sampling.
    #[staticmethod]
    fn identity() -> Self {
        Self { inner: TimeSampling::identity() }
    }
    
    /// Create uniform time sampling.
    /// 
    /// Args:
    ///     time_per_cycle: Time between samples (e.g., 1/24 for 24fps)
    ///     start_time: Time of first sample
    #[staticmethod]
    fn uniform(time_per_cycle: f64, start_time: f64) -> Self {
        Self { inner: TimeSampling::uniform(time_per_cycle, start_time) }
    }
    
    /// Create acyclic time sampling with explicit times.
    #[staticmethod]
    fn acyclic(times: Vec<f64>) -> Self {
        Self { inner: TimeSampling::acyclic(times) }
    }
    
    /// Create cyclic time sampling.
    #[staticmethod]
    fn cyclic(time_per_cycle: f64, times: Vec<f64>) -> Self {
        Self { inner: TimeSampling::cyclic(time_per_cycle, times) }
    }
    
    /// Check if this is identity (static) sampling.
    fn isIdentity(&self) -> bool {
        self.inner.is_identity()
    }
    
    /// Check if this is uniform sampling.
    fn isUniform(&self) -> bool {
        self.inner.is_uniform()
    }
    
    /// Check if this is cyclic sampling.
    fn isCyclic(&self) -> bool {
        self.inner.is_cyclic()
    }
    
    /// Check if this is acyclic sampling.
    fn isAcyclic(&self) -> bool {
        self.inner.is_acyclic()
    }
    
    /// Get the number of samples per cycle.
    fn getSamplesPerCycle(&self) -> usize {
        self.inner.samples_per_cycle()
    }
    
    /// Get time per cycle.
    fn getTimePerCycle(&self) -> f64 {
        self.inner.time_per_cycle()
    }
    
    /// Get number of stored times.
    fn getNumStoredTimes(&self) -> usize {
        self.inner.num_stored_times()
    }
    
    /// Get stored times.
    fn getStoredTimes(&self) -> Vec<f64> {
        self.inner.stored_times()
    }
    
    /// Get the time for a specific sample index.
    fn getSampleTime(&self, index: usize, num_samples: usize) -> f64 {
        self.inner.sample_time(index, num_samples)
    }
    
    /// Get floor index for a time.
    fn getFloorIndex(&self, time: f64, num_samples: usize) -> (usize, f64) {
        self.inner.floor_index(time, num_samples)
    }
    
    /// Get ceiling index for a time.
    fn getCeilIndex(&self, time: f64, num_samples: usize) -> (usize, f64) {
        self.inner.ceil_index(time, num_samples)
    }
    
    /// Get nearest index for a time.
    fn getNearIndex(&self, time: f64, num_samples: usize) -> (usize, f64) {
        self.inner.near_index(time, num_samples)
    }
    
    /// Get bracketing time samples for interpolation.
    /// Returns (floor_index, ceil_index, interpolation_factor).
    fn getBracketingTimeSamples(&self, time: f64, num_samples: usize) -> (usize, usize, f64) {
        self.inner.get_bracketing_time_samples(time, num_samples)
    }
    
    /// Check if interpolation is needed at the given time.
    fn needsInterpolation(&self, time: f64, num_samples: usize) -> bool {
        self.inner.needs_interpolation(time, num_samples)
    }
    
    /// Get time range (min, max).
    fn getTimeRange(&self, num_samples: usize) -> (f64, f64) {
        self.inner.time_range(num_samples)
    }
    
    fn __repr__(&self) -> String {
        let type_str = match &self.inner.sampling_type {
            TimeSamplingType::Identity => "Identity".to_string(),
            TimeSamplingType::Uniform { time_per_cycle, .. } => {
                format!("Uniform({:.2}fps)", 1.0 / time_per_cycle)
            }
            TimeSamplingType::Cyclic { time_per_cycle, times } => {
                format!("Cyclic({} per {:.2}s)", times.len(), time_per_cycle)
            }
            TimeSamplingType::Acyclic { times } => {
                format!("Acyclic({} times)", times.len())
            }
        };
        format!("<TimeSampling {}>", type_str)
    }
}

impl From<TimeSampling> for PyTimeSampling {
    fn from(ts: TimeSampling) -> Self {
        Self { inner: ts }
    }
}

impl From<&TimeSampling> for PyTimeSampling {
    fn from(ts: &TimeSampling) -> Self {
        Self { inner: ts.clone() }
    }
}

// ============================================================================
// ISampleSelector — Python wrapper for SampleSelector
// ============================================================================

use crate::core::SampleSelector;

/// Python wrapper for ISampleSelector.
///
/// Provides time-based or index-based sample selection for reading property
/// values at specific times. Matches the original Alembic ISampleSelector API.
///
/// # Example
/// ```python
/// sel = ISampleSelector(5)          # by index
/// sel = ISampleSelector(1.5)        # nearest time
/// sel = ISampleSelector.floor(1.5)  # floor time
/// sel = ISampleSelector.ceil(1.5)   # ceil time
/// ```
#[pyclass(name = "ISampleSelector")]
#[derive(Clone)]
pub struct PyISampleSelector {
    pub(crate) inner: SampleSelector,
}

#[pymethods]
impl PyISampleSelector {
    /// Create selector from index (int) or time (float).
    ///
    /// If given an int, selects by sample index.
    /// If given a float, selects by nearest time.
    #[new]
    #[pyo3(signature = (value=None))]
    fn new(value: Option<&Bound<'_, pyo3::types::PyAny>>) -> PyResult<Self> {
        match value {
            None => Ok(Self { inner: SampleSelector::first() }),
            Some(v) => {
                if let Ok(idx) = v.extract::<usize>() {
                    Ok(Self { inner: SampleSelector::index(idx) })
                } else if let Ok(t) = v.extract::<f64>() {
                    Ok(Self { inner: SampleSelector::time_near(t) })
                } else {
                    Err(pyo3::exceptions::PyTypeError::new_err(
                        "ISampleSelector expects int (index) or float (time)",
                    ))
                }
            }
        }
    }

    /// Select by floor time (largest index <= time).
    #[staticmethod]
    fn floor(time: f64) -> Self {
        Self { inner: SampleSelector::time_floor(time) }
    }

    /// Select by ceil time (smallest index >= time).
    #[staticmethod]
    fn ceil(time: f64) -> Self {
        Self { inner: SampleSelector::time_ceil(time) }
    }

    /// Select by nearest time.
    #[staticmethod]
    fn near(time: f64) -> Self {
        Self { inner: SampleSelector::time_near(time) }
    }

    /// Select by exact index.
    #[staticmethod]
    fn index(idx: usize) -> Self {
        Self { inner: SampleSelector::index(idx) }
    }

    /// Resolve to actual sample index given time sampling and num samples.
    fn getIndex(&self, ts: &PyTimeSampling, num_samples: usize) -> usize {
        self.inner.resolve(&ts.inner, num_samples)
    }

    /// Get the requested index (for Index selectors).
    fn getRequestedIndex(&self) -> usize {
        match self.inner {
            SampleSelector::Index(i) => i,
            _ => 0,
        }
    }

    /// Get the requested time (for time-based selectors).
    fn getRequestedTime(&self) -> f64 {
        match self.inner {
            SampleSelector::TimeFloor(t)
            | SampleSelector::TimeCeil(t)
            | SampleSelector::TimeNear(t) => t,
            SampleSelector::Index(_) => 0.0,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            SampleSelector::Index(i) => format!("<ISampleSelector index={}>", i),
            SampleSelector::TimeFloor(t) => format!("<ISampleSelector floor={:.4}>", t),
            SampleSelector::TimeCeil(t) => format!("<ISampleSelector ceil={:.4}>", t),
            SampleSelector::TimeNear(t) => format!("<ISampleSelector near={:.4}>", t),
        }
    }
}
