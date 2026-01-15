//! FaceSet schema implementation.
//!
//! FaceSets provide a way to group faces of a mesh for material assignment
//! or other organizational purposes.

use crate::abc::IObject;
use crate::util::{Result, Error, BBox3d};

/// FaceSet schema identifier.
pub const FACESET_SCHEMA: &str = "AbcGeom_FaceSet_v1";

/// Metadata key for face exclusivity.
pub const FACE_EXCLUSIVITY_KEY: &str = "faceExclusivity";

/// Hint to indicate face membership is mutually exclusive.
/// 
/// Some structures that group faces only allow a face to belong
/// to one FaceSet, while other times a face is allowed to belong
/// to any number of FaceSets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FaceSetExclusivity {
    /// Faces can belong to multiple FaceSets (default).
    #[default]
    NonExclusive = 0,
    /// Faces can only belong to one FaceSet.
    Exclusive = 1,
}

impl FaceSetExclusivity {
    /// Parse from string (as stored in metadata).
    pub fn parse(s: &str) -> Self {
        match s {
            "1" | "exclusive" => Self::Exclusive,
            _ => Self::NonExclusive,
        }
    }
    
    /// Convert to string for metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NonExclusive => "0",
            Self::Exclusive => "1",
        }
    }
}

/// FaceSet sample data.
#[derive(Clone, Debug, Default)]
pub struct FaceSetSample {
    /// Face indices that belong to this face set.
    pub faces: Vec<i32>,
    /// Self bounds (optional).
    pub self_bounds: Option<BBox3d>,
}

impl FaceSetSample {
    /// Create an empty sample.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if sample has valid data.
    pub fn is_valid(&self) -> bool {
        !self.faces.is_empty()
    }
    
    /// Get number of faces in this face set.
    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }
    
    /// Check if a face index is in this face set.
    pub fn contains(&self, face_index: i32) -> bool {
        self.faces.contains(&face_index)
    }
}

/// Internal storage for IFaceSet - can be borrowed or owned.
enum FaceSetObject<'a> {
    Borrowed(&'a IObject<'a>),
    Owned(IObject<'a>),
}

impl<'a> FaceSetObject<'a> {
    fn as_ref(&self) -> &IObject<'_> {
        match self {
            Self::Borrowed(r) => r,
            Self::Owned(o) => o,
        }
    }
}

/// Input FaceSet schema reader.
pub struct IFaceSet<'a> {
    object: FaceSetObject<'a>,
    exclusivity: FaceSetExclusivity,
}

impl<'a> IFaceSet<'a> {
    /// Wrap an IObject reference as an IFaceSet.
    /// Returns None if the object doesn't have the FaceSet schema.
    pub fn new(object: &'a IObject<'a>) -> Option<Self> {
        if object.matchesSchema(FACESET_SCHEMA) {
            let exclusivity = Self::read_exclusivity_from(object);
            Some(Self { 
                object: FaceSetObject::Borrowed(object), 
                exclusivity 
            })
        } else {
            None
        }
    }
    
    /// Create an IFaceSet from an owned IObject.
    /// Returns None if the object doesn't have the FaceSet schema.
    pub fn from_owned(object: IObject<'a>) -> Option<Self> {
        if object.matchesSchema(FACESET_SCHEMA) {
            let exclusivity = Self::read_exclusivity_from(&object);
            Some(Self { 
                object: FaceSetObject::Owned(object), 
                exclusivity 
            })
        } else {
            None
        }
    }
    
    /// Read exclusivity from object metadata.
    fn read_exclusivity_from(object: &IObject<'_>) -> FaceSetExclusivity {
        let header = object.getHeader();
        if let Some(excl_str) = header.meta_data.get(FACE_EXCLUSIVITY_KEY) {
            FaceSetExclusivity::parse(excl_str)
        } else {
            FaceSetExclusivity::NonExclusive
        }
    }
    
