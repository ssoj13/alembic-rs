//! Common utilities for geometry schemas.
//!
//! This module provides shared functionality to reduce code duplication
//! across geometry schema implementations.

use crate::abc::IObject;
use crate::util::BBox3d;

// ============================================================================
// Sample Count Helpers  
// ============================================================================

/// Get number of samples from an array property in .geom.
#[inline]
pub fn num_samples_from_property(object: &IObject<'_>, prop_name: &str) -> usize {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return 1 };
    let Some(geom) = geom_prop.asCompound() else { return 1 };
    let Some(prop) = geom.getPropertyByName(prop_name) else { return 1 };
    let Some(array) = prop.asArray() else { return 1 };
    array.getNumSamples()
}

/// Get number of samples from P (positions) property.
#[inline]
pub fn num_samples_from_positions(object: &IObject<'_>) -> usize {
    num_samples_from_property(object, "P")
}

// ============================================================================
// Arbitrary Geometry Parameters
// ============================================================================

/// Check if the schema has arbitrary geometry parameters.
pub fn has_arb_geom_params(object: &IObject<'_>) -> bool {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
    let Some(geom) = geom_prop.asCompound() else { return false };
    geom.hasProperty(".arbGeomParams")
}

/// Get names of arbitrary geometry parameters.
pub fn arb_geom_param_names(object: &IObject<'_>) -> Vec<String> {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return Vec::new() };
    let Some(geom) = geom_prop.asCompound() else { return Vec::new() };
    let Some(arb_prop) = geom.getPropertyByName(".arbGeomParams") else { return Vec::new() };
    let Some(arb) = arb_prop.asCompound() else { return Vec::new() };
    arb.getPropertyNames()
}

// ============================================================================
// User Properties
// ============================================================================

/// Check if the schema has user properties.
pub fn has_user_properties(object: &IObject<'_>) -> bool {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
    let Some(geom) = geom_prop.asCompound() else { return false };
    geom.hasProperty(".userProperties")
}

/// Get names of user properties.
pub fn user_property_names(object: &IObject<'_>) -> Vec<String> {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return Vec::new() };
    let Some(geom) = geom_prop.asCompound() else { return Vec::new() };
    let Some(user_prop) = geom.getPropertyByName(".userProperties") else { return Vec::new() };
    let Some(user) = user_prop.asCompound() else { return Vec::new() };
    user.getPropertyNames()
}

// ============================================================================
// Bounds Helpers
// ============================================================================

/// Check if the schema has self bounds (.selfBnds).
pub fn has_self_bounds(object: &IObject<'_>) -> bool {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
    let Some(geom) = geom_prop.asCompound() else { return false };
    geom.hasProperty(".selfBnds")
}

/// Check if the schema has child bounds (.childBnds).
pub fn has_child_bounds(object: &IObject<'_>) -> bool {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
    let Some(geom) = geom_prop.asCompound() else { return false };
    geom.hasProperty(".childBnds")
}

/// Read child bounds at a given sample index.
pub fn read_child_bounds(object: &IObject<'_>, index: usize) -> Option<BBox3d> {
    let props = object.getProperties();
    let geom_prop = props.getPropertyByName(".geom")?;
    let geom = geom_prop.asCompound()?;
    let bnds_prop = geom.getPropertyByName(".childBnds")?;
    let scalar = bnds_prop.asScalar()?;
    
    let mut buf = [0u8; 48]; // 6 x f64
    scalar.getSample(index, &mut buf).ok()?;
    
    let doubles: &[f64] = bytemuck::try_cast_slice(&buf).ok()?;
    if doubles.len() >= 6 {
        Some(BBox3d::new(
            glam::dvec3(doubles[0], doubles[1], doubles[2]),
            glam::dvec3(doubles[3], doubles[4], doubles[5]),
        ))
    } else {
        None
    }
}

/// Get number of child bounds samples.
pub fn child_bounds_num_samples(object: &IObject<'_>) -> usize {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return 0 };
    let Some(geom) = geom_prop.asCompound() else { return 0 };
    let Some(bnds_prop) = geom.getPropertyByName(".childBnds") else { return 0 };
    let Some(scalar) = bnds_prop.asScalar() else { return 0 };
    scalar.getNumSamples()
}

/// Get the time sampling index for child bounds.
pub fn child_bounds_time_sampling_index(object: &IObject<'_>) -> u32 {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return 0 };
    let Some(geom) = geom_prop.asCompound() else { return 0 };
    let Some(bnds_prop) = geom.getPropertyByName(".childBnds") else { return 0 };
    bnds_prop.getHeader().time_sampling_index
}

/// Get time sampling index from positions property (P).
/// Used for curves, points, polymesh, etc.
pub fn positions_time_sampling_index(object: &IObject<'_>) -> u32 {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return 0 };
    let Some(geom) = geom_prop.asCompound() else { return 0 };
    let Some(p_prop) = geom.getPropertyByName("P") else { return 0 };
    p_prop.getHeader().time_sampling_index
}

