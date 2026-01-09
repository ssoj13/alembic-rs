//! Sample types for Alembic properties.
//!
//! Samples represent a single time slice of data for a property.

use crate::util::Chrono;

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
    pub fn is_exact(&self) -> bool {
        self.floor_index == self.ceil_index || self.alpha == 0.0
    }
}

/// Scope/extent of data in a geom schema sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GeometryScope {
    /// Constant for entire object.
    #[default]
    Constant,
    /// Per-face varying.
    Uniform,
    /// Per-vertex.
    Varying,
    /// Per-face-vertex.
    Vertex,
    /// Per-face-vertex (indexed).
    FaceVarying,
}

impl GeometryScope {
    /// Parse from string (as stored in metadata).
    pub fn from_str(s: &str) -> Self {
        match s {
            "con" | "constant" => Self::Constant,
            "uni" | "uniform" => Self::Uniform,
            "var" | "varying" => Self::Varying,
            "vtx" | "vertex" => Self::Vertex,
            "fvr" | "facevarying" => Self::FaceVarying,
            _ => Self::Constant,
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
        assert_eq!(GeometryScope::from_str("fvr"), GeometryScope::FaceVarying);
        assert_eq!(GeometryScope::Vertex.as_str(), "vtx");
    }
}
