//! Shared types for archive writing.
//!
//! These mirror C++ helper structs in `Foundation.h` and write paths.

use super::super::object::OObject;

/// Deferred group for bottom-up writing.
/// Matches C++ OGroup freeze behavior.
#[derive(Debug)]
pub(super) struct DeferredGroup {
    /// Children of this group (data positions have MSB set, group indices don't).
    pub(super) children: Vec<u64>,
    /// Final position after writing (set during flush).
    pub(super) final_pos: Option<u64>,
}

impl DeferredGroup {
    pub(super) fn new(children: Vec<u64>) -> Self {
        Self { children, final_pos: None }
    }
}

/// Context for computing object headers inside write_properties.
///
/// Object headers depend on data_hash which is computed during property writing,
/// so we carry child hashes alongside the object list.
pub(super) struct ObjectHeadersContext<'a> {
    pub(super) children: &'a [OObject],
    pub(super) child_hash1: u64,
    pub(super) child_hash2: u64,
}

/// Accumulated property sample state.
///
/// Mirrors the sampling tracking in C++ `PropertyHeaderAndFriends`.
pub(super) struct PropertySampleState {
    pub(super) children: Vec<u64>,
    pub(super) sample_hash: Option<(u64, u64)>,
    pub(super) first_changed_index: u32,
    pub(super) last_changed_index: u32,
    pub(super) is_homogenous: bool,
    pub(super) num_samples: u32,
}

impl Default for PropertySampleState {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            sample_hash: None,
            first_changed_index: 0,
            last_changed_index: 0,
            is_homogenous: true,
            num_samples: 0,
        }
    }
}