    /// Get the underlying object.
    pub fn object(&self) -> &IObject<'_> {
        self.object.as_ref()
    }
    
    /// Get the object name.
    pub fn getName(&self) -> &str {
        self.object.as_ref().getName()
    }
    
    /// Get the full path.
    pub fn getFullName(&self) -> &str {
        self.object.as_ref().getFullName()
    }
    
    /// Get the face exclusivity setting.
    pub fn face_exclusivity(&self) -> FaceSetExclusivity {
        self.exclusivity
    }
    
    /// Get number of samples.
    pub fn getNumSamples(&self) -> usize {
        let props = self.object.as_ref().getProperties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 1 };
        let Some(geom) = geom_prop.as_compound() else { return 1 };
        let Some(faces_prop) = geom.property_by_name(".faces") else { return 1 };
        let Some(array_reader) = faces_prop.as_array() else { return 1 };
        array_reader.num_samples()
    }
    
    /// Check if this face set is constant (single sample).
    pub fn is_constant(&self) -> bool {
        self.getNumSamples() <= 1
    }
    
    /// Get time sampling index from faces property.
    pub fn time_sampling_index(&self) -> u32 {
        let props = self.object.as_ref().getProperties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 0 };
        let Some(geom) = geom_prop.as_compound() else { return 0 };
        let Some(faces_prop) = geom.property_by_name(".faces") else { return 0 };
        faces_prop.getHeader().time_sampling_index
    }
    
    /// Get the time sampling index for child bounds property.
    pub fn child_bounds_time_sampling_index(&self) -> u32 {
        let props = self.object.as_ref().getProperties();
        let Some(geom_prop) = props.property_by_name(".geom") else { return 0 };
        let Some(geom) = geom_prop.as_compound() else { return 0 };
        let Some(bnds_prop) = geom.property_by_name(".childBnds") else { return 0 };
        bnds_prop.getHeader().time_sampling_index
    }
    
    /// Read a sample at the given index.
    pub fn get_sample(&self, index: usize) -> Result<FaceSetSample> {
        let mut sample = FaceSetSample::new();
        
        let props = self.object.as_ref().getProperties();
        let geom_prop = props.property_by_name(".geom")
            .ok_or_else(|| Error::invalid("No .geom property"))?;
        let geom = geom_prop.as_compound()
            .ok_or_else(|| Error::invalid(".geom is not compound"))?;
        
        // Read .faces
        if let Some(faces_prop) = geom.property_by_name(".faces") {
            if let Some(array_reader) = faces_prop.as_array() {
                let data = array_reader.read_sample_vec(index)?;
                sample.faces = bytemuck::try_cast_slice::<_, i32>(&data).unwrap_or(&[]).to_vec();
            }
        }
        
        // Read .selfBnds if present
        if let Some(bnds_prop) = geom.property_by_name(".selfBnds") {
            if let Some(scalar) = bnds_prop.as_scalar() {
                // BBox3d is 6 f64 values: min_x, min_y, min_z, max_x, max_y, max_z
                let mut buf = [0u8; 48];
                if scalar.read_sample(index, &mut buf).is_ok() {
                    let values: &[f64] = bytemuck::try_cast_slice(&buf).unwrap_or(&[]);
                    if values.len() >= 6 {
                        sample.self_bounds = Some(BBox3d::new(
                            glam::dvec3(values[0], values[1], values[2]),
                            glam::dvec3(values[3], values[4], values[5]),
                        ));
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
    fn test_faceset_sample_empty() {
        let sample = FaceSetSample::new();
        assert!(!sample.is_valid());
        assert_eq!(sample.num_faces(), 0);
    }
    
    #[test]
    fn test_faceset_sample_basic() {
        let mut sample = FaceSetSample::new();
        sample.faces = vec![0, 1, 2, 5, 10];
        
        assert!(sample.is_valid());
        assert_eq!(sample.num_faces(), 5);
        assert!(sample.contains(2));
        assert!(!sample.contains(3));
    }
    
    #[test]
    fn test_exclusivity() {
        assert_eq!(FaceSetExclusivity::parse("0"), FaceSetExclusivity::NonExclusive);
        assert_eq!(FaceSetExclusivity::parse("1"), FaceSetExclusivity::Exclusive);
        assert_eq!(FaceSetExclusivity::parse("exclusive"), FaceSetExclusivity::Exclusive);
        assert_eq!(FaceSetExclusivity::parse(""), FaceSetExclusivity::NonExclusive);
    }
}
