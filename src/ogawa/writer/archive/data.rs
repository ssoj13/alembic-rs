//! Low-level data/group writing for Ogawa archives.
//!
//! Mirrors `WriteData`/`CopyWrittenData` behavior from C++.

use super::OArchive;
use super::super::constants::DATA_KEY_SIZE;
use super::super::write_util::{encode_sample_for_pod, pod_seed, pod_to_u8};
use crate::core::ArraySampleContentKey;
use crate::util::{Error, PlainOldDataType, Result};

impl OArchive {
    /// Write raw data block and return its position.
    ///
    /// C++: `Ogawa::OGroup::addData` with size prefix stored by the stream.
    pub fn write_data(&mut self, data: &[u8]) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        if data.is_empty() {
            return Ok(0);
        }

        let pos = self.stream.pos();
        self.stream.write_u64(data.len() as u64)?;
        self.stream.write_bytes(data)?;
        Ok(pos)
    }

    /// Write data with 16-byte key prefix and deduplication.
    ///
    /// C++: `WriteData` with `WrittenSampleMap` lookup.
    pub fn write_keyed_data(&mut self, data: &[u8], pod: PlainOldDataType) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        let encoded = encode_sample_for_pod(data, pod);
        if encoded.is_empty() {
            return Ok(0);
        }
        let pod_size = pod_seed(pod);
        let pod_tag = match pod {
            PlainOldDataType::String | PlainOldDataType::Wstring => pod_to_u8(pod),
            _ => pod_to_u8(PlainOldDataType::Int8),
        };
        let content_key = ArraySampleContentKey::from_data(&encoded, None, pod_size, pod_tag);

        if self.dedup_enabled {
            if let Some(&existing_pos) = self.dedup_map.get(&content_key) {
                return Ok(existing_pos);
            }
        }

        let pos = self.stream.pos();
        let total_size = DATA_KEY_SIZE + encoded.len();
        self.stream.write_u64(total_size as u64)?;
        self.stream.write_bytes(content_key.digest())?;
        self.stream.write_bytes(&encoded)?;

        if self.dedup_enabled {
            self.dedup_map.insert(content_key, pos);
        }

        Ok(pos)
    }

    /// Write data with specific key (for known digest).
    ///
    /// Used for raw copy paths where the digest is already known.
    pub fn write_keyed_data_with_key(
        &mut self,
        data: &[u8],
        key: &[u8; 16],
        pod: PlainOldDataType,
    ) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        if data.is_empty() {
            return Ok(0);
        }

        let pod_tag = match pod {
            PlainOldDataType::String | PlainOldDataType::Wstring => pod_to_u8(pod),
            _ => pod_to_u8(PlainOldDataType::Int8),
        };
        let content_key = ArraySampleContentKey::from_digest(*key, data.len(), pod_tag);
        if self.dedup_enabled {
            if let Some(&existing_pos) = self.dedup_map.get(&content_key) {
                return Ok(existing_pos);
            }
        }

        let pos = self.stream.pos();
        let total_size = DATA_KEY_SIZE + data.len();
        self.stream.write_u64(total_size as u64)?;
        self.stream.write_bytes(key)?;
        self.stream.write_bytes(data)?;

        if self.dedup_enabled {
            self.dedup_map.insert(content_key, pos);
        }
        Ok(pos)
    }

    /// Write a group and return its position.
    ///
    /// C++: `Ogawa::OGroup::addGroup` stored inline with size prefix.
    pub fn write_group(&mut self, children: &[u64]) -> Result<u64> {
        if self.frozen {
            return Err(Error::Frozen);
        }

        if children.is_empty() {
            return Ok(0);
        }

        let pos = self.stream.pos();
        self.stream.write_u64(children.len() as u64)?;
        for &child in children {
            self.stream.write_u64(child)?;
        }
        Ok(pos)
    }

}
