# CHANGELOG

## Session 2026-01-13: Time Sampling & Clippy Fixes

### Time Sampling Index Methods
Added `time_sampling_index()` to all geometry types, matching C++ Alembic implementation.

| Schema | Property Used |
|--------|---------------|
| IPolyMesh | `.geom` -> `P` (positions) |
| ISubD | `.geom` -> `P` (positions) |
| ICurves | `.geom` -> `P` (positions) |
| IPoints | `.geom` -> `P` (positions) |
| INuPatch | `.geom` -> `P` (positions) |
| ICamera | `.camera` -> `.core` |
| IXform | `.xform` -> `.inherits` |
| ILight | `.geom` -> `.childBnds` (fallback to `.camera/.core`) |
| IFaceSet | `.geom` -> `.faces` |

### Files Changed
- `src/geom/polymesh.rs` - Added `time_sampling_index()`
- `src/geom/subd.rs` - Added `time_sampling_index()`
- `src/geom/camera.rs` - Added `time_sampling_index()`
- `src/geom/xform.rs` - Added `time_sampling_index()`
- `src/geom/faceset.rs` - Fixed to use `.geom` compound and `self.object.as_ref()`
- `src/geom/light.rs` - Fixed to use `.geom` compound with proper fallback chain
- `src/python/schemas.rs` - Updated `getTimeSamplingIndex()` to use new methods

### Clippy Warnings Fixed
- `crates/standard-surface/src/params.rs` - Fixed `field_reassign_with_default` warnings
- `src/python/write.rs` - Suppressed `too_many_arguments` with comments (5 functions)

### Documentation
- Created `PLAN3.md` with remaining tasks summary

---

## Session 2026-01-13: Project Restructuring

### Changes
Moved viewer and CLI from separate crates into main library structure.

### New Structure
- `src/viewer/` - Viewer module (was `crates/alembic-viewer/`)
- `src/bin/alembic/main.rs` - CLI binary (was `crates/abc/`)
- Library: `alembic_core` (renamed to avoid collision with binary)
- Binary: `alembic`

### Removed
- `crates/alembic-viewer/` - moved to `src/viewer/`
- `crates/abc/` - moved to `src/bin/alembic/`

### Files Changed
- `src/viewer/mod.rs` - New viewer module entry point
- `src/viewer/*.rs` - Updated imports (`alembic::` -> `crate::`, `crate::` -> `super::`)
- `src/bin/alembic/main.rs` - CLI using `alembic_core::`
- `src/lib.rs` - Added `#[cfg(feature = "viewer")] pub mod viewer`
- `Cargo.toml` - Removed old workspace members, lib renamed to `alembic_core`

---

## Session 2026-01-13: Unified CLI + Viewer Binary

### Changes
Merged `alembic-cli` and `alembic-viewer` into single binary `abc`.

### New Structure
- `crates/abc/` - Unified CLI with viewer support
- Binary name: `abc` (was `alembic-cli` and `alembic-viewer`)
- Viewer enabled by default via `--features viewer`

### CLI Commands
```
abc view <file>              # Open 3D viewer (Esc to exit)
abc info <file>              # Archive info and object counts
abc tree <file>              # Object hierarchy
abc stats <file>             # Detailed statistics
abc dump <file> [pattern]    # Xform transforms (--json for JSON)
abc copy <in> <out>          # Round-trip copy test
```

### Viewer Improvements
- Added **Shadows** toggle checkbox in Display settings
- Added **Opacity** slider (0.1-1.0) for X-Ray mode transparency
- Press **Esc** to close viewer
- Settings persist between sessions

### Files Changed
- `crates/abc/` - New unified CLI crate
- `crates/alembic-viewer/src/lib.rs` - Exposed `run()` function
- `crates/alembic-viewer/src/app.rs` - Esc handling, shadows toggle
- `crates/alembic-viewer/src/settings.rs` - Added `show_shadows`, `xray_alpha`
- `crates/alembic-viewer/src/renderer.rs` - Conditional shadow pass, xray_alpha uniform
- `crates/standard-surface/src/params.rs` - Added `xray_alpha` to CameraUniform
- `crates/standard-surface/src/shaders/standard_surface.wgsl` - Alpha override in fragment shader
- `Cargo.toml` - Removed cyclic dependency, added abc to workspace

---

## Session 2026-01-13: Xform Op Decoding Fix (BMW.abc rendering bug)

### Problem
BMW.abc geometry was "flying apart" in the viewer - wheels and body parts scattered instead of showing assembled car.

### Root Cause
Xform operation code decoding used wrong nibble order.

**C++ encoding** (XformOp.cpp line 279):
```cpp
return ( m_type << 4 ) | ( m_hint & 0xF );
```
- Upper nibble (bits 4-7) = operation type
- Lower nibble (bits 0-3) = hint

**Bug in Rust** (decode_xform_op):
```rust
// WRONG: was extracting lower nibble as type
let op_type = code & 0x0F;
```

**Fix**:
```rust
// CORRECT: extract upper nibble
let op_type = code >> 4;
```

### Secondary Bug - Writer Op Encoding
Writer was encoding Matrix op incorrectly:
```rust
// WRONG
let op_data = vec![3u8; 1];  // Type 3 in lower nibble

// CORRECT
let op_data = vec![0x30u8; 1];  // (3 << 4) | 0 = 0x30
```

### Tertiary Bug - Ops/Vals Size Reading
Was finding ops count by searching for null byte (0x00), but 0x00 is valid Scale operation!

**C++ behavior** (IXform.cpp):
```cpp
std::size_t numOps = ops->getHeader().getDataType().getExtent();
```

**Fix**: Use `header().data_type.extent` instead of null-byte search.

### XformOperationType Enum (Foundation.h)
```
0 = kScaleOperation
1 = kTranslateOperation
2 = kRotateOperation
3 = kMatrixOperation
4 = kRotateXOperation
5 = kRotateYOperation
6 = kRotateZOperation
```

### Files Changed
- `src/geom/xform.rs` - Fixed `decode_xform_op()` and ops/vals reading
- `src/ogawa/writer.rs` - Fixed Matrix op encoding (3 -> 0x30)

### Verification
BMW.abc now renders correctly with all parts in place.

---

## Session 2026-01-12: Python API Parity & Documentation

### Schema Reader Classes (Original Alembic API Style)
Added full set of schema reader wrappers matching the original C++ Alembic Python bindings:

- `IPolyMesh` / `IPolyMeshSchema`
- `IXform` / `IXformSchema`
- `ISubD` / `ISubDSchema`
- `ICurves` / `ICurvesSchema`
- `IPoints` / `IPointsSchema`
- `ICamera` / `ICameraSchema`
- `ILight` / `ILightSchema`
- `INuPatch` / `INuPatchSchema`
- `IFaceSetTyped` / `IFaceSetSchema`

Usage:
```python
from alembic_rs import IPolyMesh

mesh = IPolyMesh(obj)
schema = mesh.getSchema()
sample = schema.getValue()  # default frame 0
sample = schema.getValue(5)  # specific frame
```

### Documentation Updates
- Created README.md with experimental notice
- Updated mdbook Python API docs with schema-style API
- Added Python examples to reference/schemas.md

### Files Changed
- `src/python/schemas.rs` - New file with 18 schema classes
- `src/python/mod.rs` - Registered all new classes
- `python/alembic_rs/__init__.py` - Updated __all__ exports
- `docs/src/python/overview.md` - Added schema reader classes
- `docs/src/python/reading.md` - Added schema-style API section
- `docs/src/reference/schemas.md` - Added Python examples

---

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
