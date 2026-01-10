//! Compression support for Alembic data.
//!
//! Alembic uses zlib compression for data blocks when compression is enabled.

use std::io::{Read, Write};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::util::Result;

/// Compress data using zlib.
/// 
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-9, where 0 is no compression, 9 is max)
/// 
/// Returns compressed data with 8-byte header containing uncompressed size.
pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>> {
    if level <= 0 || data.is_empty() {
        // No compression - return original data
        return Ok(data.to_vec());
    }
    
    let compression_level = match level {
        1 => Compression::fast(),
        2..=5 => Compression::default(),
        6..=9 => Compression::best(),
        _ => Compression::default(),
    };
    
    let mut encoder = ZlibEncoder::new(Vec::new(), compression_level);
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    
    // Only use compression if it actually saves space
    if compressed.len() >= data.len() {
        return Ok(data.to_vec());
    }
    
    // Format: [uncompressed_size: u64 LE][compressed_data]
    let mut result = Vec::with_capacity(8 + compressed.len());
    result.extend_from_slice(&(data.len() as u64).to_le_bytes());
    result.extend_from_slice(&compressed);
    
    Ok(result)
}

/// Decompress data that was compressed with zlib.
/// 
/// # Arguments
/// * `data` - Compressed data with 8-byte header containing uncompressed size
/// 
/// Returns decompressed data, or original data if not compressed.
/// Returns error only if data appears to be compressed but decompression fails.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    // First check if data appears to be compressed using our heuristics
    if !is_compressed(data) {
        // Data doesn't look compressed - return as-is (this is normal)
        return Ok(data.to_vec());
    }
    
    // Data looks compressed - parse header and decompress
    let uncompressed_size = u64::from_le_bytes([
        data[0], data[1], data[2], data[3],
        data[4], data[5], data[6], data[7],
    ]) as usize;
    
    // Additional sanity check for unreasonable sizes
    if uncompressed_size > 1024 * 1024 * 1024 { // > 1GB is suspicious
        return Err(crate::util::Error::invalid(
            "Compressed data claims unreasonable uncompressed size"
        ));
    }
    
    let compressed_data = &data[8..];
    
    let mut decoder = ZlibDecoder::new(compressed_data);
    let mut decompressed = Vec::with_capacity(uncompressed_size);
    
    decoder.read_to_end(&mut decompressed).map_err(|e| {
        crate::util::Error::invalid(format!(
            "Failed to decompress data: {}", e
        ))
    })?;
    
    // Verify decompressed size matches expected
    if decompressed.len() != uncompressed_size {
        return Err(crate::util::Error::invalid(format!(
            "Decompressed size mismatch: expected {}, got {}",
            uncompressed_size, decompressed.len()
        )));
    }
    
    Ok(decompressed)
}

/// Check if data appears to be compressed.
/// 
/// Returns true if data has the zlib header signature.
pub fn is_compressed(data: &[u8]) -> bool {
    if data.len() < 10 {
        return false;
    }
    
    // Check for zlib header after size prefix
    // zlib header: 0x78 followed by 0x01, 0x5E, 0x9C, or 0xDA
    let zlib_header = data[8];
    let zlib_flags = data[9];
    
    // Valid zlib compression levels:
    // 0x01 - no compression
    // 0x9C - default compression  
    // 0xDA - best compression
    // Note: 0x5E was previously listed but is not a standard zlib level
    zlib_header == 0x78 && matches!(zlib_flags, 0x01 | 0x9C | 0xDA)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compress_decompress() {
        let original = b"Hello, World! This is some test data that should compress well when repeated. ".repeat(100);
        
        let compressed = compress(&original, 6).unwrap();
        
        // Compressed should be smaller
        assert!(compressed.len() < original.len());
        
        let decompressed = decompress(&compressed).unwrap();
        
        assert_eq!(decompressed, original);
    }
    
    #[test]
    fn test_no_compression_level_zero() {
        let original = b"Short data";
        
        let result = compress(original, 0).unwrap();
        
        assert_eq!(result, original);
    }
    
    #[test]
    fn test_no_compression_if_larger() {
        // Very short data that won't compress well
        let original = b"Hi";
        
        let result = compress(original, 9).unwrap();
        
        // Should return original if compression doesn't help
        assert_eq!(result, original);
    }
    
    #[test]
    fn test_decompress_uncompressed() {
        let original = b"Not compressed data";
        
        // Should return as-is if not compressed (no zlib header)
        let result = decompress(original).unwrap();
        
        assert_eq!(result, original);
    }
    
    #[test]
    fn test_decompress_corrupt_fails() {
        // Create data that looks compressed (has zlib header) but is corrupt
        let mut corrupt = vec![0u8; 16];
        corrupt[0..8].copy_from_slice(&100u64.to_le_bytes()); // Claims 100 bytes
        corrupt[8] = 0x78; // zlib header
        corrupt[9] = 0x9C; // default compression flag
        // Rest is garbage - should fail to decompress
        
        let result = decompress(&corrupt);
        assert!(result.is_err(), "Should fail on corrupt compressed data");
    }
    
    #[test]
    fn test_is_compressed() {
        let original = b"Test data for compression ".repeat(50);
        let compressed = compress(&original, 6).unwrap();
        
        assert!(is_compressed(&compressed));
        assert!(!is_compressed(&original));
    }
}
