//! Camera schema implementation.
//!
//! Provides reading of camera data from Alembic files.

use crate::abc::IObject;
use crate::util::Result;

/// Camera schema identifier.
pub const CAMERA_SCHEMA: &str = "AbcGeom_Camera_v1";

/// Film back transform operation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FilmBackXformOpType {
    /// Translate operation (2 channels: x, y).
    #[default]
    Translate = 0,
    /// Scale operation (2 channels: x, y).
    Scale = 1,
    /// Full 3x3 matrix (9 channels).
    Matrix = 2,
}

impl FilmBackXformOpType {
    /// Get number of channels for this operation type.
    pub fn num_channels(&self) -> usize {
        match self {
            Self::Translate | Self::Scale => 2,
            Self::Matrix => 9,
        }
    }
    
    /// Create from raw value.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Translate),
            1 => Some(Self::Scale),
            2 => Some(Self::Matrix),
            _ => None,
        }
    }
}

/// Film back transform operation.
///
/// Holds data about a particular film back transform operation.
/// Used for pan/scan, letter boxing, and other film back adjustments.
#[derive(Clone, Debug)]
pub struct FilmBackXformOp {
    /// Operation type.
    op_type: FilmBackXformOpType,
    /// Hint string for DCC interpretation (e.g., Maya).
    hint: String,
    /// Channel values.
    channels: Vec<f64>,
}

impl Default for FilmBackXformOp {
    fn default() -> Self {
        Self {
            op_type: FilmBackXformOpType::Translate,
            hint: String::new(),
            channels: vec![0.0, 0.0],
        }
    }
}

impl FilmBackXformOp {
    /// Create a new translate operation.
    pub fn translate(x: f64, y: f64) -> Self {
        Self {
            op_type: FilmBackXformOpType::Translate,
            hint: String::new(),
            channels: vec![x, y],
        }
    }
    
    /// Create a new scale operation.
    pub fn scale(x: f64, y: f64) -> Self {
        Self {
            op_type: FilmBackXformOpType::Scale,
            hint: String::new(),
            channels: vec![x, y],
        }
    }
    
    /// Create a new matrix operation.
    pub fn matrix(m: [[f64; 3]; 3]) -> Self {
        Self {
            op_type: FilmBackXformOpType::Matrix,
            hint: String::new(),
            channels: vec![
                m[0][0], m[0][1], m[0][2],
                m[1][0], m[1][1], m[1][2],
                m[2][0], m[2][1], m[2][2],
            ],
        }
    }
    
    /// Set hint string.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
    
    /// Get operation type.
    pub fn op_type(&self) -> FilmBackXformOpType {
        self.op_type
    }
    
    /// Get hint string.
    pub fn hint(&self) -> &str {
        &self.hint
    }
    
