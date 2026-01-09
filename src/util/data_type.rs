//! DataType - combines POD type with extent (dimensionality).

use super::PlainOldDataType;
use std::fmt;

/// DataType describes how an element of a sample is stored.
///
/// It combines a [`PlainOldDataType`] with an extent (dimensionality).
/// For example, a Vec3f would be Float32 with extent 3.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataType {
    /// The base plain old data type
    pub pod: PlainOldDataType,
    /// Number of POD elements (1 for scalar, 2 for Vec2, 3 for Vec3, etc.)
    pub extent: u8,
}

impl DataType {
    /// Create a new DataType with given POD and extent.
    #[inline]
    pub const fn new(pod: PlainOldDataType, extent: u8) -> Self {
        Self { pod, extent }
    }

    /// Create a scalar DataType (extent = 1).
    #[inline]
    pub const fn scalar(pod: PlainOldDataType) -> Self {
        Self { pod, extent: 1 }
    }

    /// Returns the total size in bytes for one element.
    #[inline]
    pub const fn num_bytes(&self) -> usize {
        self.pod.num_bytes() * self.extent as usize
    }

    /// Returns true if this is a valid (known) type.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        !matches!(self.pod, PlainOldDataType::Unknown) && self.extent > 0
    }

    /// Unknown/invalid DataType.
    pub const UNKNOWN: Self = Self::new(PlainOldDataType::Unknown, 0);

    // === Common predefined types ===

    // Scalars
    pub const BOOL: Self = Self::scalar(PlainOldDataType::Boolean);
    pub const UINT8: Self = Self::scalar(PlainOldDataType::Uint8);
    pub const INT8: Self = Self::scalar(PlainOldDataType::Int8);
    pub const UINT16: Self = Self::scalar(PlainOldDataType::Uint16);
    pub const INT16: Self = Self::scalar(PlainOldDataType::Int16);
    pub const UINT32: Self = Self::scalar(PlainOldDataType::Uint32);
    pub const INT32: Self = Self::scalar(PlainOldDataType::Int32);
    pub const UINT64: Self = Self::scalar(PlainOldDataType::Uint64);
    pub const INT64: Self = Self::scalar(PlainOldDataType::Int64);
    pub const FLOAT16: Self = Self::scalar(PlainOldDataType::Float16);
    pub const FLOAT32: Self = Self::scalar(PlainOldDataType::Float32);
    pub const FLOAT64: Self = Self::scalar(PlainOldDataType::Float64);
    pub const STRING: Self = Self::scalar(PlainOldDataType::String);
    pub const WSTRING: Self = Self::scalar(PlainOldDataType::Wstring);

    // Vectors (float32)
    pub const VEC2F: Self = Self::new(PlainOldDataType::Float32, 2);
    pub const VEC3F: Self = Self::new(PlainOldDataType::Float32, 3);
    pub const VEC4F: Self = Self::new(PlainOldDataType::Float32, 4);

    // Vectors (float64)
    pub const VEC2D: Self = Self::new(PlainOldDataType::Float64, 2);
    pub const VEC3D: Self = Self::new(PlainOldDataType::Float64, 3);
    pub const VEC4D: Self = Self::new(PlainOldDataType::Float64, 4);

    // Vectors (int32)
    pub const VEC2I: Self = Self::new(PlainOldDataType::Int32, 2);
    pub const VEC3I: Self = Self::new(PlainOldDataType::Int32, 3);

    // Matrices (float32) - stored as extent = rows * cols
    pub const MAT33F: Self = Self::new(PlainOldDataType::Float32, 9);
    pub const MAT44F: Self = Self::new(PlainOldDataType::Float32, 16);

    // Matrices (float64)
    pub const MAT33D: Self = Self::new(PlainOldDataType::Float64, 9);
    pub const MAT44D: Self = Self::new(PlainOldDataType::Float64, 16);

    // Quaternion (float32: x, y, z, w)
    pub const QUATF: Self = Self::new(PlainOldDataType::Float32, 4);
    pub const QUATD: Self = Self::new(PlainOldDataType::Float64, 4);

    // Color types
    pub const COLOR3F: Self = Self::new(PlainOldDataType::Float32, 3);
    pub const COLOR4F: Self = Self::new(PlainOldDataType::Float32, 4);

    // Normal type (same as Vec3f but semantically different)
    pub const NORMAL3F: Self = Self::new(PlainOldDataType::Float32, 3);
    pub const NORMAL3D: Self = Self::new(PlainOldDataType::Float64, 3);

    // Point types (same storage as vectors)
    pub const POINT2F: Self = Self::VEC2F;
    pub const POINT3F: Self = Self::VEC3F;
    pub const POINT2D: Self = Self::VEC2D;
    pub const POINT3D: Self = Self::VEC3D;

    // Box types (min + max = 2 * vec3)
    pub const BOX3F: Self = Self::new(PlainOldDataType::Float32, 6);
    pub const BOX3D: Self = Self::new(PlainOldDataType::Float64, 6);
}

impl Default for DataType {
    fn default() -> Self {
        Self::UNKNOWN
    }
}

impl fmt::Debug for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.extent == 1 {
            write!(f, "{}", self.pod.name())
        } else {
            write!(f, "{}[{}]", self.pod.name(), self.extent)
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl PartialOrd for DataType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DataType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.pod.cmp(&other.pod) {
            std::cmp::Ordering::Equal => self.extent.cmp(&other.extent),
            ord => ord,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_sizes() {
        assert_eq!(DataType::BOOL.num_bytes(), 1);
        assert_eq!(DataType::INT32.num_bytes(), 4);
        assert_eq!(DataType::FLOAT32.num_bytes(), 4);
        assert_eq!(DataType::VEC3F.num_bytes(), 12);
        assert_eq!(DataType::MAT44F.num_bytes(), 64);
        assert_eq!(DataType::BOX3D.num_bytes(), 48);
    }

    #[test]
    fn test_data_type_display() {
        assert_eq!(format!("{}", DataType::FLOAT32), "float32_t");
        assert_eq!(format!("{}", DataType::VEC3F), "float32_t[3]");
        assert_eq!(format!("{}", DataType::MAT44F), "float32_t[16]");
    }

    #[test]
    fn test_data_type_validity() {
        assert!(DataType::FLOAT32.is_valid());
        assert!(DataType::VEC3F.is_valid());
        assert!(!DataType::UNKNOWN.is_valid());
        assert!(!DataType::new(PlainOldDataType::Float32, 0).is_valid());
    }
}
