//! Metadata and time-sampling serialization.
//!
//! Mirrors `MetaDataMap` and time sampling table layout in C++.

use super::OArchive;
use crate::core::{MetaData, TimeSamplingType};

impl OArchive {
    /// Add or get indexed metadata, returns index.
    ///
    /// Matches `MetaDataMap::getIndex` behavior.
    pub fn add_indexed_metadata(&mut self, md: &MetaData) -> u8 {
        let serialized = md.serialize();

        if serialized.is_empty() {
            return 0;
        }

        if let Some(&idx) = self.metadata_map.get(&serialized) {
            return idx as u8;
        }

        // max 254 entries + empty (index 0) => len < 255
        if self.indexed_metadata.len() >= 255 || serialized.len() > 255 {
            return 0xff;
        }

        let idx = self.indexed_metadata.len();
        self.indexed_metadata.push(md.clone());
        self.metadata_map.insert(serialized, idx);
        idx as u8
    }

    /// Set indexed metadata from source archive (for copying).
    pub fn set_indexed_metadata(&mut self, metadata: &[MetaData]) {
        self.indexed_metadata.clear();
        self.metadata_map.clear();

        self.indexed_metadata.push(MetaData::new());

        for (i, md) in metadata.iter().enumerate().skip(1) {
            self.indexed_metadata.push(md.clone());
            let serialized = md.serialize();
            if !serialized.is_empty() {
                self.metadata_map.insert(serialized, i);
            }
        }
    }

    /// Serialize time samplings and max samples table.
    ///
    /// Matches `AwImpl::writeTimeSamples` layout.
    pub(super) fn serialize_time_samplings(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        for (i, ts) in self.time_samplings.iter().enumerate() {
            let max_sample = self.max_samples.get(i).copied().unwrap_or(0);
            buf.extend_from_slice(&max_sample.to_le_bytes());

            let (tpc, samples): (f64, Vec<f64>) = match &ts.sampling_type {
                TimeSamplingType::Identity => (1.0, vec![0.0]),
                TimeSamplingType::Uniform { time_per_cycle, start_time } => {
                    (*time_per_cycle, vec![*start_time])
                }
                TimeSamplingType::Cyclic { time_per_cycle, times } => {
                    (*time_per_cycle, times.clone())
                }
                TimeSamplingType::Acyclic { times } => {
                    (super::super::constants::ACYCLIC_TIME_PER_CYCLE, times.clone())
                }
            };

            buf.extend_from_slice(&tpc.to_le_bytes());
            buf.extend_from_slice(&(samples.len() as u32).to_le_bytes());
            for sample in samples {
                buf.extend_from_slice(&sample.to_le_bytes());
            }
        }

        buf
    }

    /// Serialize indexed metadata table.
    ///
    /// Matches `MetaDataMap::write` layout.
    pub(super) fn serialize_indexed_metadata(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for md in self.indexed_metadata.iter().skip(1) {
            let serialized = md.serialize();
            buf.push(serialized.len() as u8);
            buf.extend_from_slice(serialized.as_bytes());
        }
        buf
    }
}
