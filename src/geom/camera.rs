//! Camera schema implementation.
//!
//! Provides reading of camera data from Alembic files.

use crate::abc::IObject;
use crate::util::Result;

/// Camera schema identifier.
pub const CAMERA_SCHEMA: &str = "AbcGeom_Camera_v1";

/// Camera sample data.
#[derive(Clone, Debug)]
pub struct CameraSample {
    /// Focal length in millimeters.
    pub focal_length: f64,
    /// Horizontal aperture (film width) in centimeters.
    pub horizontal_aperture: f64,
    /// Vertical aperture (film height) in centimeters.
    pub vertical_aperture: f64,
    /// Horizontal film offset in centimeters.
    pub horizontal_film_offset: f64,
    /// Vertical film offset in centimeters.
    pub vertical_film_offset: f64,
    /// Near clipping plane distance.
    pub near_clipping_plane: f64,
    /// Far clipping plane distance.
    pub far_clipping_plane: f64,
    /// Focus distance.
    pub focus_distance: f64,
    /// F-stop (aperture size).
    pub f_stop: f64,
    /// Shutter open time (fraction of frame).
    pub shutter_open: f64,
    /// Shutter close time (fraction of frame).
    pub shutter_close: f64,
    /// Lens squeeze ratio (for anamorphic lenses).
    pub lens_squeeze_ratio: f64,
    /// Overscan left.
    pub overscan_left: f64,
    /// Overscan right.
    pub overscan_right: f64,
    /// Overscan top.
    pub overscan_top: f64,
    /// Overscan bottom.
    pub overscan_bottom: f64,
}

impl Default for CameraSample {
    fn default() -> Self {
        Self {
            focal_length: 35.0,
            horizontal_aperture: 3.6,  // 36mm = 3.6cm
            vertical_aperture: 2.4,    // 24mm = 2.4cm
            horizontal_film_offset: 0.0,
            vertical_film_offset: 0.0,
            near_clipping_plane: 0.1,
            far_clipping_plane: 100000.0,
            focus_distance: 5.0,
            f_stop: 5.6,
            shutter_open: 0.0,
            shutter_close: 0.0,
            lens_squeeze_ratio: 1.0,
            overscan_left: 0.0,
            overscan_right: 0.0,
            overscan_top: 0.0,
            overscan_bottom: 0.0,
        }
    }
}

impl CameraSample {
    /// Create a new camera sample with default values.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Compute horizontal field of view in radians.
    pub fn horizontal_fov(&self) -> f64 {
        2.0 * (self.horizontal_aperture / (2.0 * self.focal_length / 10.0)).atan()
    }
    
    /// Compute vertical field of view in radians.
    pub fn vertical_fov(&self) -> f64 {
        2.0 * (self.vertical_aperture / (2.0 * self.focal_length / 10.0)).atan()
    }
    
    /// Compute aspect ratio.
    pub fn aspect_ratio(&self) -> f64 {
        self.horizontal_aperture / self.vertical_aperture
    }
}

/// Input Camera schema reader.
pub struct ICamera<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ICamera<'a> {
    /// Wrap an IObject as ICamera.
    /// Returns None if the object doesn't have the Camera schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matches_schema(CAMERA_SCHEMA) {
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
    
    /// Get property names.
    pub fn property_names(&self) -> Vec<String> {
        let props = self.object.properties();
        if let Some(cam_prop) = props.property_by_name(".camera") {
            if let Some(cam) = cam_prop.as_compound() {
                return cam.property_names();
            }
        }
        Vec::new()
    }
    
    /// Get number of samples.
    pub fn num_samples(&self) -> usize {
        let props = self.object.properties();
        let Some(cam_prop) = props.property_by_name(".camera") else { return 1 };
        let Some(cam) = cam_prop.as_compound() else { return 1 };
        let Some(core_prop) = cam.property_by_name(".core") else { return 1 };
        let Some(scalar) = core_prop.as_scalar() else { return 1 };
        scalar.num_samples()
    }
    
    /// Check if camera is constant.
    pub fn is_constant(&self) -> bool {
        self.num_samples() <= 1
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<CameraSample> {
        use crate::util::Error;
        
        let props = self.object.properties();
        let cam_prop = props.property_by_name(".camera")
            .ok_or_else(|| Error::invalid("No .camera property"))?;
        let cam = cam_prop.as_compound()
            .ok_or_else(|| Error::invalid(".camera is not compound"))?;
        
        let mut sample = CameraSample::new();
        
        // Read .core (combined camera parameters as doubles array)
        // Order: focalLength, horizontalAperture, verticalAperture,
        //        horizontalFilmOffset, verticalFilmOffset, lensSqueezeRatio,
        //        overscanLeft, overscanRight, overscanTop, overscanBottom,
        //        fStop, focusDistance, shutterOpen, shutterClose,
        //        nearClippingPlane, farClippingPlane
        if let Some(core_prop) = cam.property_by_name(".core") {
            if let Some(scalar) = core_prop.as_scalar() {
                let mut buf = vec![0u8; 16 * 8]; // 16 doubles
                if scalar.read_sample(index, &mut buf).is_ok() {
                    let doubles: &[f64] = bytemuck::cast_slice(&buf);
                    if doubles.len() >= 16 {
                        sample.focal_length = doubles[0];
                        sample.horizontal_aperture = doubles[1];
                        sample.vertical_aperture = doubles[2];
                        sample.horizontal_film_offset = doubles[3];
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
        
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_camera_sample_default() {
        let sample = CameraSample::new();
        assert_eq!(sample.focal_length, 35.0);
        assert!((sample.aspect_ratio() - 1.5).abs() < 0.01);
    }
    
    #[test]
    fn test_camera_fov() {
        let mut sample = CameraSample::new();
        sample.focal_length = 50.0;
        sample.horizontal_aperture = 3.6;
        
        // Approximate check for 50mm lens FOV
        let fov = sample.horizontal_fov().to_degrees();
        assert!(fov > 35.0 && fov < 45.0);
    }
}
