# Alembic-RS Bug Hunt Report

## Executive Summary

Code audit of `alembic-rs` Rust implementation of Alembic (.abc) 3D interchange format.
Codebase: ~27,162 lines across 70 Rust source files.

---

## 1. Dead Code Analysis

### 1.1 #[allow(dead_code)] Annotations

Found **31 instances** of `#[allow(dead_code)]` across the codebase:

#### `src/ogawa/abc_impl.rs`
- **Line 33**: `inner: Arc<OgawaIArchive>` - field stored but never accessed
- **Line 42-43**: `cache: Arc<ReadArraySampleCache>` - field stored but never accessed

**Analysis**: These fields are being stored for potential future use or to maintain Arc reference counts. The `inner` field may be needed for lifetime management.

#### `src/ogawa/writer.rs`
- **Line 630**: `fn write_properties()` - wrapper method, superseded by `write_properties_with_data()`
- **Line 694**: `fn write_property()` - wrapper method, superseded by `write_property_with_data()`
- **Line 818**: `fn serialize_object_headers()` - wrapper method, superseded by `serialize_object_headers_with_hash()`

**Analysis**: These are **intentional API wrappers** for potential future use. They provide simpler interfaces without hash computation. Can be removed if not needed.

#### `src/viewer/` module (16 instances)
Fields held alive for GPU resource management:
- `viewport.rs:29` - `texture: wgpu::Texture`
- `renderer.rs:36-74` - Various GPU resources (layouts, buffers, textures)
- `environment.rs:8-19` - Environment map resources

Fields for future features:
- `mesh_converter.rs:93` - `widths: Vec<f32>` (point sprites)
- `mesh_converter.rs:107-127` - Camera aperture and aspect methods
- `mesh_converter.rs:134-180` - `SceneLight`, `SceneMaterial` structs
- `renderer.rs:903` - `has_points()` method
- `renderer.rs:1020` - `update_curves_transform()` method
- `app.rs:422` - Old tree node filtering method (kept for reference)

**Verdict**: Viewer dead code is mostly **intentional** - GPU resources need to stay alive, and some features are partially implemented.

### 1.2 Phantom Data Structs (geom/mod.rs)

```rust
// Lines 105-170: Empty output schema structs
pub struct OPolyMesh { _phantom: PhantomData<()> }
pub struct OXform { _phantom: PhantomData<()> }
pub struct OCurves { _phantom: PhantomData<()> }
pub struct OPoints { _phantom: PhantomData<()> }
pub struct OSubD { _phantom: PhantomData<()> }
pub struct OCamera { _phantom: PhantomData<()> }
pub struct OFaceSet { _phantom: PhantomData<()> }
pub struct ONuPatch { _phantom: PhantomData<()> }
```

**ISSUE**: These are **placeholder structs** that conflict with real implementations in `ogawa/writer.rs`. The writer module has full implementations:
- `ogawa/writer.rs:1352-1472` - `OPolyMesh` (real implementation)
- `ogawa/writer.rs:1496-1583` - `OXform` (real implementation)
- `ogawa/writer.rs:1643-1754` - `OCurves` (real implementation)
- etc.

**Recommendation**: Remove PhantomData stubs from `geom/mod.rs` or consolidate with writer types.

---

## 2. Code Duplication Patterns

### 2.1 Schema Writer Duplication (ogawa/writer.rs)

Each schema writer (OPolyMesh, OCurves, OPoints, OSubD) has nearly identical `get_or_create_array` and `get_or_create_scalar` methods:

```rust
// Repeated pattern in OPolyMesh (line 1430), OCurves (line 1720),
// OPoints (line 1823), OSubD (line 1980)
fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
    if let OPropertyData::Compound(children) = &mut self.geom_compound.data {
        if let Some(idx) = children.iter().position(|p| p.name == name) {
            return &mut children[idx];
        }
        children.push(OProperty::array(name, dt));
        children.last_mut().unwrap()
    } else {
        unreachable!()
    }
}
```

**Recommendation**: Extract into a trait or helper:
```rust
trait GeomPropertyAccess {
    fn geom_compound_mut(&mut self) -> &mut OProperty;

    fn get_or_create_array(&mut self, name: &str, dt: DataType) -> &mut OProperty {
        // shared implementation
    }
}
```

### 2.2 ICamera vs Other Schemas

`ICamera` (camera.rs) doesn't use `geom::util` helpers while other schemas do. Camera uses direct property access patterns that duplicate functionality in util.rs.

Compare:
- `curves.rs:237-254`: Uses `geom_util::num_samples_from_positions()`, `geom_util::positions_time_sampling_index()`
- `camera.rs:393-514`: Manual property navigation without helpers

**Recommendation**: Refactor ICamera to use `geom::util` helpers.

### 2.3 ISubD and IPolyMesh get_uvs/get_normals

Both `ISubD` and `IPolyMesh` have similar patterns for reading indexed UVs/normals with identical expansion logic.

`subd.rs:392-435` and `polymesh.rs` share ~90% identical code for:
- Reading indexed compound with `.vals` and `.indices`
- Expanding indices to per-face-vertex values
- Fallback to direct array read

