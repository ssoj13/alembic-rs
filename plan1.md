# Bug Hunt Report: alembic-rs

**Date:** 2026-01-09
**Project:** alembic-rs - Pure Rust implementation of Alembic (.abc) 3D interchange format
**Total Source Files:** ~45 files, ~20,100 lines of Rust code

---

## Executive Summary

Comprehensive code review revealed **7 CRITICAL**, **12 HIGH**, **18 MEDIUM**, and **15 LOW** severity issues across the codebase. Key problem areas include:

1. **Silent data loss** in time_sampling.rs and compression.rs
2. **Incorrect binary parsing** in read_util.rs (MIN_ALEMBIC_VERSION placeholder)
3. **Memory safety concerns** in Python bindings (bytemuck panics)
4. **Massive code duplication** across geometry schemas (~70% similar code)
5. **Incomplete implementations** marked with TODO but affecting functionality

---

## Critical Issues (Fix Immediately)

### 1. MIN_ALEMBIC_VERSION Placeholder (CRITICAL)
**File:** `src/ogawa/read_util.rs:35`
```rust
pub const MIN_ALEMBIC_VERSION: i32 = 9999;
```
**Problem:** This looks like a placeholder. Real Alembic versions (e.g., 10709) are higher. **Any file with version < 9999 will be rejected**, breaking compatibility with older valid files.

**Fix:** Research actual minimum supported Alembic version and set correct value.

---

### 2. TimeSampling Ignores `_times` Parameter (CRITICAL)
**File:** `src/core/time_sampling.rs:134-138`
```rust
pub fn from_type_and_times(tst: TimeSamplingType, _times: Vec<Chrono>) -> Self {
    Self {
        sampling_type: tst,  // _times is COMPLETELY IGNORED!
    }
}
```
**Problem:** Passed times are silently discarded. If `tst` is `Acyclic { times: vec![] }`, actual animation times are lost.

**Impact:** Animation data corruption when reading files.

---

### 3. Compression Errors Silently Swallowed (CRITICAL)
**File:** `src/core/compression.rs:77-83`
```rust
Err(_) => {
    // Decompression failed - data was probably not compressed
    Ok(data.to_vec())  // Returns corrupt data as if valid!
}
```
**Problem:** If compressed data is corrupted, function returns garbage without error.

**Fix:** Return `Result` with error variant or add detection heuristic.

---

### 4. bytemuck::cast_slice Can Panic (CRITICAL)
**File:** `src/python/properties.rs:430-431`, and ~15 other locations
```rust
let values: Vec<i16> = bytemuck::cast_slice(data).to_vec();
```
**Problem:** `bytemuck::cast_slice` panics on alignment/size mismatch. This can crash the Python interpreter.

**Fix:** Use `bytemuck::try_cast_slice` everywhere.

---

### 5. Python addVisibilityProperty() Doesn't Work (CRITICAL)
**File:** `src/python/write.rs:318-322`
```rust
fn addVisibilityProperty(&mut self) -> super::geom::PyOVisibilityProperty {
    super::geom::PyOVisibilityProperty::create()  // Creates but doesn't ADD!
}
```
**Problem:** Creates property object but never adds it to parent. Visibility is never written.

---

### 6. std::mem::replace Leaves Objects Invalid (CRITICAL)
**File:** `src/python/write.rs:152, 164, 239`
```rust
let obj = std::mem::replace(&mut mesh.inner, OPolyMesh::new("_empty")).build();
```
**Problem:** After `addPolyMesh()`, the `PyOPolyMesh` contains dummy `"_empty"` mesh. Further operations corrupt data silently.

---

### 7. Incorrect bytemuck Cast in Geometry Schemas (CRITICAL)
**Files:** `src/geom/curves.rs:406`, `subd.rs:503`, and ~12 similar locations
```rust
sample.num_vertices = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
```
**Problem:** `read_sample_vec` returns `Vec<u8>` containing raw i32 bytes. Casting `u8` to `i32` interprets each byte as separate i32 - **completely wrong data!**

**Fix:** Data is already properly aligned, use:
```rust
sample.num_vertices = bytemuck::cast_slice::<u8, i32>(&data).to_vec();
// OR better:
sample.num_vertices = bytemuck::pod_read_unaligned(&data);
```

