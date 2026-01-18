//! Shared helpers for schema writers.
//!
//! These are small utilities to reduce duplication without changing behavior.

use crate::core::MetaData;

/// Compute bounding box from positions.
/// Matches AbcGeom writer behavior (min/max over points).
pub(crate) fn compute_bounds_vec3(positions: &[glam::Vec3]) -> [f64; 6] {
    if positions.is_empty() {
        return [0.0; 6];
    }
    let mut min = glam::DVec3::splat(f64::MAX);
    let mut max = glam::DVec3::splat(f64::MIN);
    for p in positions {
        let p = glam::DVec3::new(p.x as f64, p.y as f64, p.z as f64);
        min = min.min(p);
        max = max.max(p);
    }
    [min.x, min.y, min.z, max.x, max.y, max.z]
}

/// Standard bounds metadata (interpretation=box).
pub(crate) fn bounds_meta() -> MetaData {
    let mut meta = MetaData::new();
    meta.set("interpretation", "box");
    meta
}