    /// Get number of channels.
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }
    
    /// Get channel value at index.
    pub fn channel_value(&self, index: usize) -> f64 {
        self.channels.get(index).copied().unwrap_or(0.0)
    }
    
    /// Set channel value at index.
    pub fn set_channel_value(&mut self, index: usize, value: f64) {
        if index < self.channels.len() {
            self.channels[index] = value;
        }
    }
    
    /// Check if this is a translate operation.
    pub fn is_translate_op(&self) -> bool {
        self.op_type == FilmBackXformOpType::Translate
    }
    
    /// Check if this is a scale operation.
    pub fn is_scale_op(&self) -> bool {
        self.op_type == FilmBackXformOpType::Scale
    }
    
    /// Check if this is a matrix operation.
    pub fn is_matrix_op(&self) -> bool {
        self.op_type == FilmBackXformOpType::Matrix
    }
    
    /// Get translate values (x, y).
    pub fn translate_value(&self) -> (f64, f64) {
        (self.channel_value(0), self.channel_value(1))
    }
    
    /// Get scale values (x, y).
    pub fn scale_value(&self) -> (f64, f64) {
        (self.channel_value(0), self.channel_value(1))
    }
    
    /// Set translate values.
    pub fn set_translate(&mut self, x: f64, y: f64) {
        if self.op_type == FilmBackXformOpType::Translate {
            self.channels = vec![x, y];
        }
    }
    
    /// Set scale values.
    pub fn set_scale(&mut self, x: f64, y: f64) {
        if self.op_type == FilmBackXformOpType::Scale {
            self.channels = vec![x, y];
        }
    }
    
    /// Get matrix (row-major 3x3).
    pub fn matrix_value(&self) -> [[f64; 3]; 3] {
        if self.op_type == FilmBackXformOpType::Matrix && self.channels.len() >= 9 {
            [
                [self.channels[0], self.channels[1], self.channels[2]],
                [self.channels[3], self.channels[4], self.channels[5]],
                [self.channels[6], self.channels[7], self.channels[8]],
            ]
        } else {
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        }
    }
    
    /// Set matrix (row-major 3x3).
    pub fn set_matrix(&mut self, m: [[f64; 3]; 3]) {
        if self.op_type == FilmBackXformOpType::Matrix {
            self.channels = vec![
                m[0][0], m[0][1], m[0][2],
                m[1][0], m[1][1], m[1][2],
                m[2][0], m[2][1], m[2][2],
            ];
        }
    }
    
    /// Compute this operation as a 3x3 matrix.
    pub fn as_matrix(&self) -> [[f64; 3]; 3] {
        match self.op_type {
            FilmBackXformOpType::Translate => {
                let (tx, ty) = self.translate_value();
                [[1.0, 0.0, tx], [0.0, 1.0, ty], [0.0, 0.0, 1.0]]
            }
            FilmBackXformOpType::Scale => {
                let (sx, sy) = self.scale_value();
                [[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, 1.0]]
            }
            FilmBackXformOpType::Matrix => self.matrix_value(),
        }
    }
}

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
    /// Film back transform operations.
    pub film_back_xform_ops: Vec<FilmBackXformOp>,
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
            film_back_xform_ops: Vec::new(),
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
    
    /// Get number of film back transform operations.
    pub fn num_ops(&self) -> usize {
        self.film_back_xform_ops.len()
    }
    
    /// Get total number of channels across all operations.
    pub fn num_op_channels(&self) -> usize {
        self.film_back_xform_ops.iter().map(|op| op.num_channels()).sum()
    }
    
    /// Get operation at index.
    pub fn get_op(&self, index: usize) -> Option<&FilmBackXformOp> {
        self.film_back_xform_ops.get(index)
    }
    
    /// Get mutable operation at index.
    pub fn get_op_mut(&mut self, index: usize) -> Option<&mut FilmBackXformOp> {
        self.film_back_xform_ops.get_mut(index)
    }
    
    /// Add a film back transform operation. Returns the index.
    pub fn add_op(&mut self, op: FilmBackXformOp) -> usize {
        let idx = self.film_back_xform_ops.len();
        self.film_back_xform_ops.push(op);
        idx
    }
    
    /// Compute the concatenated film back matrix from all operations.
    pub fn film_back_matrix(&self) -> [[f64; 3]; 3] {
        let mut result = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for op in &self.film_back_xform_ops {
            let m = op.as_matrix();
            result = mul_mat3(result, m);
        }
        result
    }
    
    /// Reset sample to default state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Multiply two 3x3 matrices.
fn mul_mat3(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

/// Input Camera schema reader.
pub struct ICamera<'a> {
    object: &'a IObject<'a>,
}

impl<'a> ICamera<'a> {
    /// Wrap an IObject as ICamera.
    /// Returns None if the object doesn't have the Camera schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matchesSchema(CAMERA_SCHEMA) {
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
    