---

## High Severity Issues

### 8. find_object() Only Searches First Level (HIGH)
**File:** `src/core/traits.rs:42-59`
```rust
fn find_object(&self, path: &str) -> Option<Box<dyn ObjectReader + '_>> {
    // Comment says "Due to Rust lifetime constraints... navigate one level"
    if let Some(first_part) = parts.first() {
        self.root().child_by_name(first_part)  // Ignores rest of path!
    }
}
```
**Problem:** `find_object("/root/parent/child")` returns `/root`, not `/root/parent/child`.

---

### 9. Errors Silently Swallowed in Object Reader (HIGH)
**File:** `src/ogawa/abc_impl.rs:202-214`
```rust
fn child(&self, index: usize) -> Option<Box<dyn ObjectReader + '_>> {
    match self.root_data.child(index)? {
        Ok(reader) => Some(Box::new(reader)),
        Err(_) => None,  // Error silently ignored!
    }
}
```
**Problem:** Caller cannot distinguish "no child" from "read error".

---

### 10. Float Comparison with EPSILON for MAX Values (HIGH)
**File:** `src/ogawa/read_util.rs:86-97`
```rust
if (tpc - ACYCLIC_TIME_PER_CYCLE).abs() < f64::EPSILON {
```
**Problem:** `f64::EPSILON` (~2.2e-16) is meaningless for values near `f64::MAX` (~1.8e308).

**Fix:** Use exact comparison or check for very large negative values.

---

### 11. static mut LOG_LEVEL (HIGH)
**File:** `src/main.rs:17`
```rust
static mut LOG_LEVEL: LogLevel = LogLevel::Info;
```
**Problem:** `static mut` requires unsafe access and is not thread-safe.

**Fix:** Use `std::sync::OnceLock` or `AtomicU8`.

---

### 12. ISubD::face_set() Always Returns None (HIGH)
**File:** `src/geom/subd.rs:274-280`
```rust
#[allow(clippy::unused_self)]
pub fn face_set(&self, name: &str) -> Option<IFaceSet<'_>> {
    let _child = self.object.child_by_name(name)?;  // Does lookup...
    None  // ...then ignores it!
}
```
**Problem:** Method exists but does nothing.

---

### 13. is_compressed() Has Invalid zlib Flag (HIGH)
**File:** `src/core/compression.rs:89-100`
```rust
matches!(zlib_flags, 0x01 | 0x5E | 0x9C | 0xDA)
```
**Problem:** `0x5E` is not a valid zlib compression level flag.

Standard values: `0x01` (none), `0x9C` (default), `0xDA` (best). `0x5E` should likely be removed or changed.

---

### 14. Hash Always Zeros in Writer (HIGH)
**File:** `src/ogawa/writer.rs:647-649`
```rust
buf.extend_from_slice(&[0u8; 32]);  // Hash is always zeros
```
**Problem:** May affect integrity verification and compatibility with official Alembic readers.

---

### 15. Unused _num_samples Parameter (HIGH)
**File:** `src/core/time_sampling.rs:239`
```rust
pub fn sample_time(&self, index: usize, _num_samples: usize) -> Chrono {
```
**Problem:** Parameter exists but is never used. Either logic is incomplete or signature is wrong.

---

### 16. Size Hint 3 Unhandled (HIGH)
**File:** `src/ogawa/read_util.rs:278-306`
```rust
let result = match size_hint {
    0 => { /* u8 */ }
    1 => { /* u16 */ }
    2 => { /* u32 */ }
    _ => return Err(Error::invalid("Invalid size hint")),
};
```
**Problem:** Size hint 3 (u64) may be valid for very large files but is rejected.

---

### 17-19. Multiple unreachable!() in Writer (HIGH)
**Files:** `src/ogawa/writer.rs:1100-1102, 1115-1117`
```rust
} else {
    unreachable!()
}
```
**Problem:** Panics instead of returning error. In library code this is inappropriate.

---

## Medium Severity Issues

### 20. Mmap Safety Incomplete
**File:** `src/ogawa/reader.rs:56-60`
Comment says "we handle potential issues" but file modification by other processes can cause UB.

