//! Sample types for Alembic properties.
//!
//! Samples represent a single time slice of data for a property.

use crate::util::Chrono;
use crate::core::TimeSampling;

/// Sample selector for reading property samples.
#[derive(Clone, Copy, Debug)]
pub enum SampleSelector {
    /// Select by exact index.
    Index(usize),
    /// Select by time - floor (largest index <= time).
    TimeFloor(Chrono),
    /// Select by time - ceil (smallest index >= time).
    TimeCeil(Chrono),
    /// Select by time - nearest.
    TimeNear(Chrono),
}

impl SampleSelector {
    /// Create a selector for index 0 (first/static sample).
    pub const fn first() -> Self {
        Self::Index(0)
    }

    /// Create a selector for a specific index.
    pub const fn index(i: usize) -> Self {
        Self::Index(i)
    }

    /// Create a selector for floor time.
    pub const fn time_floor(t: Chrono) -> Self {
        Self::TimeFloor(t)
    }

    /// Create a selector for ceil time.
    pub const fn time_ceil(t: Chrono) -> Self {
        Self::TimeCeil(t)
    }

    /// Create a selector for nearest time.
    pub const fn time_near(t: Chrono) -> Self {
        Self::TimeNear(t)
    }
    
    /// Resolve the actual sample index given a time sampling and sample count.
    /// 
    /// For index-based selectors, returns the index clamped to valid range.
    /// For time-based selectors, uses the time sampling to find the appropriate index.
    pub fn get_index(&self, ts: &TimeSampling, num_samples: usize) -> usize {
        if num_samples == 0 {
            return 0;
        }
        
        let idx = match self {
            Self::Index(i) => *i,
            Self::TimeFloor(t) => ts.floor_index(*t, num_samples).0,
            Self::TimeCeil(t) => ts.ceil_index(*t, num_samples).0,
            Self::TimeNear(t) => ts.near_index(*t, num_samples).0,
        };
        
        // Clamp to valid range
        idx.min(num_samples.saturating_sub(1))
    }
    
    /// Check if this selector requests a specific index (not time-based).
    pub fn is_index(&self) -> bool {
        matches!(self, Self::Index(_))
    }
    
    /// Check if this selector is time-based.
    pub fn is_time_based(&self) -> bool {
        !self.is_index()
    }
    
    /// Get the requested index if this is an index-based selector.
    pub fn requested_index(&self) -> Option<usize> {
        match self {
            Self::Index(i) => Some(*i),
            _ => None,
        }
    }
    
    /// Get the requested time if this is a time-based selector.
    pub fn requested_time(&self) -> Option<Chrono> {
        match self {
            Self::TimeFloor(t) | Self::TimeCeil(t) | Self::TimeNear(t) => Some(*t),
            Self::Index(_) => None,
        }
    }
    
    /// Get interpolation information for time-based sampling.
    /// 
    /// Returns a SampleInterp containing floor/ceil indices and interpolation factor.
    /// For index-based selectors, returns exact interpolation at that index.
    pub fn get_sample_interp(&self, ts: &TimeSampling, num_samples: usize) -> SampleInterp {
        if num_samples == 0 {
            return SampleInterp::exact(0);
        }
        
        match self {
            Self::Index(i) => {
                let idx = (*i).min(num_samples.saturating_sub(1));
                SampleInterp::exact(idx)
            }
            Self::TimeFloor(t) | Self::TimeCeil(t) | Self::TimeNear(t) => {
                let time = *t;
                
                // Get floor and ceil
                let (floor_idx, floor_time) = ts.floor_index(time, num_samples);
                let (ceil_idx, ceil_time) = ts.ceil_index(time, num_samples);
                
                if floor_idx == ceil_idx || (ceil_time - floor_time).abs() < 1e-12 {
                    // Exact sample or zero-length interval
                    SampleInterp::exact(floor_idx)
                } else {
                    // Compute interpolation factor
                    let alpha = (time - floor_time) / (ceil_time - floor_time);
                    SampleInterp::lerp(floor_idx, ceil_idx, alpha)
                }
            }
        }
    }
}

impl Default for SampleSelector {
    fn default() -> Self {
        Self::Index(0)
    }
}

impl From<usize> for SampleSelector {
    fn from(index: usize) -> Self {
        Self::Index(index)
    }
}

impl From<Chrono> for SampleSelector {
    fn from(time: Chrono) -> Self {
        Self::TimeNear(time)
    }
}

/// Result of sample interpolation query.
#[derive(Clone, Copy, Debug)]
pub struct SampleInterp {
    /// Floor sample index.
    pub floor_index: usize,
    /// Ceil sample index.
    pub ceil_index: usize,
    /// Interpolation factor (0.0 = floor, 1.0 = ceil).
    pub alpha: f64,
}

impl SampleInterp {
    /// Create for exact sample (no interpolation needed).
    pub fn exact(index: usize) -> Self {
        Self {
            floor_index: index,
            ceil_index: index,
            alpha: 0.0,
        }
    }

