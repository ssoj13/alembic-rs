//! Property writing and header serialization.
//!
//! Mirrors `CpwData`, `SpwImpl`, and `ApwImpl` ordering and sampling rules.

use super::types::PropertySampleState;
use super::types::ObjectHeadersContext;
use super::OArchive;
use super::super::property::{OProperty, OPropertyData};
use super::super::write_util::{
    encode_sample_for_pod, hash_dimensions, hash_property_header, pod_seed, pod_to_u8,
    write_with_hint,
};
use crate::core::{ArraySampleContentKey, TimeSampling};
use crate::ogawa::format::{make_data_offset, make_group_offset, EMPTY_DATA};
use crate::util::{PlainOldDataType, Result};
use spooky_hash::SpookyHash;

impl OArchive {
    /// Write properties and return (pos, hash1, hash2, raw_hashes).
    pub(super) fn write_properties_with_data(
        &mut self,
        props: &[OProperty],
    ) -> Result<(u64, u64, u64, Vec<u64>)> {
        let (pos, h1, h2, _, raw_hashes) = self.write_properties_with_object_headers(props, None)?;
        Ok((pos, h1, h2, raw_hashes))
    }

    /// Write properties and optionally emit object headers using data hash.
    pub(super) fn write_properties_with_object_headers(
        &mut self,
        props: &[OProperty],
        obj_ctx: Option<ObjectHeadersContext<'_>>,
    ) -> Result<(u64, u64, u64, u64, Vec<u64>)> {
        if props.is_empty() {
            let hasher = SpookyHash::new(0, 0);
            let (h1, h2) = hasher.finalize();

            let obj_headers_pos = if let Some(ctx) = obj_ctx {
                let obj_headers = self.serialize_object_headers_with_hash(
                    ctx.children,
                    h1,
                    h2,
                    ctx.child_hash1,
                    ctx.child_hash2,
                );
                self.write_data(&obj_headers)?
            } else {
                0
            };

            let pos = self.write_group(&[])?;
            return Ok((pos, h1, h2, obj_headers_pos, Vec::new()));
        }

        let mut sorted_indices: Vec<usize> = (0..props.len()).collect();
        sorted_indices.sort_by_key(|&i| (props[i].data_write_order, i));

        let mut prop_states: Vec<PropertySampleState> =
            (0..props.len()).map(|_| PropertySampleState::default()).collect();
        for &idx in &sorted_indices {
            let state = self.collect_property_sample_data(&props[idx])?;
            prop_states[idx] = state;
        }

        let mut prop_positions = vec![0u64; props.len()];
        let mut prop_hashes_pairs = vec![(0u64, 0u64); props.len()];
        let mut header_states: Vec<PropertySampleState> =
            (0..props.len()).map(|_| PropertySampleState::default()).collect();

        for idx in (0..props.len()).rev() {
            let state = std::mem::take(&mut prop_states[idx]);
            let (pos, h1, h2) = self.finalize_property_group(&props[idx], &state)?;
            prop_positions[idx] = pos;
            prop_hashes_pairs[idx] = (h1, h2);
            header_states[idx] = PropertySampleState {
                first_changed_index: state.first_changed_index,
                last_changed_index: state.last_changed_index,
                is_homogenous: state.is_homogenous,
                num_samples: state.num_samples,
                ..PropertySampleState::default()
            };
        }

        let mut prop_hashes: Vec<u64> = Vec::new();
        for (h1, h2) in &prop_hashes_pairs {
            prop_hashes.push(*h1);
            prop_hashes.push(*h2);
        }

        let (data_h1, data_h2) = {
            let hash_bytes: Vec<u8> = prop_hashes.iter().flat_map(|h| h.to_le_bytes()).collect();
            let mut hasher = SpookyHash::new(0, 0);
            hasher.update(&hash_bytes);
            hasher.finalize()
        };

        let obj_headers_pos = if let Some(ctx) = obj_ctx {
            let obj_headers = self.serialize_object_headers_with_hash(
                ctx.children,
                data_h1,
                data_h2,
                ctx.child_hash1,
                ctx.child_hash2,
            );
            self.write_data(&obj_headers)?
        } else {
            0
        };

        let headers_data = self.serialize_property_headers(props, &header_states);
        let headers_pos = self.write_data(&headers_data)?;

        let mut children = Vec::new();
        for pos in prop_positions {
            if Self::is_deferred_placeholder(pos) {
                children.push(pos);
            } else {
                children.push(make_group_offset(pos));
            }
        }
        children.push(make_data_offset(headers_pos));

        // Property groups are written in reverse creation order to mirror C++ destructor
        // finalization (see AbcCoreOgawa CpwImpl/SpwImpl/ApwImpl ownership model).
        let pos = self.write_group(&children)?;

        Ok((pos, data_h1, data_h2, obj_headers_pos, prop_hashes))
    }

