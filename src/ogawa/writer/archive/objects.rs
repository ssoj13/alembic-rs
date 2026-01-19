//! Object write path (hierarchy + object headers).
//!
//! Mirrors `OwData`/`OwImpl` ordering and header hashing.

use super::types::ObjectHeadersContext;
use super::OArchive;
use super::super::object::OObject;
use super::super::write_util::write_with_hint;
use crate::ogawa::format::{make_data_offset, make_group_offset};
use crate::util::Result;
use spooky_hash::SpookyHash;

impl OArchive {
    /// Write an object and return (position, hash1, hash2).
    ///
    /// The hash order matches C++: children hashes -> data hashes -> metadata -> name.
    pub(super) fn write_object(&mut self, obj: &OObject, parent_path: &str) -> Result<(u64, u64, u64)> {
        let full_path = if parent_path == "/" {
            format!("/{}", obj.name)
        } else {
            format!("{}/{}", parent_path, obj.name)
        };

        let mut child_positions = Vec::new();
        let mut child_hashes: Vec<u64> = Vec::new();
        for child in &obj.children {
            let (child_pos, h1, h2) = self.write_object(child, &full_path)?;
            child_positions.push(child_pos);
            child_hashes.push(h1);
            child_hashes.push(h2);
        }

        let (child_hash1, child_hash2) = if child_hashes.is_empty() {
            (0u64, 0u64)
        } else {
            let child_hash_bytes: Vec<u8> = child_hashes.iter()
                .flat_map(|h| h.to_le_bytes())
                .collect();
            let mut hasher = SpookyHash::new(0, 0);
            hasher.update(&child_hash_bytes);
            hasher.finalize()
        };

        let obj_ctx = ObjectHeadersContext { children: &obj.children, child_hash1, child_hash2 };
        let (props_pos, data_hash1, data_hash2, headers_pos, _) =
            self.write_properties_with_object_headers(&obj.properties, Some(obj_ctx))?;

        let mut children = Vec::new();
        if Self::is_deferred_placeholder(props_pos) {
            children.push(props_pos);
        } else {
            children.push(make_group_offset(props_pos));
        }

        for pos in child_positions {
            if Self::is_deferred_placeholder(pos) {
                children.push(pos);
            } else {
                children.push(make_group_offset(pos));
            }
        }

        if headers_pos != 0 {
            children.push(make_data_offset(headers_pos));
        }

        let pos = if self.deferred_mode {
            self.add_deferred_group(children)
        } else {
            self.write_group(&children)?
        };

        let mut combined_hash = SpookyHash::new(0, 0);
        if !child_hashes.is_empty() {
            let child_hash_bytes: Vec<u8> = child_hashes.iter()
                .flat_map(|h| h.to_le_bytes())
                .collect();
            combined_hash.update(&child_hash_bytes);
        }
        combined_hash.update(&data_hash1.to_le_bytes());
        combined_hash.update(&data_hash2.to_le_bytes());

        let meta_str = obj.meta_data.serialize();
        if !meta_str.is_empty() {
            combined_hash.update(meta_str.as_bytes());
        }

        combined_hash.update(obj.name.as_bytes());
        let (final_h1, final_h2) = combined_hash.finalize();

        Ok((pos, final_h1, final_h2))
    }

    /// Serialize object headers for children with 32-byte SpookyHash suffix.
    ///
    /// Matches `WriteObjectHeader` + object hash suffix in C++.
    pub(super) fn serialize_object_headers_with_hash(
        &mut self,
        children: &[OObject],
        data_hash1: u64,
        data_hash2: u64,
        child_hash1: u64,
        child_hash2: u64,
    ) -> Vec<u8> {
        let mut buf = Vec::new();

        for child in children {
            let name_bytes = child.name.as_bytes();
            write_with_hint(&mut buf, name_bytes.len() as u32, 2);
            buf.extend_from_slice(name_bytes);

            let meta_idx = self.add_indexed_metadata(&child.meta_data);
            write_with_hint(&mut buf, meta_idx as u32, 0);

            if meta_idx == 0xff {
                let meta_str = child.meta_data.serialize();
                write_with_hint(&mut buf, meta_str.len() as u32, 2);
                buf.extend_from_slice(meta_str.as_bytes());
            }
        }

        buf.extend_from_slice(&data_hash1.to_le_bytes());
        buf.extend_from_slice(&data_hash2.to_le_bytes());
        buf.extend_from_slice(&child_hash1.to_le_bytes());
        buf.extend_from_slice(&child_hash2.to_le_bytes());

        buf
    }
}
