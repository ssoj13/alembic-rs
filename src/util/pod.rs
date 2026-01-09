//! Plain Old Data types - fundamental storage types in Alembic.

use bytemuck::{Pod, Zeroable};
use half::f16;
use std::fmt;

/// Plain Old Data type enum - represents basic storage types.
///
/// These are the fundamental types that can be stored in Alembic properties.
/// Each type has a fixed size and well-defined binary representation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum PlainOldDataType {
    /// Boolean (stored as u8: 0 = false, non-zero = true)
    Boolean = 0,
    /// Unsigned 8-bit integer
    Uint8 = 1,
    /// Signed 8-bit integer
    Int8 = 2,
    /// Unsigned 16-bit integer
    Uint16 = 3,
    /// Signed 16-bit integer
    Int16 = 4,
    /// Unsigned 32-bit integer
    Uint32 = 5,
    /// Signed 32-bit integer
    Int32 = 6,
    /// Unsigned 64-bit integer
    Uint64 = 7,
    /// Signed 64-bit integer
    Int64 = 8,
    /// 16-bit floating point (IEEE 754 half precision)
    Float16 = 9,
    /// 32-bit floating point (IEEE 754 single precision)
    Float32 = 10,
    /// 64-bit floating point (IEEE 754 double precision)
    Float64 = 11,
    /// UTF-8 string
    String = 12,
    /// Wide string (stored as UTF-8 in Rust)
    Wstring = 13,
    /// Unknown/invalid type
    #[default]
    Unknown = 127,
}

impl PlainOldDataType {
    /// Number of POD types (excluding Unknown)
    pub const COUNT: usize = 14;

    /// Returns the size in bytes of a single element of this type.
    /// For String/Wstring, returns size of a pointer (platform-dependent).
    #[inline]
    pub const fn num_bytes(self) -> usize {
        match self {
            Self::Boolean => 1,
            Self::Uint8 => 1,
            Self::Int8 => 1,
            Self::Uint16 => 2,
            Self::Int16 => 2,
            Self::Uint32 => 4,
            Self::Int32 => 4,
            Self::Uint64 => 8,
            Self::Int64 => 8,
            Self::Float16 => 2,
            Self::Float32 => 4,
            Self::Float64 => 8,
            // Strings are stored separately, this is just for in-memory representation
            Self::String => std::mem::size_of::<usize>(),
            Self::Wstring => std::mem::size_of::<usize>(),
            Self::Unknown => 0,
        }
    }

    /// Returns the name of this type as a string.
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Boolean => "bool_t",
            Self::Uint8 => "uint8_t",
            Self::Int8 => "int8_t",
            Self::Uint16 => "uint16_t",
            Self::Int16 => "int16_t",
            Self::Uint32 => "uint32_t",
            Self::Int32 => "int32_t",
            Self::Uint64 => "uint64_t",
            Self::Int64 => "int64_t",
            Self::Float16 => "float16_t",
            Self::Float32 => "float32_t",
            Self::Float64 => "float64_t",
            Self::String => "string",
            Self::Wstring => "wstring",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Parse POD type from its name string.
    pub fn from_name(name: &str) -> Self {
        match name {
            "bool_t" => Self::Boolean,
            "uint8_t" => Self::Uint8,
            "int8_t" => Self::Int8,
            "uint16_t" => Self::Uint16,
            "int16_t" => Self::Int16,
            "uint32_t" => Self::Uint32,
            "int32_t" => Self::Int32,
            "uint64_t" => Self::Uint64,
            "int64_t" => Self::Int64,
            "float16_t" => Self::Float16,
            "float32_t" => Self::Float32,
            "float64_t" => Self::Float64,
            "string" => Self::String,
            "wstring" => Self::Wstring,
            _ => Self::Unknown,
        }
    }