    /// Create for interpolation between two samples.
    pub fn lerp(floor: usize, ceil: usize, alpha: f64) -> Self {
        Self {
            floor_index: floor,
            ceil_index: ceil,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Check if this is an exact sample (no interpolation).
    /// Uses tolerance for float comparison to avoid precision issues.
    pub fn is_exact(&self) -> bool {
        self.floor_index == self.ceil_index || self.alpha.abs() < 1e-9
    }
}

/// Scope/extent of data in a geom schema sample.
/// 
/// This corresponds to Renderman's "Primitive Variable Class":
/// - Constant: One value for the entire primitive
/// - Uniform: One value per face/patch
/// - Varying: One value per parametric corner (interpolated)
/// - Vertex: One value per control vertex
/// - FaceVarying: One value per face-vertex (allows discontinuities)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GeometryScope {
    /// Constant for entire object (1 value).
    #[default]
    Constant = 0,
    /// Per-face/patch uniform (1 value per face).
    Uniform = 1,
    /// Per-vertex varying (interpolated at corners).
    Varying = 2,
    /// Per control vertex.
    Vertex = 3,
    /// Per-face-vertex (allows discontinuities at edges).
    FaceVarying = 4,
    /// Unknown scope.
    Unknown = 127,
}

impl GeometryScope {
    /// Parse from string (as stored in metadata).
    pub fn parse(s: &str) -> Self {
        match s {
            "con" | "constant" => Self::Constant,
            "uni" | "uniform" => Self::Uniform,
            "var" | "varying" => Self::Varying,
            "vtx" | "vertex" => Self::Vertex,
            "fvr" | "facevarying" => Self::FaceVarying,
            "unk" | "unknown" => Self::Unknown,
            _ => Self::Unknown, // Unknown for unrecognized values
        }
    }

    /// Convert to short string for metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Constant => "con",
            Self::Uniform => "uni",
            Self::Varying => "var",
            Self::Vertex => "vtx",
            Self::FaceVarying => "fvr",
            Self::Unknown => "unk",
        }
    }
}

/// Topology variance hint for geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TopologyVariance {
    /// Topology changes every sample.
    #[default]
    Heterogeneous,
    /// Topology is constant, only positions change.
    Homogeneous,
    /// Completely static (single sample).
    Static,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_selector() {
        let sel = SampleSelector::index(5);
        assert!(matches!(sel, SampleSelector::Index(5)));

        let sel: SampleSelector = 3.into();
        assert!(matches!(sel, SampleSelector::Index(3)));

        let sel: SampleSelector = 1.5.into();
        assert!(matches!(sel, SampleSelector::TimeNear(t) if (t - 1.5).abs() < 1e-10));
    }

    #[test]
    fn test_sample_interp() {
        let exact = SampleInterp::exact(5);
        assert!(exact.is_exact());
        assert_eq!(exact.floor_index, 5);

        let lerp = SampleInterp::lerp(2, 3, 0.5);
        assert!(!lerp.is_exact());
        assert_eq!(lerp.floor_index, 2);
        assert_eq!(lerp.ceil_index, 3);
        assert!((lerp.alpha - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_geometry_scope() {
        assert_eq!(GeometryScope::parse("fvr"), GeometryScope::FaceVarying);
        assert_eq!(GeometryScope::Vertex.as_str(), "vtx");
    }
    
    #[test]
    fn test_sample_selector_get_index() {
        // Uniform sampling at 24 fps
        let ts = TimeSampling::uniform(1.0 / 24.0, 0.0);
        let num_samples = 100;
        
        // Index-based
        let sel = SampleSelector::index(5);
        assert_eq!(sel.get_index(&ts, num_samples), 5);
        
        // Index clamped to range
        let sel = SampleSelector::index(200);
        assert_eq!(sel.get_index(&ts, num_samples), 99);
        
        // Time-based near: 1.0 second = frame 24
        let sel = SampleSelector::time_near(1.0);
        assert_eq!(sel.get_index(&ts, num_samples), 24);
        
        // Time-based floor
        let sel = SampleSelector::time_floor(1.02); // Slightly after frame 24
        assert_eq!(sel.get_index(&ts, num_samples), 24);
        
        // Time-based ceil  
        let sel = SampleSelector::time_ceil(1.02);
        assert_eq!(sel.get_index(&ts, num_samples), 25);
    }
    
    #[test]
    fn test_sample_selector_interp() {
        let ts = TimeSampling::uniform(1.0, 0.0); // 1 fps
        let num_samples = 10;
        
        // Exactly at sample
        let sel = SampleSelector::time_near(5.0);
        let interp = sel.get_sample_interp(&ts, num_samples);
        assert!(interp.is_exact());
        assert_eq!(interp.floor_index, 5);
        
        // Between samples
        let sel = SampleSelector::time_near(5.5);
        let interp = sel.get_sample_interp(&ts, num_samples);
        assert!(!interp.is_exact());
        assert_eq!(interp.floor_index, 5);
        assert_eq!(interp.ceil_index, 6);
        assert!((interp.alpha - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_sample_selector_helpers() {
        let sel = SampleSelector::index(5);
        assert!(sel.is_index());
        assert!(!sel.is_time_based());
        assert_eq!(sel.requested_index(), Some(5));
        assert_eq!(sel.requested_time(), None);
        
        let sel = SampleSelector::time_near(1.5);
        assert!(!sel.is_index());
        assert!(sel.is_time_based());
        assert_eq!(sel.requested_index(), None);
        assert_eq!(sel.requested_time(), Some(1.5));
    }
}
