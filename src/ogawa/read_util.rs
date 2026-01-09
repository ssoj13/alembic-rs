//! Alembic data reading utilities.
//!
//! This module implements parsing of Alembic-specific data structures
//! from raw Ogawa data blocks.

use crate::core::{MetaData, TimeSampling};
use crate::util::{DataType, PlainOldDataType, Error, Result};
use super::IData;

/// Convert u8 to PlainOldDataType.
fn pod_from_u8(value: u8) -> Option<PlainOldDataType> {
    match value {
        0 => Some(PlainOldDataType::Boolean),
        1 => Some(PlainOldDataType::Uint8),
        2 => Some(PlainOldDataType::Int8),
        3 => Some(PlainOldDataType::Uint16),
        4 => Some(PlainOldDataType::Int16),
        5 => Some(PlainOldDataType::Uint32),
        6 => Some(PlainOldDataType::Int32),
        7 => Some(PlainOldDataType::Uint64),
        8 => Some(PlainOldDataType::Int64),
        9 => Some(PlainOldDataType::Float16),
        10 => Some(PlainOldDataType::Float32),
        11 => Some(PlainOldDataType::Float64),
        12 => Some(PlainOldDataType::String),
        13 => Some(PlainOldDataType::Wstring),
        _ => None,
    }
}

/// Alembic Ogawa file version constant.
pub const ALEMBIC_OGAWA_FILE_VERSION: i32 = 1;

/// Minimum supported Alembic version.
pub const MIN_ALEMBIC_VERSION: i32 = 9999;

// ============================================================================
// Time Sampling Parsing
// ============================================================================

/// Read time samplings and max sample counts from archive data.
pub fn read_time_samplings_and_max(
    data: &IData,
) -> Result<(Vec<TimeSampling>, Vec<u32>)> {
    let mut time_samples = Vec::new();
    let mut max_samples = Vec::new();
    
    if data.is_empty() {
        return Ok((time_samples, max_samples));
    }
    
    let buf = data.read_all()?;
    let buf_size = buf.len();
    let mut pos = 0;
    
    while pos < buf_size {
        // Read max_sample (u32), tpc (f64), num_samples (u32)
        if pos + 4 + 8 + 4 > buf_size {
            return Err(Error::invalid("TimeSamples info truncated"));
        }
        
        let max_sample = read_u32_le(&buf[pos..]);
        pos += 4;
        max_samples.push(max_sample);
        
        let tpc = read_f64_le(&buf[pos..]);
        pos += 8;
        
        let num_samples = read_u32_le(&buf[pos..]) as usize;
        pos += 4;
        
        if num_samples == 0 || pos + 8 * num_samples > buf_size {
            return Err(Error::invalid("TimeSamples sample times invalid"));
        }
        
        // Read sample times
        let mut sample_times = Vec::with_capacity(num_samples);
        for _ in 0..num_samples {
            sample_times.push(read_f64_le(&buf[pos..]));
            pos += 8;
        }
        
        // Determine sampling type
        // Acyclic time per cycle is a special value (very large negative)
        const ACYCLIC_TIME_PER_CYCLE: f64 = -f64::MAX;
        
        let ts = if (tpc - ACYCLIC_TIME_PER_CYCLE).abs() < f64::EPSILON {
            // Acyclic: explicit times for each sample
            TimeSampling::acyclic(sample_times)
        } else if num_samples == 1 {
            // Uniform: single stored time = start_time, tpc = time between samples
            let start_time = sample_times[0];
            TimeSampling::uniform(tpc, start_time)
        } else {
            // Cyclic: multiple times per cycle that repeat
            TimeSampling::cyclic(tpc, sample_times)
        };
        
        time_samples.push(ts);
    }
    
    Ok((time_samples, max_samples))
}

// ============================================================================
// Indexed Metadata Parsing
// ============================================================================

/// Read indexed metadata from archive data.
pub fn read_indexed_metadata(data: &IData) -> Result<Vec<MetaData>> {
    let mut metadata_vec = Vec::new();
    
    // First entry is always empty metadata
    metadata_vec.push(MetaData::new());
    
    if data.is_empty() {
        return Ok(metadata_vec);
    }
    
    // Indexed metadata is limited to 256 entries of max 256 bytes each
    if data.size() > 65536 {
        return Err(Error::invalid("Indexed MetaData buffer too large"));
    }
    
    let buf = data.read_all()?;
    let buf_size = buf.len();
    let mut pos = 0;
    
    while pos < buf_size {
        if pos + 1 > buf_size {
            return Err(Error::invalid("Indexed MetaData size truncated"));
        }
        
        let metadata_size = buf[pos] as usize;
        pos += 1;
        
        if pos + metadata_size > buf_size {
            return Err(Error::invalid("Indexed MetaData string truncated"));
        }
        
        if metadata_size == 0 {
            metadata_vec.push(MetaData::new());
        } else {
            let metadata_str = std::str::from_utf8(&buf[pos..pos + metadata_size])
                .map_err(|e| Error::other(format!("Invalid UTF-8 in metadata: {}", e)))?;
            pos += metadata_size;
            
            let md = MetaData::parse(metadata_str);
            metadata_vec.push(md);
        }
    }
    
    Ok(metadata_vec)
}

