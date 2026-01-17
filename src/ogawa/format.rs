//! Ogawa format constants and structures.

/// Magic bytes at the start of an Ogawa file.
pub const OGAWA_MAGIC: &[u8; 5] = b"Ogawa";

/// Size of the file header in bytes.
pub const HEADER_SIZE: usize = 16;

/// Offset of the frozen flag in the header.
pub const FROZEN_OFFSET: usize = 5;

/// Offset of the version in the header.
pub const VERSION_OFFSET: usize = 6;

/// Offset of the root group position in the header.
pub const ROOT_POS_OFFSET: usize = 8;

/// Current Ogawa format version.
pub const CURRENT_VERSION: u16 = 1;

/// Frozen flag value when archive is frozen (finalized).
pub const FROZEN_FLAG: u8 = 0xFF;

/// Frozen flag value when archive is not frozen (still being written).
pub const NOT_FROZEN_FLAG: u8 = 0x00;

/// Bit mask for the type flag in child offsets.
/// In ACTUAL Alembic files (contrary to C++ code comments):
/// - MSB SET (1) = DATA
/// - MSB NOT SET (0) = GROUP
///
/// This is the opposite of what CprImpl.cpp comments claim!
pub const TYPE_FLAG_MASK: u64 = 1 << 63;

/// Mask to extract the actual offset from a child pointer.
pub const OFFSET_MASK: u64 = !(1 << 63);

/// Empty group marker - a group with no children.
pub const EMPTY_GROUP_SIZE: u64 = 0;

/// Empty data marker - data with zero bytes.
pub const EMPTY_DATA_SIZE: u64 = 0;

/// Empty data offset marker - offset 0 with MSB set.
/// Used when dimensions can be inferred from data size (rank <= 1, non-string types).
pub const EMPTY_DATA: u64 = TYPE_FLAG_MASK;  // 0x8000000000000000

/// Minimum valid offset for data (after header).
pub const MIN_DATA_OFFSET: u64 = HEADER_SIZE as u64;

/// Check if a child offset represents a group (MSB NOT set = 0).
/// In real Alembic files, MSB=0 means GROUP, MSB=1 means DATA.
#[inline]
pub const fn is_group_offset(offset: u64) -> bool {
    (offset & TYPE_FLAG_MASK) == 0
}

/// Check if a child offset represents data (MSB SET = 1).
/// In real Alembic files, MSB=0 means GROUP, MSB=1 means DATA.
#[inline]
pub const fn is_data_offset(offset: u64) -> bool {
    (offset & TYPE_FLAG_MASK) != 0
}

/// Extract the actual position from a child offset.
#[inline]
pub const fn extract_offset(offset: u64) -> u64 {
    offset & OFFSET_MASK
}

/// Create a group child offset (MSB clear = 0).
#[inline]
pub const fn make_group_offset(pos: u64) -> u64 {
    pos & OFFSET_MASK
}

/// Create a data child offset (MSB set = 1).
#[inline]
pub const fn make_data_offset(pos: u64) -> u64 {
    pos | TYPE_FLAG_MASK
}

/// Check if an offset is the "empty" marker for groups or data.
#[inline]
pub const fn is_empty_offset(offset: u64) -> bool {
    extract_offset(offset) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic() {
        assert_eq!(OGAWA_MAGIC, b"Ogawa");
        assert_eq!(OGAWA_MAGIC.len(), 5);
    }

    #[test]
    fn test_offsets() {
        // Group has MSB clear (0) - this is the ACTUAL file format
        let group_offset = make_group_offset(0x1234);
        assert!(is_group_offset(group_offset));
        assert!(!is_data_offset(group_offset));
        assert_eq!(extract_offset(group_offset), 0x1234);
        assert_eq!(group_offset, 0x1234);  // MSB=0 for group

        // Data has MSB set (1) - this is the ACTUAL file format
        let data_offset = make_data_offset(0x5678);
        assert!(is_data_offset(data_offset));
        assert!(!is_group_offset(data_offset));
        assert_eq!(extract_offset(data_offset), 0x5678);
        assert_eq!(data_offset, 0x8000000000005678);  // MSB=1 for data
    }

    #[test]
    fn test_empty_offset() {
        assert!(is_empty_offset(0));  // Empty group (MSB=0, offset=0)
        assert!(is_empty_offset(TYPE_FLAG_MASK)); // Empty data (MSB=1, offset=0)
        assert!(!is_empty_offset(0x100));
    }
}
