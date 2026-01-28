# CHANGELOG

## Session 2026-01-26 .. 2026-01-28: Viewer Major Upgrade

### Path Tracer (GPU compute, new module `src/viewer/pathtracer/`)
- Full GPU path tracer via wgpu compute shaders with BVH acceleration
- PBR materials: glass (transmission + IOR), metals, plastics, rubber, leather
- Auto-materializer: `guess_material_from_path()` assigns materials based on object name keywords
- Depth of Field with aperture/focus controls, world-space focus point tracking
- Focus picking: Ctrl+LMB or MMB click sets DoF focus to picked surface point
- HDR environment lighting integration (env map sampling in PT shaders)
- Curves rendered as ribbon triangles in PT (`curves_to_ribbon_tris()`)
- Per-object visibility buffer for PT (floor toggle syncs to PT)
- Accumulation reset on camera move, env change, background toggle, focus change
- Settings: `pt_samples_per_update`, `pt_max_transmission_depth`, `pt_dof_enabled`, `pt_aperture`, `pt_focus_distance`, `materialize_missing`

### Object Picking & Hover Highlighting
- Object ID render pass (`object_id.wgsl`) for GPU-based picking
- Outline shader (`outline.wgsl`) with configurable thickness and alpha
- Highlight shader (`highlight.wgsl`) for tint-based hover
- `HoverMode` enum: None, Outline, Tint, Both (persisted in settings)
- LMB click = object selection, selected object shown in properties panel

### Camera Overhaul
- Replaced `dolly` crate with custom Maya-style orbit camera (`OrbitCamera`)
- Pure glam math: target + yaw/pitch + arm distance
- Inertia system: exponential velocity decay on orbit/pan/zoom after drag release
- `begin_drag()` / `end_drag()` / `update(dt)` / `kill_inertia()` API
- Configurable `inertia_ms` (default 150ms half-life)
- Ctrl+LMB = continuous DoF focus sampling (disables orbit)

### Renderer Optimizations
- Pre-cached post-fx bind groups: SSAO, blur (H+V separate), lighting bind groups rebuilt only on resize/env change (`postfx_bind_groups_dirty` flag)
- Separated SSAO blur into horizontal + vertical passes (two buffers/bind groups)
- GPU buffer reuse: `write_buffer()` when new data fits existing allocation, recreate only when larger
- Pre-computed per-object data hashes on worker thread, O(1) comparison on main thread
- `apply_scene()` skips full scene hash during animation (frame change = data change)
- Vertex buffer created with `COPY_DST` usage for in-place updates
- Performance tracing: warnings logged when mesh upload or PT upload exceeds 5ms

### Smooth Normals
- `smooth_normals.rs`: angle-weighted smooth normal recalculation
- Smooth angle threshold setting, dirty flag per mesh
- Base vertices preserved for re-smoothing on deformation

### Write Tests (891 new lines in `tests/write_tests.rs`)
- Roundtrip tests: curves (Linear, Cubic, Periodic, multi-width), points, SubD, camera, light, NuPatch, FaceSet
- Multi-sample animation tests for all geometry types
- Binary diff analysis test

### Python Bindings
- `ISampleSelector` class for frame-based sample queries
- `getValue(index, selector)` on all schema classes (PolyMesh, Xform, SubD, Curves, Points, Camera, Light, NuPatch, FaceSet)
- `getArchive()` and `getMetaData()` on `IObject`
- `TimeSampling` class exposed
- Fixed `CurveType` constants to match C++ (`kCubic=0, kLinear=1, kVariableOrder=2`)

### CLI Additions
- Extended `alembic` binary with new commands and options
- `bmw_mat.abc` test data file

### Other
- Removed `dolly` dependency, added glam-only camera
- `.gitignore` updates for trace/log files
- `FINDINGS.md`, `PLAN_1.md`, `PLAN_2.md` working documents

---

## Session 2026-01-16: FULL BINARY PARITY with C++ Alembic

### Achievement
**100% byte-identical output** with C++ Alembic library. The `copy2` command now produces files that are SHA256-identical to C++ reference output.

### Verification
```
Rust: 1011 bytes
C++:  1011 bytes
Total differences: 0
SHA256 match: True
```

### Critical Fixes

#### 1. Property Order in All Schemas
**Problem**: Properties were written in Rust iteration order, not C++ order.

**C++ order** (PolyMesh example from OPolyMeshSchema.cpp):
1. `P` (positions)
2. `.selfBnds` (self bounds)
3. `.faceIndices`
4. `.faceCounts`
5. `uv` (if present)
6. `N` (normals, if present)
7. `velocity` (if present)

**Solution**: Explicit property ordering matching C++ destructor call order in all schema writers.

#### 2. Indexed Metadata Order
**Problem**: `.selfBnds` was being written before `P` in indexed metadata.

**C++ behavior**: Properties are added to indexed metadata in creation order. `P` is created first in OPolyMeshSchema constructor.

**Solution**: Ensured `P` property header appears first in indexed metadata, then `.selfBnds`.

#### 3. App String Copying
**Problem**: Writer was using hardcoded app string instead of copying from source file.

**Solution**: `copy2` command now extracts and preserves the original app metadata string:
```rust
let app_str = archive.app_info().unwrap_or_default();
let writer = OArchive::new_with_app(&output_path, &app_str)?;
```

#### 4. Time Sampling Copying
**Problem**: Time sampling data was not being copied from source archive.