// ============================================================================
// Object Header Parsing
// ============================================================================

/// Parsed object header with all fields.
#[derive(Debug, Clone)]
pub struct ParsedObjectHeader {
    pub name: String,
    pub full_name: String,
    pub metadata: MetaData,
}

/// Read object headers from a data block.
pub fn read_object_headers(
    data: &IData,
    parent_name: &str,
    indexed_metadata: &[MetaData],
) -> Result<Vec<ParsedObjectHeader>> {
    let mut headers = Vec::new();
    
    // Skip if data is too small (need at least 32 bytes for hashes)
    if data.size() <= 32 {
        return Ok(headers);
    }
    
    // Read all data except the last 32 bytes (hashes)
    let total_size = data.size() as usize;
    let data_size = total_size - 32;
    
    let buf = data.read_all()?;
    let buf = &buf[..data_size];
    let buf_size = buf.len();
    let mut pos = 0;
    
    while pos < buf_size {
        if pos + 4 > buf_size {
            return Err(Error::invalid("Object header name size truncated"));
        }
        
        let name_size = read_u32_le(&buf[pos..]) as usize;
        pos += 4;
        
        if name_size == 0 || pos + name_size + 1 > buf_size {
            return Err(Error::invalid("Object header name invalid"));
        }
        
        let name = std::str::from_utf8(&buf[pos..pos + name_size])
            .map_err(|e| Error::other(format!("Invalid UTF-8 in object name: {}", e)))?
            .to_string();
        pos += name_size;
        
        let metadata_index = buf[pos] as usize;
        pos += 1;
        
        let full_name = if parent_name.is_empty() || parent_name == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", parent_name, name)
        };
        
        let metadata = if metadata_index == 0xff {
            // Inline metadata
            if pos + 4 > buf_size {
                return Err(Error::invalid("Object header metadata size truncated"));
            }
            
            let metadata_size = read_u32_le(&buf[pos..]) as usize;
            pos += 4;
            
            if pos + metadata_size > buf_size {
                return Err(Error::invalid("Object header metadata string truncated"));
            }
            
            let metadata_str = std::str::from_utf8(&buf[pos..pos + metadata_size])
                .map_err(|e| Error::other(format!("Invalid UTF-8 in metadata: {}", e)))?;
            pos += metadata_size;
            
            MetaData::parse(metadata_str)
        } else if metadata_index < indexed_metadata.len() {
            indexed_metadata[metadata_index].clone()
        } else {
            return Err(Error::invalid(format!("Invalid metadata index: {}", metadata_index)));
        };
        
        headers.push(ParsedObjectHeader {
            name,
            full_name,
            metadata,
        });
    }
    
    Ok(headers)
}

// ============================================================================
// Property Header Parsing
// ============================================================================

/// Property type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    Compound,
    Scalar,
    Array,
}

/// Parsed property header with all fields.
#[derive(Debug, Clone)]
pub struct ParsedPropertyHeader {
    pub name: String,
    pub property_type: PropertyType,
    pub metadata: MetaData,
    pub data_type: DataType,
    pub time_sampling_index: u32,
    pub is_scalar_like: bool,
    pub is_homogenous: bool,
    pub next_sample_index: u32,
    pub first_changed_index: u32,
    pub last_changed_index: u32,
}

/// Get uint32 with variable size hint.
fn get_u32_with_hint(buf: &[u8], buf_size: usize, size_hint: u32, pos: &mut usize) -> Result<u32> {
    let result = match size_hint {
        0 => {
            if *pos + 1 > buf_size {
                return Err(Error::invalid("Truncated u8 in property header"));
            }
            let val = buf[*pos] as u32;
            *pos += 1;
            val
        }
        1 => {
            if *pos + 2 > buf_size {
                return Err(Error::invalid("Truncated u16 in property header"));
            }
            let val = read_u16_le(&buf[*pos..]) as u32;
            *pos += 2;
            val
        }
        2 => {
            if *pos + 4 > buf_size {
                return Err(Error::invalid("Truncated u32 in property header"));
            }
            let val = read_u32_le(&buf[*pos..]);
            *pos += 4;
            val
        }
        _ => return Err(Error::invalid("Invalid size hint")),
    };
    Ok(result)
}