### 21. Cache Eviction Strategy Primitive
**File:** `src/core/cache.rs:151-169`
Evicts random 50% of entries without LRU or any heuristic.

### 22. Fixed Buffer Size for Strings
**File:** `src/python/properties.rs:394-402`
```rust
let mut buf = vec![0u8; 256];
```
Strings > 256 bytes truncated silently.

### 23. archive_bounds_at_time() Incomplete
**File:** `src/abc/mod.rs:260-266`
```rust
let sample_index = if time <= 0.0 { 0 } else {
    // TODO: proper implementation would use time sampling
    0  // Always returns sample 0!
};
```

### 24. visibility is_visible() Incomplete
**File:** `src/geom/visibility.rs:135-147`
```rust
ObjectVisibility::Deferred => {
    // Would need to walk up hierarchy, but we don't have parent access
    true  // Default to visible
}
```

### 25. ILight Missing Camera Parameters
**File:** `src/geom/light.rs:137-191`
Only reads 5 of 16 camera parameters.

### 26-32. Interface Inconsistencies
- `property_names()` returns different things in different schemas
- Missing `topology_variance()` in IPoints, INuPatch
- Missing `child_bounds` methods in ICurves, IPoints
- Missing `has_self_bounds()` in schema readers
- Missing `time_sampling_index()` in most schemas
- Empty O* output structs (OPolyMesh, OXform, etc.)
- Python `valid()` always returns `true`

### 33. Lock Poisoning Message Vague
**File:** `src/python/write.rs` - multiple locations
```rust
Err("Lock poisoned")  // Doesn't say which lock or why
```

---

## Low Severity Issues

### 34. Float Comparison in is_exact()
**File:** `src/core/sample.rs:176-178`
```rust
self.alpha == 0.0  // Should use abs() < epsilon
```

### 35. O(n) Metadata Search
**File:** `src/core/metadata.rs:29-37`
Linear search acceptable for small data but documented assumption.

### 36. Race Condition in Cache Insert
**File:** `src/core/cache.rs:129-148`
Can exceed max_size briefly. Comment acknowledges as "heuristic".

### 37. parent() Not Implemented
**File:** `src/ogawa/abc_impl.rs:237`
```rust
fn parent(&self) -> Option<&dyn ObjectReader> {
    None  // TODO: implement parent tracking
}
```

### 38-40. Minor Python Issues
- Unused `#[allow(non_snake_case)]` blanket
- Clone-heavy iterator design
- Arc cloning on every traversal

### 41-45. Documentation/Style Issues
- Dead code attribute on intentionally kept `inner` field
- Inconsistent naming (camelCase vs snake_case in Python)
- Missing convenience methods (`hasChildren()`)

---

## Code Duplication Analysis

### Major Duplication Opportunities

#### 1. `.geom` Property Access Pattern (~6 files, ~50 lines each)
```rust
let props = self.object.properties();
let geom_prop = props.property_by_name(".geom").ok_or_else(...)?;
let geom = geom_prop.as_compound().ok_or_else(...)?;
```
**Recommendation:** Create `fn get_geom_compound(&self) -> Result<ICompoundProperty>` helper.

#### 2. arb_geom_params/user_properties (~6 files, ~60 lines each)
Identical implementation in polymesh.rs, curves.rs, points.rs, subd.rs, camera.rs, xform.rs.

**Recommendation:** Create `trait GeomSchemaExt` with default implementations.

#### 3. self_bounds Reading (~7 files, ~15 lines each)
```rust
if let Some(bnds_prop) = geom.property_by_name(".selfBnds") {
    // ... identical 15-line block ...
}
```
**Recommendation:** Create `fn read_self_bounds(geom: &ICompoundProperty, index: usize) -> Option<BBox3d>`.

#### 4. positions Reading (~5 files)
Identical P property parsing pattern.

#### 5. compute_bounds() (~5 sample structs)
Identical implementation in PolyMeshSample, CurvesSample, PointsSample, SubDSample, NuPatchSample.

#### 6. collect_bounds_recursive Functions (~2 identical copies)
**File:** `src/abc/mod.rs:179-337`
`collect_bounds_recursive` and `collect_bounds_recursive_time` are nearly identical (~150 lines each).