**Solution**: Added time sampling extraction and application in copy2:
- Extract time sampling from source properties
- Apply matching time sampling index to output properties
- Preserve acyclic vs cyclic time sampling types

#### 5. Compound Property Hash Computation
**Problem**: Nested compound properties had incorrect hash computation.

**C++ behavior** (CpwImpl::~CpwImpl):
```cpp
// For nested compounds:
// 1. computeHash(hash) - updates with m_hashes (child property hashes)
// 2. HashPropertyHeader(header, hash) - adds property name + metadata
// 3. hash.Final() - produces final hash
```

**Solution**: Modified `finalize_property_group` to return raw property hashes, then recompute compound hash with property header:
```rust
let (pos, _, _, raw_prop_hashes) = self.write_properties_with_data(sub_props)?;

// Recompute hash: child_hashes + property_header
let mut hasher = SpookyHash::new(0, 0);
let hash_bytes: Vec<u8> = raw_prop_hashes.iter()
    .flat_map(|h| h.to_le_bytes())
    .collect();
hasher.update(&hash_bytes);
hash_property_header(&mut hasher, prop, &time_sampling);
let (h1, h2) = hasher.finalize();
```

#### 6. Deferred Mode Disabled (KEY FIX)
**Problem**: Rust was using `deferred_mode: true` which wrote object groups at end of file, but C++ writes them inline.

**C++ behavior**: Object groups are written when destructors run (inline with data).
- C++: object groups at offset 519, indexed metadata at 543
- Rust (deferred): indexed metadata at 519, object groups at 572+

**Solution**: Disabled deferred_mode to write groups inline:
```rust
OArchiveBuilder {
    deferred_mode: false,  // Disabled for binary parity
    ...
}
```

### Files Changed
- `src/ogawa/writer.rs`:
  - `write_properties_with_object_headers()` returns 5-tuple with raw prop hashes
  - `finalize_property_group()` Compound branch recomputes hash
  - `OArchiveBuilder::deferred_mode` set to `false`
- `src/bin/alembic/main.rs`:
  - `copy2_archive()` copies app string from source
  - Added time sampling copying logic

### Test Files
Comparison tools in `tools/`:
- `compare_files.ps1` - Byte-by-byte comparison
- `dump_region.ps1` - Hex dump with ASCII
- `decode_headers.ps1` - Property header analysis

### C++ Reference Consulted
- `_ref/alembic/lib/Alembic/AbcCoreOgawa/AwImpl.cpp` - Archive destructor order
- `_ref/alembic/lib/Alembic/AbcCoreOgawa/CpwImpl.cpp` - Compound hash computation
- `_ref/alembic/lib/Alembic/AbcCoreOgawa/OwData.cpp` - Object data/headers
- `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp` - HashPropertyHeader
- `_ref/alembic/lib/Alembic/AbcGeom/OPolyMesh.cpp` - Property creation order

---

## Session 2026-01-14: IBL (Image-Based Lighting) Fix

### Problem
HDR environment maps were creating unwanted "texture" patterns on all objects. The diffuse IBL was sampling the HDR directly along surface normals, projecting high-contrast HDR patterns onto geometry.

### Solution

#### Diffuse IBL - Average Environment Color
Instead of sampling HDR along normal (which creates texture), sample 6 axis directions (+X, -X, +Y, -Y, +Z, -Z) and average them. This gives uniform environment tint without patterns.

```wgsl
var env_diffuse = sample_env(vec3<f32>(1.0, 0.0, 0.0));
env_diffuse += sample_env(vec3<f32>(-1.0, 0.0, 0.0));
env_diffuse += sample_env(vec3<f32>(0.0, 1.0, 0.0));
env_diffuse += sample_env(vec3<f32>(0.0, -1.0, 0.0));
env_diffuse += sample_env(vec3<f32>(0.0, 0.0, 1.0));
env_diffuse += sample_env(vec3<f32>(0.0, 0.0, -1.0));
env_diffuse = env_diffuse / 6.0;
```

#### Specular IBL - Proper Fresnel
Enhanced specular reflections with proper Fresnel effect (brighter at grazing angles):

```wgsl
let fresnel = F0 + (vec3<f32>(1.0) - F0) * pow5(1.0 - NdotV);
let spec_atten = 1.0 - specular_roughness * specular_roughness;
specular_accum += env_specular * fresnel * spec_atten;
```

### Files Changed
- `crates/standard-surface/src/shaders/standard_surface.wgsl` - IBL section rewritten

---

## Session 2026-01-13: Viewer UI Enhancements

### Flat Shading Mode
Added flat shading toggle using `dpdx/dpdy` screen-space derivatives for face normals.

**Files Changed:**
- `crates/standard-surface/src/params.rs` - Added `flat_shading` to CameraUniform
- `crates/standard-surface/src/shaders/standard_surface.wgsl` - Compute flat normals in fragment shader
- `src/viewer/settings.rs` - Added `flat_shading` setting
- `src/viewer/renderer.rs` - Pass flat_shading to shader
- `src/viewer/app.rs` - UI checkbox

### Object Hierarchy Tree
Added left panel showing scene hierarchy with collapsible nodes.

**Features:**
- Icons per object type (▲ PolyMesh, ■ SubD, ↺ Xform, ◎ Camera, ☀ Light, ∿ Curves, • Points)
- Collapsible tree nodes
- Object selection

### Properties Panel
Shows selected object properties in right panel.

**Displays:**
- Object name and type
- Sample count
- PolyMesh: vertex/face counts
- Xform: position and rotation
- Camera: focal length, aperture

---

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