**Recommendation**: Extract to `geom::util::read_and_expand_indexed_vec2/vec3()`.

---

## 3. TODO/FIXME Items

### Single TODO Found
**File**: `src/viewer/viewport.rs:221`
```rust
// TODO: Calculate scene bounds
```

**Context**: In viewport fitting logic. Currently uses hardcoded bounds.

---

## 4. Error Handling Analysis

### 4.1 unwrap() Usage Summary
- **Total**: 70 occurrences across 12 files
- **Tests**: 18 occurrences (acceptable)
- **Production code concerns**:
  - `murmur3/lib.rs:4` - Hash computation
  - `ogawa/writer.rs:16` - File writing
  - `geom/curves.rs:2` - Sample parsing

### 4.2 expect() Usage Summary
- **Total**: 119 occurrences across 4 files
- **All in test files** - acceptable

### Assessment
The library code has minimal `unwrap()` usage. Most are in:
1. Test files (acceptable)
2. Internal assertions where panic is appropriate
3. Pattern matches on `Option` after `.last_mut()` (safe after push)

---

## 5. Interface Compatibility Issues

### 5.1 OObject in abc/mod.rs vs ogawa/writer.rs

**abc/mod.rs:567-583**:
```rust
pub struct OObject<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl OObject<'_> {
    pub fn getName(&self) -> &str { "" }  // Stub!
    pub fn valid(&self) -> bool { true }
}
```

**ogawa/writer.rs:1128-1180**:
```rust
pub struct OObject {
    pub name: String,
    pub meta_data: MetaData,
    pub children: Vec<OObject>,
    pub properties: Vec<OProperty>,
}
// Full implementation with add_child(), add_property(), etc.
```

**ISSUE**: Two different `OObject` types exist - abc/mod.rs has a useless stub, while ogawa/writer.rs has the real implementation.

**Impact**:
- `abc::OArchive::root()` returns the stub OObject
- Users must use `ogawa::OArchive` directly for actual writing

### 5.2 Output Property Stubs

Similarly, `abc/mod.rs` has stub output types:
- `OCompoundProperty<'a>` (line 671-673) - empty struct
- `OScalarProperty<'a>` (line 793-795) - empty struct
- `OArrayProperty<'a>` (line 1137-1139) - empty struct

These are never used - all writing goes through `ogawa::writer` types.

---

## 6. Architecture Analysis

### 6.1 Module Hierarchy

```
alembic/
├── abc/           # High-level API (IArchive, IObject) - READ works, WRITE is stub
├── core/          # Traits, TimeSampling, MetaData
├── geom/          # Geometry schemas (I* readers work, O* are stubs in mod.rs)
├── ogawa/         # Low-level Ogawa format (both read and write work)
│   ├── reader.rs  # Binary reading
│   ├── writer.rs  # Binary writing + O* schema builders (REAL implementations)
│   └── abc_impl.rs# Archive/Object reader implementations
├── material/      # Material schema
├── collection/    # Collection schema
├── viewer/        # Optional 3D viewer
└── python/        # Optional Python bindings
```

### 6.2 Dataflow Diagram

```
                    ┌─────────────────────────────────────────────────────┐
                    │                    User Code                         │
                    └─────────────────────────────────────────────────────┘
                                           │
                      ┌────────────────────┼────────────────────┐
                      │ READ               │                WRITE│
                      ▼                    │                    ▼
              ┌───────────────┐            │           ┌───────────────┐
              │  abc::IArchive │            │           │ogawa::OArchive│
              │   (high-level) │            │           │  (low-level)  │
              └───────┬───────┘            │           └───────┬───────┘
                      │                    │                    │
                      ▼                    │                    ▼
              ┌───────────────┐            │           ┌───────────────┐
              │  abc::IObject  │            │           │ogawa::OObject │
              │               │            │           │+ O* Schemas   │
              └───────┬───────┘            │           └───────┬───────┘
                      │                    │                    │
                      ▼                    │                    ▼
              ┌───────────────┐            │           ┌───────────────┐
              │ geom::I*      │            │           │ogawa::writer  │
              │(IPolyMesh,etc)│            │           │(OPolyMesh,etc)│
              └───────┬───────┘            │           └───────────────┘
                      │                    │
                      ▼                    │
              ┌───────────────┐            │
              │ogawa::reader  │            │
              │(binary format)│            │
              └───────────────┘            │
```

### 6.3 Key Insight

**Read path**: Well-structured through abc → geom → ogawa layers
**Write path**: Bypasses abc layer entirely - goes directly to ogawa::OArchive

This asymmetry should either be:
1. Documented as intentional (low-level write API)
2. Fixed by implementing abc::OArchive properly

---

## 7. Recommendations

### Priority 1 (Should Fix)

1. **Remove or document PhantomData output stubs in geom/mod.rs**
   - Either remove entirely or add clear documentation that these are placeholders
   - Users should use `ogawa::writer::O*` types directly

