//! Core layer - abstract traits and fundamental types.
//!
//! This module provides:
//! - [`TimeSampling`] - Time sampling for animated properties
//! - [`MetaData`] - Key-value metadata storage
//! - [`ObjectHeader`] / [`PropertyHeader`] - Headers for objects and properties
//! - Abstract traits for reading/writing archives, objects, properties
//! - [`SampleSelector`] - Sample selection by index or time

mod time_sampling;
mod metadata;
mod header;
mod traits;
mod sample;
mod cache;
mod compression;

pub use time_sampling::{TimeSampling, TimeSamplingType};
pub use metadata::MetaData;
pub use header::{ObjectHeader, PropertyHeader, PropertyType};
pub use traits::{
    // Archive traits
    ArchiveReader, ArchiveWriter,
    // Object traits
    ObjectReader, ObjectWriter,
    // Property traits
    PropertyReader, PropertyWriter,
    ScalarPropertyReader, ScalarPropertyWriter,
    ArrayPropertyReader, ArrayPropertyWriter,
    CompoundPropertyReader, CompoundPropertyWriter,
};
pub use sample::{SampleSelector, SampleInterp, GeometryScope, TopologyVariance};
pub use cache::{
    ReadArraySampleCache, ArraySampleKey, CachedSample,
    ArraySampleContentKey, SampleDigest, compute_digest,
};
pub use compression::{compress, decompress, is_compressed};
