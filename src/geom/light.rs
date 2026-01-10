//! Light schema implementation.
//!
//! Provides reading of light data from Alembic files.
//! Lights in Alembic are container schemas that can contain
//! camera-like properties for light parameters.

use crate::abc::IObject;
use crate::util::{Result, BBox3d};
use super::camera::CameraSample;

/// Light schema identifier.
pub const LIGHT_SCHEMA: &str = "AbcGeom_Light_v1";

/// Light sample data.
/// 
/// Lights in Alembic use camera-like parameters for their properties.
/// The actual light type and parameters are determined by the application
/// reading the file - Alembic itself doesn't define light types.
#[derive(Clone, Debug, Default)]
pub struct LightSample {
    /// Camera parameters (shared with ICamera).
    pub camera: CameraSample,
    /// Child bounds (optional).
    pub child_bounds: Option<BBox3d>,
}

impl LightSample {
    /// Create an empty sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if sample has valid data.
    pub fn is_valid(&self) -> bool {
        true // Light samples are always valid even if empty
    }
}

/// Input Light schema reader.
/// 
/// The Light schema is a container that can hold camera-like parameters
/// for representing light properties. The interpretation of these values
/// depends on the application.
pub struct ILight<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ILight<'a> {
    /// Wrap an IObject as an ILight.
    /// Returns None if the object doesn't have the Light schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(LIGHT_SCHEMA) {
            Some(Self { object })
        } else {
            None
        }
    }
    
    /// Get the underlying object.
    pub fn object(&self) -> &IObject<'a> {
        self.object
    }
    
    /// Get the object name.
    pub fn name(&self) -> &str {
        self.object.name()
    }
    
    /// Get the full path.
    pub fn full_name(&self) -> &str {
        self.object.full_name()
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        // Lights may not have animated properties, default to 1
        let props = self.object.properties();
        
        // Check for camera-like properties
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                // Check focalLength as it's commonly animated
                if let Some(fl_prop) = geom.property_by_name("focalLength") {
                    if let Some(scalar) = fl_prop.as_scalar() {
                        return scalar.num_samples();
                    }
                }
            }
        }
        1
    }
    
    /// Check if this light is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Get available property names.
    pub fn property_names(&self) -> Vec<String> {
        self.object.properties().property_names()
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<LightSample> {
        let mut sample = LightSample::new();
        
        let props = self.object.properties();
        
        // Try to read camera-like properties from .geom
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                // Read camera parameters
                sample.camera = Self::read_camera_params(&geom, index);
                
                // Read .childBnds if present
                if let Some(bnds_prop) = geom.property_by_name(".childBnds") {
                    if let Some(scalar) = bnds_prop.as_scalar() {
                        let mut buf = [0u8; 48];
                        if scalar.read_sample(index, &mut buf).is_ok() {
                            let values: &[f64] = bytemuck::try_cast_slice(&buf).unwrap_or(&[]);
                            if values.len() >= 6 {
                                sample.child_bounds = Some(BBox3d::new(
                                    glam::dvec3(values[0], values[1], values[2]),
                                    glam::dvec3(values[3], values[4], values[5]),
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(sample)
    }
    
    /// Read camera-like parameters from a compound property.
    fn read_camera_params(geom: &crate::abc::ICompoundProperty<'_>, index: usize) -> CameraSample {
        let mut cam = CameraSample::default();
        
        // Helper to read f64 property
        let read_f64 = |name: &str| -> Option<f64> {
            let prop = geom.property_by_name(name)?;
            let scalar = prop.as_scalar()?;
            let mut buf = [0u8; 8];
            scalar.read_sample(index, &mut buf).ok()?;
            Some(f64::from_le_bytes(buf))
        };
        
        // Core lens parameters
        if let Some(v) = read_f64("focalLength") { cam.focal_length = v; }
        if let Some(v) = read_f64("horizontalAperture") { cam.horizontal_aperture = v; }
        if let Some(v) = read_f64("verticalAperture") { cam.vertical_aperture = v; }
        if let Some(v) = read_f64("horizontalFilmOffset") { cam.horizontal_film_offset = v; }
        if let Some(v) = read_f64("verticalFilmOffset") { cam.vertical_film_offset = v; }
        if let Some(v) = read_f64("lensSqueezeRatio") { cam.lens_squeeze_ratio = v; }
        
        // Overscan parameters
        if let Some(v) = read_f64("overscanLeft") { cam.overscan_left = v; }
        if let Some(v) = read_f64("overscanRight") { cam.overscan_right = v; }
        if let Some(v) = read_f64("overscanTop") { cam.overscan_top = v; }
        if let Some(v) = read_f64("overscanBottom") { cam.overscan_bottom = v; }
        
        // Focus/DOF parameters
        if let Some(v) = read_f64("fStop") { cam.f_stop = v; }
        if let Some(v) = read_f64("focusDistance") { cam.focus_distance = v; }
        
        // Shutter parameters
        if let Some(v) = read_f64("shutterOpen") { cam.shutter_open = v; }
        if let Some(v) = read_f64("shutterClose") { cam.shutter_close = v; }
        
        // Clipping planes
        if let Some(v) = read_f64("nearClippingPlane") { cam.near_clipping_plane = v; }
        if let Some(v) = read_f64("farClippingPlane") { cam.far_clipping_plane = v; }
        
        cam
    }
    
    /// Check if this light has child bounds.
    pub fn has_child_bounds(&self) -> bool {
        let props = self.object.properties();
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                return geom.has_property(".childBnds");
            }
        }
        false
    }
    
    /// Check if this light has arbitrary geometry params.
    pub fn has_arb_geom_params(&self) -> bool {
        let props = self.object.properties();
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                return geom.has_property(".arbGeomParams");
            }
        }
        false
    }
    
    /// Check if this light has user properties.
    pub fn has_user_properties(&self) -> bool {
        let props = self.object.properties();
        if let Some(geom_prop) = props.property_by_name(".geom") {
            if let Some(geom) = geom_prop.as_compound() {
                return geom.has_property(".userProperties");
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_light_sample_default() {
        let sample = LightSample::new();
        assert!(sample.is_valid());
        assert!(sample.child_bounds.is_none());
    }
}
