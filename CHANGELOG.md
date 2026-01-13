# CHANGELOG - Ogawa Writer Binary Compatibility

## Session 2026-01-12: Binary Compatibility Achieved

### Summary
Achieved **binary-compatible hash computation** with C++ Alembic implementation. All hash values now match exactly.

### Critical Fixes

#### 1. MurmurHash3 for Sample Digests
**Problem**: C++ uses MurmurHash3_x64_128 for ArraySampleContentKey digests, we were using MD5.

**Solution**: Created new `murmur3` crate (`crates/murmur3/`) with exact C++ implementation:
- `hash128()` - returns (u64, u64) tuple
- `hash128_bytes()` - returns [u8; 16] array
- Handles body (16-byte blocks), tail, and finalization

**Files changed**:
- `crates/murmur3/src/lib.rs` - New crate
- `src/core/cache.rs` - Use MurmurHash3 instead of MD5

#### 2. Empty Properties Hash
**Problem**: C++ calls `dataHash.Final()` even for empty compound properties, returning non-zero hash. We returned (0, 0).

**Solution**: Call `SpookyHash::new(0, 0).finalize()` for empty properties.

**C++ behavior** (OwData::writeHeaders):
```cpp
Util::SpookyHash dataHash;
dataHash.Init(0, 0);
m_data->computeHash(dataHash);  // May have no updates
dataHash.Final(&hashes[0], &hashes[1]);  // Still produces hash!
```

**Fixed Rust** (write_properties_with_data):
```rust
if props.is_empty() {
    let hasher = SpookyHash::new(0, 0);
    let (h1, h2) = hasher.finalize();  // Non-zero!
    return Ok((pos, h1, h2));
}
```

#### 3. Object Hash Includes Metadata and Name
**Problem**: Returned object hash didn't include metadata and object name.

**C++ behavior** (OwImpl::~OwImpl):
```cpp
m_data->writeHeaders(mdMap, hash);
hash.Update(&metaDataStr[0], metaDataStr.size());  // Add metadata
hash.Update(&m_header->getName()[0], m_header->getName().size());  // Add name
hash.Final(&hash0, &hash1);
```

**Fixed Rust** (write_object):
```rust
// Step 3: Update with metadata (if not empty)
let meta_str = obj.meta_data.serialize();
if !meta_str.is_empty() {
    combined_hash.update(meta_str.as_bytes());
}
// Step 4: Update with object name
combined_hash.update(obj.name.as_bytes());
// Step 5: Finalize
let (final_h1, final_h2) = combined_hash.finalize();
```

#### 4. Ogawa File Version Correction
**Problem**: We used `OGAWA_FILE_VERSION = 1`, C++ uses `0`.

**Solution**: Changed to match C++ `#define ALEMBIC_OGAWA_FILE_VERSION 0`.

### Crate Reorganization
Moved crates from root to `crates/` directory:
- `spooky-hash/` → `crates/spooky-hash/`
- `murmur3/` → `crates/murmur3/`

### Verification Results

**Hash comparison** (minimal file with root + one child "child1"):

| Region | C++ | Rust | Status |
|--------|-----|------|--------|
| Data hash (0x7B-0x8A) | `19 09 f5 6b fc 06 27 23 c7 51 e8 b4 65 ee 72 8b` | IDENTICAL | ✓ |
| Child hash (0x8B-0x9A) | `8e 86 83 86 10 f1 1e e4 ea 04 28 bd 17 77 2a 63` | IDENTICAL | ✓ |
| File version | 0 | 0 | ✓ |

**Remaining expected differences**:
- Metadata strings differ (our app name vs C++ app info)
- File size differs by ~10 bytes due to metadata length

### Test Results
All 143 tests pass:
- 109 unit tests
- 1 minimal hexdump test
- 1 C++ comparison test
- 13 read tests
- 18 write/roundtrip tests

---

## Session 2026-01-12 (earlier): Incremental Hashing Refactor

### Problem Analysis
Binary comparison showed ~46MB differences in a 50MB file. Root cause: hash computation approach differs.

### Changes Made

#### 1. SpookyHash Extension
Added `short_end_mix()` for ShortEnd accumulation.

#### 2. Property Hash Computation
- `hash_property_header()` - Exact match of C++ `HashPropertyHeader()`
- `hash_dimensions()` - Matches C++ `HashDimensions()`

#### 3. Incremental Sample Hashing
Using `short_end_mix()` to accumulate sample hashes matching C++ behavior.

### Key C++ Nuances Discovered
1. Scalar vs Array marker: Only scalars push a 0 byte
2. HashDimensions order: dimensions first, then digest
3. ShortEnd accumulation: First sample sets hash, subsequent mix
4. Property header includes TimeSampling data
5. Compound hash: Update with all child hashes as flat u64 array