/// Read property headers from a data block.
pub fn read_property_headers(
    data: &IData,
    indexed_metadata: &[MetaData],
) -> Result<Vec<ParsedPropertyHeader>> {
    let mut headers = Vec::new();
    
    if data.is_empty() {
        return Ok(headers);
    }
    
    let buf = data.read_all()?;
    let buf_size = buf.len();
    let mut pos = 0;
    
    while pos < buf_size {
        if pos + 4 > buf_size {
            return Err(Error::invalid("Property header info truncated"));
        }
        
        // First 4 bytes is info bitmask
        let info = read_u32_le(&buf[pos..]);
        pos += 4;
        
        // Property type (bits 0-1)
        let ptype = info & 0x0003;
        let is_scalar_like = (ptype & 1) != 0;
        let property_type = match ptype {
            0 => PropertyType::Compound,
            1 => PropertyType::Scalar,
            _ => PropertyType::Array,
        };
        
        // Size hint (bits 2-3)
        let size_hint = (info & 0x000c) >> 2;
        
        let (data_type, time_sampling_index, is_homogenous, 
             next_sample_index, first_changed_index, last_changed_index) = 
        if property_type != PropertyType::Compound {
            // POD type (bits 4-7)
            let pod = ((info & 0x00f0) >> 4) as u8;
            let pod_type = pod_from_u8(pod)
                .ok_or_else(|| Error::invalid(format!("Invalid POD type: {}", pod)))?;
            
            // Extent (bits 12-19)
            let extent = ((info & 0xff000) >> 12) as u8;
            let data_type = DataType::new(pod_type, extent);
            
            // Is homogenous (bit 10)
            let is_homogenous = (info & 0x400) != 0;
            
            // Next sample index
            let next_sample_index = get_u32_with_hint(&buf, buf_size, size_hint, &mut pos)?;
            
            // First/last changed indices
            let (first_changed_index, last_changed_index) = if (info & 0x0200) != 0 {
                // Explicit first/last
                let first = get_u32_with_hint(&buf, buf_size, size_hint, &mut pos)?;
                let last = get_u32_with_hint(&buf, buf_size, size_hint, &mut pos)?;
                (first, last)
            } else if (info & 0x800) != 0 {
                // All samples same
                (0, 0)
            } else {
                // Default: all change
                (1, next_sample_index.saturating_sub(1))
            };
            
            // Time sampling index
            let time_sampling_index = if (info & 0x0100) != 0 {
                get_u32_with_hint(&buf, buf_size, size_hint, &mut pos)?
            } else {
                0
            };
            
            (data_type, time_sampling_index, is_homogenous,
             next_sample_index, first_changed_index, last_changed_index)
        } else {
            (DataType::default(), 0, false, 0, 0, 0)
        };
        
        // Property name
        let name_size = get_u32_with_hint(&buf, buf_size, size_hint, &mut pos)? as usize;
        if name_size == 0 || pos + name_size > buf_size {
            return Err(Error::invalid("Property header name invalid"));
        }
        
        let name = std::str::from_utf8(&buf[pos..pos + name_size])
            .map_err(|e| Error::other(format!("Invalid UTF-8 in property name: {}", e)))?
            .to_string();
        pos += name_size;
        
        // Metadata
        let metadata_index = ((info & 0xff00000) >> 20) as usize;
        let metadata = if metadata_index == 0xff {
            // Inline metadata
            let metadata_size = get_u32_with_hint(&buf, buf_size, size_hint, &mut pos)? as usize;
            
            if pos + metadata_size > buf_size {
                return Err(Error::invalid("Property header metadata truncated"));
            }
            
            if metadata_size == 0 {
                MetaData::new()
            } else {
                let metadata_str = std::str::from_utf8(&buf[pos..pos + metadata_size])
                    .map_err(|e| Error::other(format!("Invalid UTF-8 in metadata: {}", e)))?;
                pos += metadata_size;
                MetaData::parse(metadata_str)
            }
        } else if metadata_index < indexed_metadata.len() {
            indexed_metadata[metadata_index].clone()
        } else {
            return Err(Error::invalid(format!("Invalid metadata index: {}", metadata_index)));
        };
        
        headers.push(ParsedPropertyHeader {
            name,
            property_type,
            metadata,
            data_type,
            time_sampling_index,
            is_scalar_like,
            is_homogenous,
            next_sample_index,
            first_changed_index,
            last_changed_index,
        });
    }
    
    Ok(headers)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Read little-endian u16 from bytes.
#[inline]
fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Read little-endian u32 from bytes.
#[inline]
fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Read little-endian f64 from bytes.
#[inline]
fn read_f64_le(bytes: &[u8]) -> f64 {
    f64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_read_u32_le() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        assert_eq!(read_u32_le(&bytes), 0x04030201);
    }
    
    #[test]
    fn test_read_f64_le() {
        let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F];
        assert_eq!(read_f64_le(&bytes), 1.0);
    }
}