    /// Get property names.
    pub fn getPropertyNames(&self) -> Vec<String> {
        let props = self.object.getProperties();
        if let Some(cam_prop) = props.getPropertyByName(".geom") {
            if let Some(cam) = cam_prop.asCompound() {
                return cam.getPropertyNames();
            }
        }
        Vec::new()
    }
    
    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else { return 1 };
        let Some(cam) = cam_prop.asCompound() else { return 1 };
        let Some(core_prop) = cam.getPropertyByName(".core") else { return 1 };
        let Some(scalar) = core_prop.asScalar() else { return 1 };
        scalar.getNumSamples()
    }
    
    /// Check if camera is constant.
    pub fn isConstant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Read a sample at the given index.
    pub fn getSample(&self, index: usize) -> Result<CameraSample> {
        use crate::util::Error;
        
        let props = self.object.getProperties();
        let cam_prop = props.getPropertyByName(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let cam = cam_prop.asCompound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        
        let mut sample = CameraSample::new();
        
        // Read .core (combined camera parameters as doubles array)
        // Order: focalLength, horizontalAperture, verticalAperture,
        //        horizontalFilmOffset, verticalFilmOffset, lensSqueezeRatio,
        //        overscanLeft, overscanRight, overscanTop, overscanBottom,
        //        fStop, focusDistance, shutterOpen, shutterClose,
        //        nearClippingPlane, farClippingPlane
        if let Some(core_prop) = cam.getPropertyByName(".core") {
            if let Some(scalar) = core_prop.asScalar() {
                let mut buf = vec![0u8; 16 * 8]; // 16 doubles
                if scalar.getSample(index, &mut buf).is_ok() {
                    let doubles: &[f64] = bytemuck::try_cast_slice(&buf).unwrap_or(&[]);
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
    
    /// Check if this camera has child bounds property.
    pub fn has_child_bounds(&self) -> bool {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else {
            return false;
        };
        let Some(cam) = cam_prop.asCompound() else {
            return false;
        };
        cam.hasProperty(".childBnds")
    }
    
    /// Read child bounds at the given sample index.
    pub fn child_bounds(&self, index: usize) -> Option<crate::util::BBox3d> {
        let props = self.object.getProperties();
        let cam_prop = props.getPropertyByName(".geom")?;
        let cam = cam_prop.asCompound()?;
        let bnds_prop = cam.getPropertyByName(".childBnds")?;
        let scalar = bnds_prop.asScalar()?;
        
        let mut buf = [0u8; 48]; // 6 x f64
        if scalar.getSample(index, &mut buf).is_ok() {
            let doubles: &[f64] = bytemuck::try_cast_slice(&buf).unwrap_or(&[]);
            if doubles.len() >= 6 {
                return Some(crate::util::BBox3d::new(
                    glam::dvec3(doubles[0], doubles[1], doubles[2]),
                    glam::dvec3(doubles[3], doubles[4], doubles[5]),
                ));
            }
        }
        None
    }
    
    /// Get the number of child bounds samples.
    pub fn child_bounds_num_samples(&self) -> usize {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else { return 0 };
        let Some(cam) = cam_prop.asCompound() else { return 0 };
        let Some(bnds_prop) = cam.getPropertyByName(".childBnds") else { return 0 };
        let Some(scalar) = bnds_prop.asScalar() else { return 0 };
        scalar.getNumSamples()
    }
    
    /// Get time sampling index from core properties.
    pub fn getTimeSamplingIndex(&self) -> u32 {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else { return 0 };
        let Some(cam) = cam_prop.asCompound() else { return 0 };
        let Some(core_prop) = cam.getPropertyByName(".core") else { return 0 };
        core_prop.getHeader().time_sampling_index
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else { return 0 };
        let Some(cam) = cam_prop.asCompound() else { return 0 };
        let Some(bnds_prop) = cam.getPropertyByName(".childBnds") else { return 0 };
        bnds_prop.getHeader().time_sampling_index
    }
    
    /// Check if this camera has arbitrary geometry parameters.
    pub fn has_arb_geom_params(&self) -> bool {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else {
            return false;
        };
        let Some(cam) = cam_prop.asCompound() else {
            return false;
        };
        cam.hasProperty(".arbGeomParams")
    }
    
    /// Get names of arbitrary geometry parameters.
    pub fn arb_geom_param_names(&self) -> Vec<String> {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else {
            return Vec::new();
        };
        let Some(cam) = cam_prop.asCompound() else {
            return Vec::new();
        };
        let Some(arb_prop) = cam.getPropertyByName(".arbGeomParams") else {
            return Vec::new();
        };
        let Some(arb) = arb_prop.asCompound() else {
            return Vec::new();
        };
        arb.getPropertyNames()
    }
    
    /// Check if this camera has user properties.
    pub fn has_user_properties(&self) -> bool {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else {
            return false;
        };
        let Some(cam) = cam_prop.asCompound() else {
            return false;
        };
        cam.hasProperty(".userProperties")
    }
    
    /// Get names of user properties.
    pub fn user_property_names(&self) -> Vec<String> {
        let props = self.object.getProperties();
        let Some(cam_prop) = props.getPropertyByName(".geom") else {
            return Vec::new();
        };
        let Some(cam) = cam_prop.asCompound() else {
            return Vec::new();
        };
        let Some(user_prop) = cam.getPropertyByName(".userProperties") else {
            return Vec::new();
        };
        let Some(user) = user_prop.asCompound() else {
            return Vec::new();
        };
        user.getPropertyNames()
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
