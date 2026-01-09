//! Time sampling types for Alembic.
//!
//! Alembic properties are sampled over time. The TimeSampling struct
//! describes when each sample was recorded.

use crate::util::Chrono;

/// Type of time sampling.
#[derive(Clone, Debug, PartialEq)]
pub enum TimeSamplingType {
    /// Single static sample at time 0 (identity sampling).
    Identity,

    /// Uniform sampling: samples at regular intervals.
    /// start_time + index * time_per_cycle
    Uniform {
        time_per_cycle: Chrono,
        start_time: Chrono,
    },

    /// Cyclic sampling: repeating pattern of sample times.
    Cyclic {
        time_per_cycle: Chrono,
        times: Vec<Chrono>,
    },

    /// Acyclic sampling: explicit time for each sample.
    Acyclic {
        times: Vec<Chrono>,
    },
}

impl TimeSamplingType {
    /// Check if this is identity (static) sampling.
    #[inline]
    pub fn is_identity(&self) -> bool {
        matches!(self, Self::Identity)
    }

    /// Check if this is uniform sampling.
    #[inline]
    pub fn is_uniform(&self) -> bool {
        matches!(self, Self::Uniform { .. })
    }

    /// Check if this is cyclic sampling.
    #[inline]
    pub fn is_cyclic(&self) -> bool {
        matches!(self, Self::Cyclic { .. })
    }

    /// Check if this is acyclic sampling.
    #[inline]
    pub fn is_acyclic(&self) -> bool {
        matches!(self, Self::Acyclic { .. })
    }

    /// Get the number of samples per cycle (1 for uniform/identity).
    pub fn samples_per_cycle(&self) -> usize {
        match self {
            Self::Identity => 1,
            Self::Uniform { .. } => 1,
            Self::Cyclic { times, .. } => times.len(),
            Self::Acyclic { times } => times.len(),
        }
    }
}

impl Default for TimeSamplingType {
    fn default() -> Self {
        Self::Identity
    }
}

/// Time sampling information for a property.
#[derive(Clone, Debug)]
pub struct TimeSampling {
    /// The type of sampling.
    pub sampling_type: TimeSamplingType,
}

impl TimeSampling {
    /// Identity time sampling (single sample at time 0).
    pub const IDENTITY: Self = Self {
        sampling_type: TimeSamplingType::Identity,
    };

    /// Create uniform time sampling.
    pub fn uniform(time_per_cycle: Chrono, start_time: Chrono) -> Self {
        Self {
            sampling_type: TimeSamplingType::Uniform {
                time_per_cycle,
                start_time,
            },
        }
    }

    /// Create acyclic time sampling from explicit times.
    pub fn acyclic(times: Vec<Chrono>) -> Self {
        Self {
            sampling_type: TimeSamplingType::Acyclic { times },
        }
    }
    
    /// Create cyclic time sampling.
    pub fn cyclic(time_per_cycle: Chrono, times: Vec<Chrono>) -> Self {
        Self {
            sampling_type: TimeSamplingType::Cyclic {
                time_per_cycle,
                times,
            },
        }
    }
    
    /// Create from type and times (general constructor).
    pub fn from_type_and_times(tst: TimeSamplingType, _times: Vec<Chrono>) -> Self {
        Self {
            sampling_type: tst,
        }
    }

    /// Get the time for a specific sample index.
    pub fn sample_time(&self, index: usize, _num_samples: usize) -> Chrono {
        match &self.sampling_type {
            TimeSamplingType::Identity => 0.0,
            TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                *start_time + (index as Chrono) * *time_per_cycle
            }
            TimeSamplingType::Cyclic { time_per_cycle, times } => {
                if times.is_empty() {
                    return 0.0;
                }
                let cycle = index / times.len();
                let local_idx = index % times.len();
                times[local_idx] + (cycle as Chrono) * *time_per_cycle
            }
            TimeSamplingType::Acyclic { times } => {
                times.get(index).copied().unwrap_or(0.0)
            }
        }
    }

    /// Find the floor index (largest index with time <= given time).
    pub fn floor_index(&self, time: Chrono, num_samples: usize) -> (usize, Chrono) {
        if num_samples == 0 {
            return (0, 0.0);
        }

        match &self.sampling_type {
            TimeSamplingType::Identity => (0, 0.0),
            TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                if time <= *start_time {
                    return (0, *start_time);
                }
                let idx = ((time - start_time) / time_per_cycle).floor() as usize;
                let idx = idx.min(num_samples - 1);
                (idx, self.sample_time(idx, num_samples))
            }
            TimeSamplingType::Cyclic { .. } | TimeSamplingType::Acyclic { .. } => {
                // Binary search for floor
                let mut lo = 0;
                let mut hi = num_samples;
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if self.sample_time(mid, num_samples) <= time {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                let idx = if lo > 0 { lo - 1 } else { 0 };
                (idx, self.sample_time(idx, num_samples))
            }
        }
    }

    /// Find the ceiling index (smallest index with time >= given time).
    pub fn ceil_index(&self, time: Chrono, num_samples: usize) -> (usize, Chrono) {
        if num_samples == 0 {
            return (0, 0.0);
        }

        let (floor_idx, floor_time) = self.floor_index(time, num_samples);
        if floor_time >= time {
            return (floor_idx, floor_time);
        }

        let ceil_idx = (floor_idx + 1).min(num_samples - 1);
        (ceil_idx, self.sample_time(ceil_idx, num_samples))
    }

    /// Find the nearest index to the given time.
    pub fn near_index(&self, time: Chrono, num_samples: usize) -> (usize, Chrono) {
        if num_samples == 0 {
            return (0, 0.0);
        }

        let (floor_idx, floor_time) = self.floor_index(time, num_samples);
        if floor_idx >= num_samples - 1 {
            return (floor_idx, floor_time);
        }

        let ceil_idx = floor_idx + 1;
        let ceil_time = self.sample_time(ceil_idx, num_samples);

        if (time - floor_time).abs() <= (ceil_time - time).abs() {
            (floor_idx, floor_time)
        } else {
            (ceil_idx, ceil_time)
        }
    }
}

impl Default for TimeSampling {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_sampling() {
        let ts = TimeSampling::uniform(1.0 / 24.0, 0.0); // 24 fps

        assert_eq!(ts.sample_time(0, 100), 0.0);
        assert!((ts.sample_time(24, 100) - 1.0).abs() < 1e-10);
        assert!((ts.sample_time(48, 100) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_acyclic_sampling() {
        let ts = TimeSampling::acyclic(vec![0.0, 0.5, 1.0, 2.0]);

        assert_eq!(ts.sample_time(0, 4), 0.0);
        assert_eq!(ts.sample_time(1, 4), 0.5);
        assert_eq!(ts.sample_time(2, 4), 1.0);
        assert_eq!(ts.sample_time(3, 4), 2.0);
    }

    #[test]
    fn test_floor_index() {
        let ts = TimeSampling::uniform(1.0, 0.0);

        assert_eq!(ts.floor_index(0.5, 10).0, 0);
        assert_eq!(ts.floor_index(1.5, 10).0, 1);
        assert_eq!(ts.floor_index(5.0, 10).0, 5);
    }
}
