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
        if object.matchesSchema(LIGHT_SCHEMA) {
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
    pub fn getName(&self) -> &str {
        self.object.getName()
    }
    
    /// Get the full path.
    pub fn getFullName(&self) -> &str {
        self.object.getFullName()
    }
    
    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        // Lights store samples in .geom/.camera/.core
        let props = self.object.getProperties();
        
        if let Some(geom_prop) = props.getPropertyByName(".geom") {
            if let Some(geom) = geom_prop.asCompound() {
                // Check .camera/.core for sample count
                if let Some(cam_prop) = geom.getPropertyByName(".camera") {
                    if let Some(cam) = cam_prop.asCompound() {
                        if let Some(core_prop) = cam.getPropertyByName(".core") {
                            if let Some(scalar) = core_prop.asScalar() {
                                return scalar.getNumSamples();
                            }
                        }
                    }
                }
            }
        }
        1
    }
    
    /// Check if this light is constant (single sample).
    pub fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Get time sampling index (from child bounds or camera schema).
    /// Follows original Alembic: checks childBounds first, then camera schema.
    pub fn getTimeSamplingIndex(&self) -> u32 {
        let props = self.object.getProperties();
        let Some(geom_prop) = props.getPropertyByName(".geom") else { return 0 };
        let Some(geom) = geom_prop.asCompound() else { return 0 };
        
        // Try child bounds first
        if let Some(bnds_prop) = geom.getPropertyByName(".childBnds") {
            let ts = bnds_prop.getHeader().time_sampling_index;
            if ts > 0 {
                return ts;
            }
        }
        
        // Fall back to camera schema core property
        if let Some(cam_prop) = geom.getPropertyByName(".camera") {
            if let Some(cam) = cam_prop.asCompound() {
                if let Some(core_prop) = cam.getPropertyByName(".core") {
                    return core_prop.getHeader().time_sampling_index;
                }
            }
        }
        
        0
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        let props = self.object.getProperties();
        let Some(geom_prop) = props.getPropertyByName(".geom") else { return 0 };
        let Some(geom) = geom_prop.asCompound() else { return 0 };
        let Some(bnds_prop) = geom.getPropertyByName(".childBnds") else { return 0 };
        bnds_prop.getHeader().time_sampling_index
    }
    
    /// Get available property names.
    pub fn getPropertyNames(&self) -> Vec<String> {
        self.object.getProperties().getPropertyNames()
    }
    
    /// Read a sample at the given index.
    pub fn getSample(&self, index: usize) -> Result<LightSample> {
        let mut sample = LightSample::new();
        
        let props = self.object.getProperties();
        
        // Try to read camera-like properties from .geom/.camera/.core
        if let Some(geom_prop) = props.getPropertyByName(".geom") {
            if let Some(geom) = geom_prop.asCompound() {
                // Read camera parameters from embedded camera schema
                if let Some(cam_prop) = geom.getPropertyByName(".camera") {
                    if let Some(cam) = cam_prop.asCompound() {
                        sample.camera = Self::read_camera_core(&cam, index);
                    }
                }
                
                // Read .childBnds if present
                if let Some(bnds_prop) = geom.getPropertyByName(".childBnds") {
                    if let Some(scalar) = bnds_prop.asScalar() {
                        let mut buf = [0u8; 48];
                        if scalar.getSample(index, &mut buf).is_ok() {
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
    
    /// Read camera parameters from .core scalar property.
    fn read_camera_core(cam: &crate::abc::ICompoundProperty<'_>, index: usize) -> CameraSample {
        let mut sample = CameraSample::default();
        
        // Read .core (combined camera parameters as 16 doubles)
        if let Some(core_prop) = cam.getPropertyByName(".core") {
            if let Some(scalar) = core_prop.asScalar() {
                let mut buf = vec![0u8; 16 * 8]; // 16 doubles
                if scalar.getSample(index, &mut buf).is_ok() {
                    let doubles: &[f64] = bytemuck::try_cast_slice(&buf).unwrap_or(&[]);
                    if doubles.len() >= 16 {
                        sample.focal_length = doubles[0];
                        sample.horizontal_aperture = doubles[1];
                        sample.horizontal_film_offset = doubles[2];
                        sample.vertical_aperture = doubles[3];
                        sample.vertical_film_offset = doubles[4];
                        sample.lens_squeeze_ratio = doubles[5];
                        sample.overscan_left = doubles[6];
                        sample.overscan_right = doubles[7];
                        sample.overscan_top = doubles[8];
                        sample.overscan_bottom = doubles[9];
                        sample.f_stop = doubles[10];
                        sample.focus_distance = doubles[11];
                        sample.shutter_open = doubles[12];
                        sample.shutter_close = doubles[13];
                        sample.near_clipping_plane = doubles[14];
                        sample.far_clipping_plane = doubles[15];
                    }
                }
            }
        }
        
        sample
    }
    
    /// Check if this light has child bounds.
    pub fn has_child_bounds(&self) -> bool {
        let props = self.object.getProperties();
        if let Some(geom_prop) = props.getPropertyByName(".geom") {
            if let Some(geom) = geom_prop.asCompound() {
                return geom.hasProperty(".childBnds");
            }
        }
        false
    }
    
    /// Check if this light has arbitrary geometry params.
    pub fn has_arb_geom_params(&self) -> bool {
        let props = self.object.getProperties();
        if let Some(geom_prop) = props.getPropertyByName(".geom") {
            if let Some(geom) = geom_prop.asCompound() {
                return geom.hasProperty(".arbGeomParams");
            }
        }
        false
    }
    
    /// Check if this light has user properties.
    pub fn has_user_properties(&self) -> bool {
        let props = self.object.getProperties();
        if let Some(geom_prop) = props.getPropertyByName(".geom") {
            if let Some(geom) = geom_prop.asCompound() {
                return geom.hasProperty(".userProperties");
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