2. **Document write API asymmetry**
   - Add clear docs that writing uses `ogawa::OArchive` not `abc::OArchive`
   - Consider deprecating `abc::OArchive` if it won't be completed

### Priority 2 (Code Quality)

3. **Extract schema writer helpers**
   - Create trait for `get_or_create_array/scalar` pattern
   - Reduces ~200 lines of duplicated code

4. **Refactor ICamera to use geom::util**
   - Consistency with other schemas
   - Reduces maintenance burden

5. **Extract indexed UV/normal expansion**
   - Shared by IPolyMesh and ISubD
   - Move to `geom::util::expand_indexed_*`

### Priority 3 (Nice to Have)

6. **Complete TODO in viewport.rs**
   - Calculate actual scene bounds for camera fitting

7. **Review viewer dead code**
   - Some fields are for future features
   - Either implement or document as planned

---

## 8. Files Analyzed

| File | Lines | Issues |
|------|-------|--------|
| src/lib.rs | 62 | Clean |
| src/abc/mod.rs | 1151 | OObject stub issue |
| src/ogawa/writer.rs | ~2100 | Dead code wrappers, duplication |
| src/ogawa/abc_impl.rs | ~900 | Dead Arc fields |
| src/geom/mod.rs | 171 | PhantomData stubs |
| src/geom/polymesh.rs | ~600 | Clean, good util usage |
| src/geom/xform.rs | 563 | Clean |
| src/geom/curves.rs | 426 | Clean |
| src/geom/points.rs | 230 | Clean |
| src/geom/subd.rs | 519 | UV/normal duplication with polymesh |
| src/geom/camera.rs | 599 | Doesn't use util helpers |
| src/geom/faceset.rs | 261 | Clean |
| src/geom/util.rs | 388 | Good shared code |
| src/viewer/*.rs | ~4000 | GPU resource dead code (intentional) |

---

## Appendix: Code Metrics

- **Total Rust files**: 70
- **Total lines**: ~27,162
- **Test files**: 10
- **#[allow(dead_code)]**: 31
- **TODO comments**: 1
- **unwrap() calls**: 70 (18 in tests)
- **expect() calls**: 119 (all in tests)

---

*Report generated: 2026-01-15*

---

## 9. Fixes Applied (2026-01-15)

### 9.1 PhantomData Stubs Removed

**Status**: ✅ FIXED

Removed useless PhantomData structs from `src/geom/mod.rs`:
- `OPolyMesh`, `OXform`, `OCurves`, `OPoints`, `OSubD`, `OCamera`, `OFaceSet`, `ONuPatch`

Added re-exports from `ogawa/writer.rs` where real implementations exist:
```rust
pub use crate::ogawa::writer::{
    OPolyMesh, OPolyMeshSample,
    OXform, OXformSample,
    OCurves, OCurvesSample,
    OPoints, OPointsSample,
    OSubD, OSubDSample,
    OCamera,
    OFaceSet, OFaceSetSample,
    ONuPatch, ONuPatchSample,
    OLight,
};
```

### 9.2 abc::OArchive Fixed

**Status**: ✅ FIXED

- Removed useless `root()` method that returned PhantomData stub
- Added `write_archive()` method delegating to `ogawa::OArchive`
- Added metadata helper methods: `set_app_name()`, `set_date_written()`, `set_description()`, `set_dedup_enabled()`, `dedup_enabled()`

### 9.3 Dead abc Stubs Removed

**Status**: ✅ FIXED

Removed from `src/abc/mod.rs`:
- `OObject<'a>` - PhantomData stub (~20 lines)
- `OCompoundProperty<'a>` - PhantomData stub
- `OScalarProperty<'a>` - PhantomData stub  
- `OArrayProperty<'a>` - PhantomData stub

### 9.4 Code Deduplication in ogawa/writer.rs

**Status**: ✅ FIXED

Added shared helper methods to `OProperty`:
```rust
impl OProperty {
    pub fn get_or_create_array_child(&mut self, name: &str, dt: DataType) -> &mut OProperty;
    pub fn get_or_create_scalar_child(&mut self, name: &str, dt: DataType) -> &mut OProperty;
}
```

Removed duplicate `get_or_create_array()` methods from:
- `OCurves` (~12 lines)
- `OPoints` (~12 lines)
- `OSubD` (~12 lines)
- `ONuPatch` (~12 lines)
- `OFaceSet` (~12 lines)

**Lines saved**: ~60 lines of duplicated code

`OPolyMesh` kept special `get_or_create_array_with_ts()` for time_sampling_index support.

### 9.5 lib.rs Prelude Updated

**Status**: ✅ FIXED

Updated prelude to export `ogawa::OObject` instead of removed `abc::OObject`.

---

## 10. Remaining Items

| Item | Status | Priority |
|------|--------|----------|
| Extract UV/normal expansion (IPolyMesh/ISubD) | Not started | Medium |
| Refactor ICamera to use geom::util | Not started | Low |
| Viewport TODO (scene bounds) | Not started | Low |
| Review viewer dead code | Documented | Info |

---

*Fixes applied: 2026-01-15*