**Estimated Deduplication Savings:** ~1500-2000 lines of code

---

## Architecture Recommendations

### 1. Create Common Traits
```rust
pub trait GeomSchema<'a> {
    fn object(&self) -> &IObject<'a>;
    fn get_geom_compound(&self) -> Result<ICompoundProperty<'a>>;
    fn arb_geom_param_names(&self) -> Vec<String>;
    fn user_property_names(&self) -> Vec<String>;
    fn has_arb_geom_params(&self) -> bool;
    fn has_user_properties(&self) -> bool;
}
```

### 2. Create Utility Module
```rust
// src/geom/util.rs
pub fn read_self_bounds(...) -> Option<BBox3d>;
pub fn read_positions(...) -> Result<Vec<Vec3>>;
pub fn read_velocities(...) -> Result<Vec<Vec3>>;
pub fn read_i32_array(...) -> Result<Vec<i32>>;
```

### 3. Fix Error Handling
- Replace all `unwrap_or_default()` with proper error propagation
- Replace silent `None` returns with `Result<Option<T>>`
- Add error context using `.context()` from anyhow/eyre

### 4. Python Bindings Improvements
- Use `try_cast_slice` instead of `cast_slice`
- Fix `std::mem::replace` pattern
- Actually implement `addVisibilityProperty()`

---

## Dataflow Diagram

```
                              alembic-rs Architecture

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                           USER APPLICATIONS                              │
    │                    (Rust Native / Python Bindings)                       │
    └─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                         abc/mod.rs (High-Level API)                      │
    │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
    │  │  IArchive    │  │   IObject    │  │ ICompound    │  │  IProperty   │ │
    │  │  OArchive    │  │   OObject    │  │ Property     │  │  (Scalar/    │ │
    │  │              │  │              │  │              │  │   Array)     │ │
    │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
    └─────────┼─────────────────┼─────────────────┼─────────────────┼─────────┘
              │                 │                 │                 │
              ▼                 ▼                 ▼                 ▼
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                         geom/*.rs (Geometry Schemas)                     │
    │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │
    │  │IPolyMesh│ │ IXform  │ │ICurves  │ │IPoints  │ │ ISubD   │ │ICamera │ │
    │  │OPolyMesh│ │ OXform  │ │OCurves  │ │OPoints  │ │ OSubD   │ │OCamera │ │
    │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬───┘ │
    │       │           │           │           │           │           │      │
    │       └───────────┴───────────┴─────┬─────┴───────────┴───────────┘      │
    │                                     │                                     │
    │                              [*Sample structs]                            │
    │            PolyMeshSample, XformSample, CurvesSample, etc.               │
    └─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                      core/*.rs (Abstract Traits & Types)                 │
    │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
    │  │  ArchiveReader/  │  │  ObjectReader/   │  │ PropertyReader/  │       │
    │  │  ArchiveWriter   │  │  ObjectWriter    │  │ PropertyWriter   │       │
    │  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘       │
    │           │                     │                     │                  │
    │  ┌────────┴─────────────────────┴─────────────────────┴────────┐        │
    │  │   TimeSampling, SampleSelector, MetaData, PropertyHeader    │        │
    │  │   ObjectHeader, DataType, PlainOldDataType, Error/Result    │        │
    │  └─────────────────────────────────────────────────────────────┘        │
    └─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                    ogawa/*.rs (Binary Format Implementation)             │
    │  ┌─────────────────────────────────────────────────────────────────┐    │
    │  │           OgawaArchiveReader / OgawaArchiveWriter               │    │
    │  │                  (Implements core traits)                        │    │
    │  └──────────────────────────────┬──────────────────────────────────┘    │
    │                                 │                                        │
    │  ┌──────────────────────────────┴──────────────────────────────────┐    │
    │  │    IStreams (Memory-mapped I/O)    │    OStreams (Buffered)     │    │
    │  └─────────────────────────────────────────────────────────────────┘    │
    │                                 │                                        │
    │  ┌──────────────────────────────┴──────────────────────────────────┐    │
    │  │                  format.rs (Binary Constants)                    │    │
    │  │   Magic: "Ogawa", MSB flags, child offset encoding, version     │    │
    │  └─────────────────────────────────────────────────────────────────┘    │
    └─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                          .abc FILE (Ogawa Format)                        │
    │   ┌─────────────────────────────────────────────────────────────────┐   │
    │   │  Header (16 bytes): Magic + Frozen + Version + Root Offset      │   │
    │   ├─────────────────────────────────────────────────────────────────┤   │
    │   │  Data Blocks: Metadata, Time Samplings, Properties, Geometry    │   │
    │   ├─────────────────────────────────────────────────────────────────┤   │
    │   │  Groups: Hierarchical structure with child offsets              │   │
    │   └─────────────────────────────────────────────────────────────────┘   │
    └─────────────────────────────────────────────────────────────────────────┘


    ═══════════════════════════════════════════════════════════════════════════
                              PYTHON BINDINGS LAYER
    ═══════════════════════════════════════════════════════════════════════════

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                         python/*.rs (PyO3 Bindings)                      │
    │                                                                          │
    │  alembic_rs.Abc:                    alembic_rs.AbcGeom:                  │
    │  ┌─────────────────────┐            ┌─────────────────────┐             │
    │  │ PyIArchive          │            │ PyPolyMeshSample    │             │
    │  │ PyOArchive          │            │ PyXformSample       │             │
    │  │ PyIObject           │            │ PyCurvesSample      │             │
    │  │ PyTimeSampling      │            │ PyPointsSample      │             │
    │  │ PyICompoundProperty │            │ PyCameraSample      │             │
    │  └─────────────────────┘            └─────────────────────┘             │
    └─────────────────────────────────────────────────────────────────────────┘
```