    /// Collect sample data for a property and return the state for header writing.
    pub(super) fn collect_property_sample_data(&mut self, prop: &OProperty) -> Result<PropertySampleState> {
        let ts_idx = prop.time_sampling_index;

        match &prop.data {
            OPropertyData::Scalar(samples) => {
                let num_samples = samples.len() as u32;

                let mut state = PropertySampleState {
                    num_samples,
                    ..PropertySampleState::default()
                };
                let mut prev_key: Option<ArraySampleContentKey> = None;
                let mut prev_data_pos: Option<u64> = None;

                for (index, sample) in samples.iter().enumerate() {
                    let sample_index = index as u32;
                    let (digest, content_key) = if let Some(d) = &sample.digest {
                        let pod_tag = match prop.data_type.pod {
                            PlainOldDataType::String | PlainOldDataType::Wstring => {
                                pod_to_u8(prop.data_type.pod)
                            }
                            _ => pod_to_u8(PlainOldDataType::Int8),
                        };
                        (*d, ArraySampleContentKey::from_digest(*d, sample.data.len(), pod_tag))
                    } else {
                        let encoded = encode_sample_for_pod(&sample.data, prop.data_type.pod);
                        let pod_size = pod_seed(prop.data_type.pod);
                        let pod_tag = match prop.data_type.pod {
                            PlainOldDataType::String | PlainOldDataType::Wstring => {
                                pod_to_u8(prop.data_type.pod)
                            }
                            _ => pod_to_u8(PlainOldDataType::Int8),
                        };
                        let content_key =
                            ArraySampleContentKey::from_data(&encoded, None, pod_size, pod_tag);
                        (*content_key.digest(), content_key)
                    };

                    let d0 = u64::from_le_bytes(digest[0..8].try_into().unwrap());
                    let d1 = u64::from_le_bytes(digest[8..16].try_into().unwrap());
                    state.sample_hash = match state.sample_hash {
                        None => Some((d0, d1)),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d0, d1)),
                    };

                    let key_changed = prev_key.as_ref().map_or(true, |k| *k != content_key);
                    if sample_index == 0 || key_changed {
                        if sample_index > 0 && state.first_changed_index != 0 {
                            for _ in (state.last_changed_index + 1)..sample_index {
                                if let Some(pos) = prev_data_pos {
                                    state.children.push(make_data_offset(pos));
                                }
                            }
                        }

                        let pos = if sample.digest.is_some() {
                            self.write_keyed_data_with_key(&sample.data, &digest, prop.data_type.pod)?
                        } else {
                            self.write_keyed_data(&sample.data, prop.data_type.pod)?
                        };
                        prev_data_pos = Some(pos);
                        prev_key = Some(content_key);
                        state.children.push(make_data_offset(pos));

                        if sample_index != 0 {
                            if state.first_changed_index == 0 {
                                state.first_changed_index = sample_index;
                            }
                            state.last_changed_index = sample_index;
                        }
                    }
                }

                let max_samples = if state.last_changed_index == 0 && state.num_samples > 0 {
                    1
                } else {
                    state.num_samples
                };
                self.update_max_samples(ts_idx, max_samples);

                Ok(state)
            }
            OPropertyData::Array(samples) => {
                let num_samples = samples.len() as u32;

                let mut state = PropertySampleState {
                    num_samples,
                    ..PropertySampleState::default()
                };
                let mut prev_key: Option<ArraySampleContentKey> = None;
                let mut prev_data_pos: Option<u64> = None;
                let mut prev_dims_pos: Option<u64> = None;
                let mut prev_num_points: Option<usize> = None;
                let mut prev_dims: Option<Vec<usize>> = None;

                for (index, sample) in samples.iter().enumerate() {
                    let sample_index = index as u32;
                    let (digest, content_key) = if let Some(d) = &sample.digest {
                        let pod_tag = match prop.data_type.pod {
                            PlainOldDataType::String | PlainOldDataType::Wstring => {
                                pod_to_u8(prop.data_type.pod)
                            }
                            _ => pod_to_u8(PlainOldDataType::Int8),
                        };
                        (*d, ArraySampleContentKey::from_digest(*d, sample.data.len(), pod_tag))
                    } else {
                        let encoded = encode_sample_for_pod(&sample.data, prop.data_type.pod);
                        let pod_size = pod_seed(prop.data_type.pod);
                        let pod_tag = match prop.data_type.pod {
                            PlainOldDataType::String | PlainOldDataType::Wstring => {
                                pod_to_u8(prop.data_type.pod)
                            }
                            _ => pod_to_u8(PlainOldDataType::Int8),
                        };
                        let content_key =
                            ArraySampleContentKey::from_data(&encoded, None, pod_size, pod_tag);
                        (*content_key.digest(), content_key)
                    };

                    let key_changed = prev_key.as_ref().map_or(true, |k| *k != content_key);
                    let dims_for_hash = if key_changed {
                        &sample.dims
                    } else {
                        prev_dims.as_ref().unwrap_or(&sample.dims)
                    };

                    let mut d = (
                        u64::from_le_bytes(digest[0..8].try_into().unwrap()),
                        u64::from_le_bytes(digest[8..16].try_into().unwrap()),
                    );
                    hash_dimensions(dims_for_hash, &mut d);
                    state.sample_hash = match state.sample_hash {
                        None => Some(d),
                        Some((h0, h1)) => Some(SpookyHash::short_end_mix(h0, h1, d.0, d.1)),
                    };

                    if sample_index == 0 || key_changed {
                        if sample_index > 0 && state.first_changed_index != 0 {
                            for _ in (state.last_changed_index + 1)..sample_index {
                                if let (Some(pos), Some(dim_pos)) = (prev_data_pos, prev_dims_pos) {
                                    state.children.push(make_data_offset(pos));
                                    state.children.push(dim_pos);
                                }
                            }
                        }

                        let data_pos = if sample.digest.is_some() {
                            self.write_keyed_data_with_key(&sample.data, &digest, prop.data_type.pod)?
                        } else {
                            self.write_keyed_data(&sample.data, prop.data_type.pod)?
                        };

                        let dims_offset = if sample.dims.len() <= 1
                            && !matches!(
                                prop.data_type.pod,
                                PlainOldDataType::String | PlainOldDataType::Wstring
                            )
                        {
                            EMPTY_DATA
                        } else {
                            let dims_data: Vec<u8> = sample
                                .dims
                                .iter()
                                .flat_map(|dim| (*dim as u64).to_le_bytes())
                                .collect();
                            make_data_offset(self.write_data(&dims_data)?)
                        };

                        let num_points = sample.dims.iter().product::<usize>()
                            * prop.data_type.extent as usize;
                        if prop.data_type.extent != 1 {
                            state.is_homogenous = false;
                        } else if let Some(prev) = prev_num_points {
                            if num_points != prev {
                                state.is_homogenous = false;
                            }
                        }
                        prev_num_points = Some(num_points);
                        prev_dims = Some(sample.dims.clone());
                        prev_dims_pos = Some(dims_offset);
                        prev_data_pos = Some(data_pos);
                        prev_key = Some(content_key);

                        state.children.push(make_data_offset(data_pos));
                        state.children.push(dims_offset);

                        if sample_index != 0 {
                            if state.first_changed_index == 0 {
                                state.first_changed_index = sample_index;
                            }
                            state.last_changed_index = sample_index;
                        }
                    }
                }

                let max_samples = if state.last_changed_index == 0 && state.num_samples > 0 {
                    1
                } else {
                    state.num_samples
                };
                self.update_max_samples(ts_idx, max_samples);

                Ok(state)
            }
            OPropertyData::Compound(_) => Ok(PropertySampleState::default()),
        }
    }

    /// Finalize property group and return (pos, hash1, hash2).
    pub(super) fn finalize_property_group(
        &mut self,
        prop: &OProperty,
        state: &PropertySampleState,
    ) -> Result<(u64, u64, u64)> {
        let ts_idx = prop.time_sampling_index;
        let time_sampling = self.time_samplings.get(ts_idx as usize)
            .cloned()
            .unwrap_or_else(TimeSampling::identity);

        match &prop.data {
            OPropertyData::Scalar(_) | OPropertyData::Array(_) => {
                let pos = self.write_group(&state.children)?;

                let mut hasher = SpookyHash::new(0, 0);
                hash_property_header(&mut hasher, prop, &time_sampling);

                if let Some((sh0, sh1)) = state.sample_hash {
                    let mut sample_bytes = Vec::with_capacity(16);
                    sample_bytes.extend_from_slice(&sh0.to_le_bytes());
                    sample_bytes.extend_from_slice(&sh1.to_le_bytes());
                    hasher.update(&sample_bytes);
                }

                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
            OPropertyData::Compound(sub_props) => {
                let (pos, _, _, raw_prop_hashes) = self.write_properties_with_data(sub_props)?;

                let mut hasher = SpookyHash::new(0, 0);
                let hash_bytes: Vec<u8> = raw_prop_hashes.iter().flat_map(|h| h.to_le_bytes()).collect();
                hasher.update(&hash_bytes);
                hash_property_header(&mut hasher, prop, &time_sampling);

                let (h1, h2) = hasher.finalize();
                Ok((pos, h1, h2))
            }
        }
    }

    /// Serialize property headers, matching `WritePropertyInfo` layout.
    pub(super) fn serialize_property_headers(
        &mut self,
        props: &[OProperty],
        states: &[PropertySampleState],
    ) -> Vec<u8> {
        let mut buf = Vec::new();

        for (idx, prop) in props.iter().enumerate() {
            let state = states.get(idx);
            let info = self.build_property_info(prop, state);
            buf.extend_from_slice(&info.to_le_bytes());

            let size_hint = ((info >> 2) & 0x03) as u8;

            if !matches!(prop.data, OPropertyData::Compound(_)) {
                let num_samples = state.map(|s| s.num_samples).unwrap_or(prop.getNumSamples() as u32);
                write_with_hint(&mut buf, num_samples, size_hint);

                if (info & 0x0200) != 0 {
                    let first = state.map(|s| s.first_changed_index).unwrap_or(prop.first_changed_index);
                    let last = state.map(|s| s.last_changed_index).unwrap_or(prop.last_changed_index);
                    write_with_hint(&mut buf, first, size_hint);
                    write_with_hint(&mut buf, last, size_hint);
                }

                if (info & 0x0100) != 0 {
                    write_with_hint(&mut buf, prop.time_sampling_index, size_hint);
                }
            }

            let name_bytes = prop.name.as_bytes();
            write_with_hint(&mut buf, name_bytes.len() as u32, size_hint);
            buf.extend_from_slice(name_bytes);

            let meta_idx = self.add_indexed_metadata(&prop.meta_data);
            if meta_idx == 0xff {
                let meta_str = prop.meta_data.serialize();
                write_with_hint(&mut buf, meta_str.len() as u32, size_hint);
                buf.extend_from_slice(meta_str.as_bytes());
            }
        }

        buf
    }

    /// Build the info word for a property header (see `WritePropertyInfo`).
    pub(super) fn build_property_info(
        &mut self,
        prop: &OProperty,
        state: Option<&PropertySampleState>,
    ) -> u32 {
        let mut info: u32 = 0;

        let name_size = prop.name.len() as u32;
        let meta_data_size = prop.meta_data.serialize().len() as u32;
        let num_samples = state.map(|s| s.num_samples).unwrap_or(prop.getNumSamples() as u32);
        let time_sampling_index = prop.time_sampling_index;

        let max_size = meta_data_size.max(name_size).max(num_samples).max(time_sampling_index);
        let size_hint = if max_size > 255 && max_size < 65536 {
            1
        } else if max_size >= 65536 {
            2
        } else {
            0
        };

        info |= (size_hint & 0x03) << 2;

        match &prop.data {
            OPropertyData::Compound(_) => {
                info |= 0;
            }
            OPropertyData::Scalar(_) => {
                info |= 1;
            }
            OPropertyData::Array(_) => {
                if prop.is_scalar_like {
                    info |= 3;
                } else {
                    info |= 2;
                }
            }
        }

        if !matches!(prop.data, OPropertyData::Compound(_)) {
            let pod = pod_to_u8(prop.data_type.pod) as u32;
            info |= (pod & 0x0f) << 4;

            info |= (prop.data_type.extent as u32 & 0xff) << 12;

            let is_homogenous = match &prop.data {
                OPropertyData::Array(_) => state.map(|s| s.is_homogenous).unwrap_or(true),
                _ => true,
            };
            if is_homogenous {
                info |= 0x400;
            }

            if prop.time_sampling_index != 0 {
                info |= 0x0100;
            }

            let first_changed = state.map(|s| s.first_changed_index).unwrap_or(prop.first_changed_index);
            let last_changed = state.map(|s| s.last_changed_index).unwrap_or(prop.last_changed_index);
            if first_changed == 0 && last_changed == 0 {
                info |= 0x800;
            } else if first_changed != 1 || last_changed != num_samples.saturating_sub(1) {
                info |= 0x0200;
            }
        }

        let meta_idx = self.add_indexed_metadata(&prop.meta_data);
        info |= (meta_idx as u32) << 20;

        info
    }
}
