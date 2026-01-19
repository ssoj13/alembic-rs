//! Deferred group writing (bottom-up).
//!
//! Mirrors the C++ Ogawa group freeze behavior used during destruction.

use super::types::DeferredGroup;
use super::OArchive;
use crate::ogawa::format::{is_data_offset, make_group_offset};
use crate::util::{Error, Result};

impl OArchive {
    /// Marker constant for deferred group placeholders.
    const DEFERRED_GROUP_MARKER: u64 = 0x4000_0000_0000_0000; // Bit 62 set.

    /// Add a deferred group and return a placeholder reference.
    pub(super) fn add_deferred_group(&mut self, children: Vec<u64>) -> u64 {
        if children.is_empty() {
            return 0;
        }
        let idx = self.deferred_groups.len();
        self.deferred_groups.push(DeferredGroup::new(children));
        Self::DEFERRED_GROUP_MARKER | (idx as u64)
    }

    /// Check if a value is a deferred group placeholder.
    #[inline]
    pub(super) fn is_deferred_placeholder(value: u64) -> bool {
        (value & Self::DEFERRED_GROUP_MARKER) != 0 && !is_data_offset(value)
    }

    /// Extract deferred group index from placeholder.
    #[inline]
    pub(super) fn deferred_group_index(placeholder: u64) -> usize {
        (placeholder & !Self::DEFERRED_GROUP_MARKER) as usize
    }

    /// Flush all deferred groups, writing them in bottom-up order.
    pub(super) fn flush_deferred_groups(&mut self) -> Result<u64> {
        if self.deferred_groups.is_empty() {
            return Ok(0);
        }

        let mut group_deps: Vec<Vec<usize>> = vec![Vec::new(); self.deferred_groups.len()];

        for (i, group) in self.deferred_groups.iter().enumerate() {
            for &child in &group.children {
                if Self::is_deferred_placeholder(child) {
                    let child_idx = Self::deferred_group_index(child);
                    group_deps[i].push(child_idx);
                }
            }
        }

        let mut written: Vec<bool> = vec![false; self.deferred_groups.len()];
        let mut order: Vec<usize> = Vec::with_capacity(self.deferred_groups.len());

        while order.len() < self.deferred_groups.len() {
            let mut found = false;
            for i in 0..self.deferred_groups.len() {
                if written[i] {
                    continue;
                }
                let deps_ok = group_deps[i].iter().all(|&d| written[d]);
                if deps_ok {
                    order.push(i);
                    written[i] = true;
                    found = true;
                }
            }
            if !found {
                return Err(Error::invalid("Circular dependency in deferred groups"));
            }
        }

        let mut last_pos = 0u64;
        for &idx in &order {
            let mut resolved_children = Vec::new();
            for &child in &self.deferred_groups[idx].children {
                if Self::is_deferred_placeholder(child) {
                    let child_idx = Self::deferred_group_index(child);
                    let child_pos = self.deferred_groups[child_idx].final_pos
                        .ok_or_else(|| Error::invalid("Deferred group not yet written"))?;
                    resolved_children.push(make_group_offset(child_pos));
                } else {
                    resolved_children.push(child);
                }
            }

            let pos = self.stream.pos();
            self.stream.write_u64(resolved_children.len() as u64)?;
            for &child in &resolved_children {
                self.stream.write_u64(child)?;
            }

            self.deferred_groups[idx].final_pos = Some(pos);
            last_pos = pos;
        }

        Ok(last_pos)
    }
}