    /// Convert from u8 value.
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Boolean,
            1 => Self::Uint8,
            2 => Self::Int8,
            3 => Self::Uint16,
            4 => Self::Int16,
            5 => Self::Uint32,
            6 => Self::Int32,
            7 => Self::Uint64,
            8 => Self::Int64,
            9 => Self::Float16,
            10 => Self::Float32,
            11 => Self::Float64,
            12 => Self::String,
            13 => Self::Wstring,
            _ => Self::Unknown,
        }
    }

    /// Returns true if this is a numeric type (int or float).
    #[inline]
    pub const fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Uint8
                | Self::Int8
                | Self::Uint16
                | Self::Int16
                | Self::Uint32
                | Self::Int32
                | Self::Uint64
                | Self::Int64
                | Self::Float16
                | Self::Float32
                | Self::Float64
        )
    }

    /// Returns true if this is an integer type.
    #[inline]
    pub const fn is_integer(self) -> bool {
        matches!(
            self,
            Self::Uint8
                | Self::Int8
                | Self::Uint16
                | Self::Int16
                | Self::Uint32
                | Self::Int32
                | Self::Uint64
                | Self::Int64
        )
    }

    /// Returns true if this is a floating point type.
    #[inline]
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float16 | Self::Float32 | Self::Float64)
    }

    /// Returns true if this is a string type.
    #[inline]
    pub const fn is_string(self) -> bool {
        matches!(self, Self::String | Self::Wstring)
    }
}

impl fmt::Display for PlainOldDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}



// === POD Trait for type-safe conversions ===

/// Trait for types that can be stored as Alembic POD data.
pub trait AlembicPod: Pod + Zeroable + Copy + Default {
    /// The corresponding PlainOldDataType enum value.
    const POD_TYPE: PlainOldDataType;

    /// Size of this type in bytes.
    const SIZE: usize = std::mem::size_of::<Self>();
}

// Implement AlembicPod for primitive types
impl AlembicPod for u8 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Uint8;
}

impl AlembicPod for i8 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Int8;
}

impl AlembicPod for u16 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Uint16;
}

impl AlembicPod for i16 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Int16;
}

impl AlembicPod for u32 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Uint32;
}

impl AlembicPod for i32 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Int32;
}

impl AlembicPod for u64 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Uint64;
}

impl AlembicPod for i64 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Int64;
}

impl AlembicPod for f32 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Float32;
}

impl AlembicPod for f64 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Float64;
}

impl AlembicPod for f16 {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Float16;
}

/// Boolean type with guaranteed 1-byte storage (like C++ Alembic bool_t).
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct Bool(u8);

impl Bool {
    pub const TRUE: Self = Self(1);
    pub const FALSE: Self = Self(0);

    #[inline]
    pub const fn new(v: bool) -> Self {
        Self(v as u8)
    }

    #[inline]
    pub const fn get(self) -> bool {
        self.0 != 0
    }
}

impl From<bool> for Bool {
    #[inline]
    fn from(v: bool) -> Self {
        Self::new(v)
    }
}

impl From<Bool> for bool {
    #[inline]
    fn from(v: Bool) -> Self {
        v.get()
    }
}

impl fmt::Debug for Bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl fmt::Display for Bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl AlembicPod for Bool {
    const POD_TYPE: PlainOldDataType = PlainOldDataType::Boolean;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pod_sizes() {
        assert_eq!(PlainOldDataType::Boolean.num_bytes(), 1);
        assert_eq!(PlainOldDataType::Uint8.num_bytes(), 1);
        assert_eq!(PlainOldDataType::Int32.num_bytes(), 4);
        assert_eq!(PlainOldDataType::Float32.num_bytes(), 4);
        assert_eq!(PlainOldDataType::Float64.num_bytes(), 8);
        assert_eq!(PlainOldDataType::Float16.num_bytes(), 2);
    }

    #[test]
    fn test_pod_names() {
        assert_eq!(PlainOldDataType::Boolean.name(), "bool_t");
        assert_eq!(PlainOldDataType::Float32.name(), "float32_t");
        assert_eq!(PlainOldDataType::from_name("int32_t"), PlainOldDataType::Int32);
    }

    #[test]
    fn test_bool_type() {
        let t = Bool::new(true);
        let f = Bool::new(false);
        assert!(t.get());
        assert!(!f.get());
        assert_eq!(std::mem::size_of::<Bool>(), 1);
    }

    #[test]
    fn test_pod_roundtrip() {
        for i in 0..14u8 {
            let pod = PlainOldDataType::from_u8(i);
            assert_ne!(pod, PlainOldDataType::Unknown);
            assert_eq!(PlainOldDataType::from_name(pod.name()), pod);
        }
    }
}
