//! Ogawa writer constants.
//!
//! These mirror the C++ Alembic Ogawa writer values.
//! References:
//! - `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp`
//! - `_ref/alembic/lib/Alembic/AbcCoreAbstract/Foundation.h`

/// Library version for written archives (e.g. 1.8.10 => 10810).
/// Matches `ALEMBIC_LIBRARY_VERSION` in C++.
pub(crate) const ALEMBIC_LIBRARY_VERSION: i32 = 10810;

/// Ogawa file format version (`ALEMBIC_OGAWA_FILE_VERSION = 0`).
pub(crate) const OGAWA_FILE_VERSION: i32 = 0;

/// Acyclic time per cycle marker (chrono_t max / 32.0).
/// Matches `TimeSamplingType::kAcyclic` encoding in C++.
pub(crate) const ACYCLIC_TIME_PER_CYCLE: f64 = f64::MAX / 32.0;

/// Size of digest/key prefix in data blocks.
/// Matches the 128-bit MurmurHash3 key in C++ `ArraySample::Key`.
pub(crate) const DATA_KEY_SIZE: usize = 16;
