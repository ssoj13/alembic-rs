//! Array sample cache implementation.
//!
//! Provides caching for array samples to improve read performance
//! when the same data is accessed multiple times.
//!
//! Also provides content-based keys for write deduplication.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::RwLock;
use md5::{Md5, Digest};

/// 128-bit digest for content-based deduplication.
pub type SampleDigest = [u8; 16];

/// Compute MD5 digest of data for deduplication.
#[inline]
pub fn compute_digest(data: &[u8]) -> SampleDigest {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Key for cache entries (position-based for reading).
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct ArraySampleKey {
    /// File position of the data group.
    pub data_pos: u64,
    /// Sample index within the property.
    pub sample_index: usize,
}

impl ArraySampleKey {
    /// Create a new cache key.
    pub fn new(data_pos: u64, sample_index: usize) -> Self {
        Self { data_pos, sample_index }
    }
}

/// Content-based key for write deduplication.
/// Uses MD5 digest of the actual data content.
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct ArraySampleContentKey {
    /// 128-bit MD5 digest of the data.
    pub digest: SampleDigest,
    /// Size of the data in bytes (for collision detection).
    pub size: usize,
}

impl ArraySampleContentKey {
    /// Create a new content key from data.
    pub fn from_data(data: &[u8]) -> Self {
        Self {
            digest: compute_digest(data),
            size: data.len(),
        }
    }
    
    /// Create from existing digest and size.
    pub fn from_digest(digest: SampleDigest, size: usize) -> Self {
        Self { digest, size }
    }
    
    /// Get the digest bytes.
    pub fn digest(&self) -> &SampleDigest {
        &self.digest
    }
    
    /// Check if digest is all zeros (no key).
    pub fn is_empty(&self) -> bool {
        self.digest == [0u8; 16]
    }
}

/// Cached array sample data.
#[derive(Clone)]
pub struct CachedSample {
    /// The sample data.
    pub data: Arc<Vec<u8>>,
    /// Approximate size in bytes (for cache eviction).
    pub size: usize,
}

/// Thread-safe cache for array samples.
/// 
/// Uses `parking_lot::RwLock` for faster, non-poisoning locks
/// and `AtomicUsize` for lock-free size tracking.
pub struct ReadArraySampleCache {
    /// Cache storage.
    cache: RwLock<HashMap<ArraySampleKey, CachedSample>>,
    /// Maximum cache size in bytes.
    max_size: usize,
    /// Current cache size in bytes (atomic for lock-free reads).
    current_size: AtomicUsize,
}

impl ReadArraySampleCache {
    /// Create a new cache with the given maximum size in bytes.
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
            current_size: AtomicUsize::new(0),
        }
    }
    
    /// Create a cache with default size (64 MB).
    pub fn default_size() -> Self {
        Self::new(64 * 1024 * 1024)
    }
    
    /// Get a cached sample if it exists.
    #[inline]
    pub fn get(&self, key: &ArraySampleKey) -> Option<Arc<Vec<u8>>> {
        let cache = self.cache.read();
        cache.get(key).map(|s| Arc::clone(&s.data))
    }
    
    /// Insert a sample into the cache.
    pub fn insert(&self, key: ArraySampleKey, data: Vec<u8>) {
        let size = data.len();
        
        // Don't cache if larger than max size
        if size > self.max_size {
            return;
        }
        
        // Check if we need to evict (relaxed ordering is fine for heuristic)
        let current = self.current_size.load(Ordering::Relaxed);
        if current + size > self.max_size {
            self.evict_some();
        }
        
        let sample = CachedSample {
            data: Arc::new(data),
            size,
        };
        
        let mut cache = self.cache.write();
        // Don't insert duplicates
        if cache.contains_key(&key) {
            return;
        }
        
        cache.insert(key, sample);
        self.current_size.fetch_add(size, Ordering::Relaxed);
    }
    
    /// Evict approximately half of the cache.
    fn evict_some(&self) {
        let mut cache = self.cache.write();
        let keys: Vec<_> = cache.keys().cloned().collect();
        let evict_count = keys.len() / 2;
        
        let mut evicted_size = 0;
        for key in keys.into_iter().take(evict_count) {
            if let Some(sample) = cache.remove(&key) {
                evicted_size += sample.size;
            }
        }
        
        // Use fetch_sub with saturating semantics
        let _ = self.current_size.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |x| Some(x.saturating_sub(evicted_size))
        );
    }
    
    /// Clear the entire cache.
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        self.current_size.store(0, Ordering::Relaxed);
    }
    
    /// Get the number of cached entries.
    #[inline]
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }
    
    /// Check if cache is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get current cache size in bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.current_size.load(Ordering::Relaxed)
    }
    
    /// Get maximum cache size in bytes.
    #[inline]
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

impl Default for ReadArraySampleCache {
    fn default() -> Self {
        Self::default_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_insert_get() {
        let cache = ReadArraySampleCache::new(1024);
        let key = ArraySampleKey::new(100, 0);
        let data = vec![1, 2, 3, 4, 5];
        
        cache.insert(key, data.clone());
        
        let result = cache.get(&key);
        assert!(result.is_some());
        assert_eq!(*result.unwrap(), data);
    }
    
    #[test]
    fn test_cache_miss() {
        let cache = ReadArraySampleCache::new(1024);
        let key = ArraySampleKey::new(100, 0);
        
        assert!(cache.get(&key).is_none());
    }
    
    #[test]
    fn test_cache_clear() {
        let cache = ReadArraySampleCache::new(1024);
        let key = ArraySampleKey::new(100, 0);
        cache.insert(key, vec![1, 2, 3]);
        
        assert!(!cache.is_empty());
        
        cache.clear();
        
        assert!(cache.is_empty());
        assert!(cache.get(&key).is_none());
    }
    
    #[test]
    fn test_cache_eviction() {
        // Small cache that can only hold ~50 bytes
        let cache = ReadArraySampleCache::new(50);
        
        // Insert several entries
        for i in 0..10u64 {
            let key = ArraySampleKey::new(i * 100, 0);
            cache.insert(key, vec![0u8; 10]);
        }
        
        // Some should have been evicted
        assert!(cache.len() <= 5);
    }
    
    #[test]
    fn test_cache_skip_large() {
        let cache = ReadArraySampleCache::new(100);
        let key = ArraySampleKey::new(100, 0);
        
        // Data larger than cache max should not be cached
        cache.insert(key, vec![0u8; 200]);
        
        assert!(cache.get(&key).is_none());
    }
}
