//! Ogawa writer helper utilities.
//!
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreAbstract/ArraySample.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreAbstract/Foundation.cpp`

use crate::core::{TimeSampling, TimeSamplingType};
use crate::util::PlainOldDataType;
use spooky_hash::SpookyHash;

use super::constants::ACYCLIC_TIME_PER_CYCLE;
use super::property::{OProperty, OPropertyData};

/// Write value with size hint (1, 2, or 4 bytes).
/// Matches `pushUint32WithHint` in C++.
pub(crate) fn write_with_hint(buf: &mut Vec<u8>, value: u32, hint: u8) {
    match hint {
        0 => buf.push(value as u8),
        1 => buf.extend_from_slice(&(value as u16).to_le_bytes()),
        _ => buf.extend_from_slice(&value.to_le_bytes()),
    }
}

/// Convert POD type to u8.
/// Matches the C++ enum order used in `WritePropertyInfo`.
pub(crate) fn pod_to_u8(pod: PlainOldDataType) -> u8 {
    match pod {
        PlainOldDataType::Boolean => 0,
        PlainOldDataType::Uint8 => 1,
        PlainOldDataType::Int8 => 2,
        PlainOldDataType::Uint16 => 3,
        PlainOldDataType::Int16 => 4,
        PlainOldDataType::Uint32 => 5,
        PlainOldDataType::Int32 => 6,
        PlainOldDataType::Uint64 => 7,
        PlainOldDataType::Int64 => 8,
        PlainOldDataType::Float16 => 9,
        PlainOldDataType::Float32 => 10,
        PlainOldDataType::Float64 => 11,
        PlainOldDataType::String => 12,
        PlainOldDataType::Wstring => 13,
        PlainOldDataType::Unknown => 0,
    }
}

/// Push f64 (chrono_t) to buffer as little-endian bytes.
pub(crate) fn push_chrono(buf: &mut Vec<u8>, value: f64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Get POD-size value used by MurmurHash3 on big-endian targets (matches `PODNumBytes`).
pub(crate) fn pod_seed(pod: PlainOldDataType) -> Option<u32> {
    let size = match pod {
        PlainOldDataType::Boolean => 1,
        PlainOldDataType::Uint8 => 1,
        PlainOldDataType::Int8 => 1,
        PlainOldDataType::Uint16 => 2,
        PlainOldDataType::Int16 => 2,
        PlainOldDataType::Uint32 => 4,
        PlainOldDataType::Int32 => 4,
        PlainOldDataType::Uint64 => 8,
        PlainOldDataType::Int64 => 8,
        PlainOldDataType::Float16 => 2,
        PlainOldDataType::Float32 => 4,
        PlainOldDataType::Float64 => 8,
        PlainOldDataType::String => 1,
        PlainOldDataType::Wstring => 4,
        PlainOldDataType::Unknown => return None,
    };
    Some(size)
}

/// Format Alembic version string matching `GetLibraryVersion()`.
pub(crate) fn format_alembic_version(version: i32) -> String {
    let major = version / 10000;
    let minor = (version / 100) % 100;
    let patch = version % 100;
    let date = option_env!("ALEMBIC_BUILD_DATE").unwrap_or("unknown");
    let time = option_env!("ALEMBIC_BUILD_TIME").unwrap_or("unknown");
    format!("Alembic {}.{}.{} (built {} {})", major, minor, patch, date, time)
}

/// Encode string samples to Alembic format (null-terminated).
/// For non-string PODs this returns the input bytes unchanged.
pub(crate) fn encode_sample_for_pod(data: &[u8], pod: PlainOldDataType) -> Vec<u8> {
    match pod {
        PlainOldDataType::String => {
            let mut out = data.to_vec();
            if out.last().copied() != Some(0) {
                out.push(0);
            }
            out
        }
        PlainOldDataType::Wstring => {
            let mut out = data.to_vec();
            let needs_term = out.len() < 4
                || out[out.len().saturating_sub(4)..].iter().any(|b| *b != 0);
            if needs_term {
                out.extend_from_slice(&[0, 0, 0, 0]);
            }
            out
        }
        _ => data.to_vec(),
    }
}

/// Encode a string array into Alembic payload (null-terminated per element).
pub(crate) fn encode_string_array(strings: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for s in strings {
        out.extend_from_slice(s.as_bytes());
        out.push(0);
    }
    out
}

/// Hash a property header matching C++ `HashPropertyHeader()`.
///
/// This builds the same data buffer as C++ and updates the hasher.
/// For non-compound properties, includes:
/// - name bytes
/// - metadata serialized bytes
/// - POD type (1 byte)
/// - extent (1 byte)
/// - 0 if scalar (1 byte) - arrays don't get this byte
/// - timePerCycle (8 bytes)
/// - samplesPerCycle count (4 bytes)
/// - each stored time (8 bytes)
pub(crate) fn hash_property_header(
    hasher: &mut SpookyHash,
    prop: &OProperty,
    time_sampling: &TimeSampling,
) {
    let mut data = Vec::new();

    // Name
    data.extend_from_slice(prop.name.as_bytes());

    // Metadata
    let meta = prop.meta_data.serialize();
    data.extend_from_slice(meta.as_bytes());

    // For non-compound properties
    if !matches!(prop.data, OPropertyData::Compound(_)) {
        // POD type
        data.push(pod_to_u8(prop.data_type.pod));
        // Extent
        data.push(prop.data_type.extent);

        // Scalar marker (only for scalars)
        if matches!(prop.data, OPropertyData::Scalar(_)) {
            data.push(0);
        }

        // Time per cycle
        let (tpc, times) = match &time_sampling.sampling_type {
            TimeSamplingType::Identity => (1.0, vec![0.0]),
            TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                (*time_per_cycle, vec![*start_time])
            }
            TimeSamplingType::Cyclic { time_per_cycle, times } => {
                (*time_per_cycle, times.clone())
            }
            TimeSamplingType::Acyclic { times } => (ACYCLIC_TIME_PER_CYCLE, times.clone()),
        };
        push_chrono(&mut data, tpc);

        // Samples per cycle count (4 bytes)
        let spc = times.len() as u32;
        data.extend_from_slice(&spc.to_le_bytes());

        // Each stored time
        for t in times {
            push_chrono(&mut data, t);
        }
    }

    if !data.is_empty() {
        hasher.update(&data);
    }
}

/// Hash dimensions for array samples, matching C++ `HashDimensions()`.
pub(crate) fn hash_dimensions(dims: &[usize], digest: &mut (u64, u64)) {
    if dims.is_empty() {
        return;
    }

    let mut hasher = SpookyHash::new(0, 0);

    // Dimensions as u64 values
    let dims_bytes: Vec<u8> = dims.iter()
        .flat_map(|d| (*d as u64).to_le_bytes())
        .collect();
    hasher.update(&dims_bytes);

    // Update with existing digest
    let mut digest_bytes = Vec::with_capacity(16);
    digest_bytes.extend_from_slice(&digest.0.to_le_bytes());
    digest_bytes.extend_from_slice(&digest.1.to_le_bytes());
    hasher.update(&digest_bytes);

    *digest = hasher.finalize();
}