/// Get time sampling index from a named property in .geom compound.
pub fn property_time_sampling_index(object: &IObject<'_>, prop_name: &str) -> u32 {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return 0 };
    let Some(geom) = geom_prop.asCompound() else { return 0 };
    let Some(prop) = geom.getPropertyByName(prop_name) else { return 0 };
    prop.getHeader().time_sampling_index
}

/// Get time sampling index from a schema-specific compound (e.g. .xform, .camera, .light).
pub fn schema_property_time_sampling_index(object: &IObject<'_>, schema_name: &str, prop_name: &str) -> u32 {
    let props = object.getProperties();
    let Some(schema_prop) = props.getPropertyByName(schema_name) else { return 0 };
    let Some(schema) = schema_prop.asCompound() else { return 0 };
    let Some(prop) = schema.getPropertyByName(prop_name) else { return 0 };
    prop.getHeader().time_sampling_index
}

// ============================================================================
// Property Access Helpers
// ============================================================================

/// Check if a property exists in .geom compound.
pub fn has_geom_property(object: &IObject<'_>, prop_name: &str) -> bool {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return false };
    let Some(geom) = geom_prop.asCompound() else { return false };
    geom.hasProperty(prop_name)
}

/// Get property names from .geom compound.
pub fn geom_property_names(object: &IObject<'_>) -> Vec<String> {
    let props = object.getProperties();
    let Some(geom_prop) = props.getPropertyByName(".geom") else { return Vec::new() };
    let Some(geom) = geom_prop.asCompound() else { return Vec::new() };
    geom.getPropertyNames()
}

// ============================================================================
// Array Property Reading Helpers
// ============================================================================

use crate::core::CompoundPropertyReader;