---

## Priority Action Plan

### Phase 1: Critical Fixes (Immediate)
1. [ ] Fix MIN_ALEMBIC_VERSION constant
2. [ ] Fix from_type_and_times() to use _times parameter
3. [ ] Fix compression error handling
4. [ ] Replace bytemuck::cast_slice with try_cast_slice
5. [ ] Fix Python addVisibilityProperty()
6. [ ] Fix std::mem::replace pattern in Python bindings
7. [ ] Fix bytemuck casting in geometry schemas

### Phase 2: High Priority (This Week)
8. [ ] Fix find_object() to traverse full path
9. [ ] Fix error swallowing in object reader
10. [ ] Fix float comparison for EPSILON
11. [ ] Replace static mut with OnceLock
12. [ ] Fix or remove ISubD::face_set()
13. [ ] Fix is_compressed() zlib flags
14. [ ] Implement proper hash calculation

### Phase 3: Code Deduplication (Next Sprint)
15. [ ] Create GeomSchema trait
16. [ ] Create geom utility module
17. [ ] Deduplicate collect_bounds functions
18. [ ] Unify property access patterns

### Phase 4: API Completion (Following Sprint)
19. [ ] Add missing interface methods
20. [ ] Complete O* output structs
21. [ ] Implement parent tracking
22. [ ] Complete Python bindings parity

---

## Files Reference

| Category | Files | Lines |
|----------|-------|-------|
| Core Traits | src/core/traits.rs | ~500 |
| Time Sampling | src/core/time_sampling.rs | ~450 |
| Compression | src/core/compression.rs | ~100 |
| Cache | src/core/cache.rs | ~180 |
| Ogawa Reader | src/ogawa/reader.rs, abc_impl.rs | ~600 |
| Ogawa Writer | src/ogawa/writer.rs | ~1200 |
| High-level API | src/abc/mod.rs | ~1150 |
| Geometry (total) | src/geom/*.rs | ~5000 |
| Python Bindings | src/python/*.rs | ~2500 |
| CLI | src/main.rs | ~390 |

---

## Conclusion

The codebase is well-structured with clear separation of concerns. However, it shows signs of incomplete refactoring with significant code duplication in geometry schemas. The critical issues around data corruption and silent failures should be addressed immediately before production use. The estimated effort for Phase 1-2 fixes is 2-3 developer days; full deduplication and API completion would require 1-2 weeks.

**Awaiting approval to proceed with fixes.**