/// Read a Vec3 array property from a compound.
/// Handles both simple arrays and GeomParam compounds (with .vals inside).
pub fn read_vec3_array(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<Vec<glam::Vec3>> {
    let prop = geom.getPropertyByName(prop_name)?;
    
    // Try as GeomParam compound first (has .vals inside)
    if let Some(compound) = prop.asCompound() {
        if let Some(vals_prop) = compound.getPropertyByName(".vals") {
            if let Some(array) = vals_prop.asArray() {
                if let Ok(data) = array.getSampleVec(index) {
                    if let Ok(floats) = bytemuck::try_cast_slice::<u8, f32>(&data) {
                        return Some(floats.chunks_exact(3)
                            .map(|c| glam::vec3(c[0], c[1], c[2]))
                            .collect());
                    }
                }
            }
        }
    }
    
    // Fall back to simple array
    let array = prop.asArray()?;
    let data = array.getSampleVec(index).ok()?;
    let floats: &[f32] = bytemuck::try_cast_slice(&data).ok()?;
    Some(floats.chunks_exact(3)
        .map(|c| glam::vec3(c[0], c[1], c[2]))
        .collect())
}

/// Read an optional Vec3 array (doesn't error if missing).
pub fn read_vec3_array_opt(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<Vec<glam::Vec3>> {
    read_vec3_array(geom, prop_name, index)
}

/// Check if a property is a simple array (not a GeomParam compound).
/// Returns true if the property exists and is a direct array, false if it's a compound or missing.
pub fn is_simple_array(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
) -> bool {
    if let Some(prop) = geom.getPropertyByName(prop_name) {
        // If it's directly an array (not compound), it's a simple array
        prop.asArray().is_some()
    } else {
        false
    }
}

/// Read a Vec2 array property from a compound.
/// Handles both simple arrays and GeomParam compounds (with .vals inside).
pub fn read_vec2_array(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<Vec<glam::Vec2>> {
    let prop = geom.getPropertyByName(prop_name)?;
    
    // Try as GeomParam compound first (has .vals inside)
    if let Some(compound) = prop.asCompound() {
        if let Some(vals_prop) = compound.getPropertyByName(".vals") {
            if let Some(array) = vals_prop.asArray() {
                if let Ok(data) = array.getSampleVec(index) {
                    if let Ok(floats) = bytemuck::try_cast_slice::<u8, f32>(&data) {
                        return Some(floats.chunks_exact(2)
                            .map(|c| glam::vec2(c[0], c[1]))
                            .collect());
                    }
                }
            }
        }
    }
    
    // Fall back to simple array
    let array = prop.asArray()?;
    let data = array.getSampleVec(index).ok()?;
    let floats: &[f32] = bytemuck::try_cast_slice(&data).ok()?;
    Some(floats.chunks_exact(2)
        .map(|c| glam::vec2(c[0], c[1]))
        .collect())
}

/// Read an i32 array property from a compound.
pub fn read_i32_array(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<Vec<i32>> {
    let prop = geom.getPropertyByName(prop_name)?;
    let array = prop.asArray()?;
    let data = array.getSampleVec(index).ok()?;
    Some(bytemuck::try_cast_slice::<u8, i32>(&data).ok()?.to_vec())
}

/// Read an f32 array property from a compound.
pub fn read_f32_array(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<Vec<f32>> {
    let prop = geom.getPropertyByName(prop_name)?;
    let array = prop.asArray()?;
    let data = array.getSampleVec(index).ok()?;
    Some(bytemuck::try_cast_slice::<u8, f32>(&data).ok()?.to_vec())
}

/// Read a u64 array property from a compound.
pub fn read_u64_array(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<Vec<u64>> {
    let prop = geom.getPropertyByName(prop_name)?;
    let array = prop.asArray()?;
    let data = array.getSampleVec(index).ok()?;
    Some(bytemuck::try_cast_slice::<u8, u64>(&data).ok()?.to_vec())
}

/// Read an i32 scalar property from a compound.
pub fn read_i32_scalar(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<i32> {
    let prop = geom.getPropertyByName(prop_name)?;
    let scalar = prop.asScalar()?;
    let mut buf = [0u8; 4];
    scalar.getSample(index, &mut buf).ok()?;
    Some(i32::from_le_bytes(buf))
}

/// Read self bounds (.selfBnds) from a .geom compound.
pub fn read_self_bounds(
    geom: &dyn CompoundPropertyReader,
    index: usize,
) -> Option<BBox3d> {
    let bnds_prop = geom.getPropertyByName(".selfBnds")?;
    let scalar = bnds_prop.asScalar()?;
    
    let mut buf = [0u8; 48]; // 6 x f64
    scalar.getSample(index, &mut buf).ok()?;
    
    let doubles: &[f64] = bytemuck::try_cast_slice(&buf).ok()?;
    if doubles.len() >= 6 {
        Some(BBox3d::new(
            glam::dvec3(doubles[0], doubles[1], doubles[2]),
            glam::dvec3(doubles[3], doubles[4], doubles[5]),
        ))
    } else {
        None
    }
}

/// Read indexed Vec3 data (compound with .vals and .indices).
/// Returns (values, optional indices).
pub fn read_indexed_vec3(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<(Vec<glam::Vec3>, Option<Vec<i32>>)> {
    let prop = geom.getPropertyByName(prop_name)?;
    let compound = prop.asCompound()?;
    
    // Read values
    let vals = read_vec3_array(compound, ".vals", index)?;
    
    // Read optional indices
    let indices = read_i32_array(compound, ".indices", index);
    
    Some((vals, indices))
}

/// Read indexed Vec2 data (compound with .vals and .indices).
/// Returns (values, optional indices).
pub fn read_indexed_vec2(
    geom: &dyn CompoundPropertyReader,
    prop_name: &str,
    index: usize,
) -> Option<(Vec<glam::Vec2>, Option<Vec<i32>>)> {
    let prop = geom.getPropertyByName(prop_name)?;
    let compound = prop.asCompound()?;
    
    // Read values
    let vals = read_vec2_array(compound, ".vals", index)?;
    
    // Read optional indices
    let indices = read_i32_array(compound, ".indices", index);
    
    Some((vals, indices))
}

// ============================================================================
// Compute Bounds (for Sample structs)
// ============================================================================

/// Compute bounding box from a slice of Vec3 positions.
#[inline]
pub fn compute_bounds_vec3(positions: &[glam::Vec3]) -> (glam::Vec3, glam::Vec3) {
    if positions.is_empty() {
        return (glam::Vec3::ZERO, glam::Vec3::ZERO);
    }
    
    let mut min = positions[0];
    let mut max = positions[0];
    
    for &p in &positions[1..] {
        min = min.min(p);
        max = max.max(p);
    }
    
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compute_bounds_empty() {
        let (min, max) = compute_bounds_vec3(&[]);
        assert_eq!(min, glam::Vec3::ZERO);
        assert_eq!(max, glam::Vec3::ZERO);
    }
    
    #[test]
    fn test_compute_bounds_single() {
        let positions = vec![glam::vec3(1.0, 2.0, 3.0)];
        let (min, max) = compute_bounds_vec3(&positions);
        assert_eq!(min, glam::vec3(1.0, 2.0, 3.0));
        assert_eq!(max, glam::vec3(1.0, 2.0, 3.0));
    }
    
    #[test]
    fn test_compute_bounds_multiple() {
        let positions = vec![
            glam::vec3(-1.0, 0.0, 0.0),
            glam::vec3(1.0, 2.0, -3.0),
            glam::vec3(0.0, -1.0, 5.0),
        ];
        let (min, max) = compute_bounds_vec3(&positions);
        assert_eq!(min, glam::vec3(-1.0, -1.0, -3.0));
        assert_eq!(max, glam::vec3(1.0, 2.0, 5.0));
    }
}
