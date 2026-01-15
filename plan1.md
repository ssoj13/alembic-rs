# Alembic-RS Parity Report

## Executive Summary

**Project**: alembic-rs - Rust port of Alembic (.abc) 3D interchange format  
**Reference**: C++ Alembic library v1.8.x (`_ref/alembic/`)  
**Analysis Date**: 2026-01-14  
**Status**: Production-ready for reading, partial writing support

### Key Findings

| Category | Status | Notes |
|----------|--------|-------|
| Ogawa Backend | **FULL** | Read/Write with deduplication |
| HDF5 Backend | **NOT IMPLEMENTED** | Not planned (legacy format) |
| AbcCoreLayer | **NOT IMPLEMENTED** | Missing feature for compositing |
| Geometry Reading | **FULL** | All schema types supported |
| Geometry Writing | **STUBS** | O* types are PhantomData |
| Viewer | **ADDITIONAL** | wgpu renderer (not in original) |
| Python Bindings | **PARTIAL** | PyO3 bindings available |

---

## 1. Module-by-Module Parity Analysis

### 1.1 Core Module (`src/core/`)

| Component | Reference | alembic-rs | Status |
|-----------|-----------|------------|--------|
| DataType | AbcCoreAbstract/DataType.h | `core/mod.rs` | FULL |
| TimeSampling | AbcCoreAbstract/TimeSampling.h | `core/mod.rs` | FULL |
| MetaData | AbcCoreAbstract/MetaData.h | `core/mod.rs` | FULL |
| ArchiveReader trait | AbcCoreAbstract/ArchiveReader.h | `core/traits.rs` | FULL |
| ArchiveWriter trait | AbcCoreAbstract/ArchiveWriter.h | `core/traits.rs` | FULL |
| ObjectReader trait | AbcCoreAbstract/ObjectReader.h | `core/traits.rs` | FULL |
| PropertyReader trait | AbcCoreAbstract/ReadArraySampleCache.h | `core/traits.rs` | FULL |

**Notes**: Core traits are well-designed and match C++ abstract interfaces.

### 1.2 Ogawa Backend (`src/ogawa/`)

| Component | Reference | alembic-rs | Status |
|-----------|-----------|------------|--------|
| OgawaReader | AbcCoreOgawa/ReadUtil.cpp | `ogawa/reader.rs` | FULL |
| OgawaWriter | AbcCoreOgawa/WriteUtil.cpp | `ogawa/writer.rs` | FULL |
| Group reading | AbcCoreOgawa/CprData.cpp | `ogawa/reader.rs` | FULL |
| Sample deduplication | AbcCoreOgawa/SpookyHash.cpp | `util/hash.rs` | FULL |
| Stream abstraction | AbcCoreOgawa/StreamManager.cpp | `ogawa/stream.rs` | FULL |

**Version**: ALEMBIC_LIBRARY_VERSION = 10709 (matches 1.7.9)

**Notes**: Ogawa implementation is complete with SpookyHash and Murmur3 for deduplication.

### 1.3 High-Level API (`src/abc/`)

| Component | Reference | alembic-rs | Status |
|-----------|-----------|------------|--------|
| IArchive | Abc/IArchive.h | `abc/mod.rs` | FULL |
| OArchive | Abc/OArchive.h | `abc/mod.rs` | FULL |
| IObject | Abc/IObject.h | `abc/mod.rs` | FULL |
| OObject | Abc/OObject.h | `abc/mod.rs` | PARTIAL |
| ICompoundProperty | Abc/ICompoundProperty.h | `abc/mod.rs` | FULL |
| IScalarProperty | Abc/IScalarProperty.h | `abc/mod.rs` | FULL |
| IArrayProperty | Abc/IArrayProperty.h | `abc/mod.rs` | FULL |
| TypedScalarProperty | Abc/ITypedScalarProperty.h | `abc/mod.rs` | FULL |
| TypedArrayProperty | Abc/ITypedArrayProperty.h | `abc/mod.rs` | FULL |

**File**: `src/abc/mod.rs` - 1128 lines, well-structured

### 1.4 Geometry Schemas (`src/geom/`)

| Schema | Reference | alembic-rs Read | alembic-rs Write | Status |
|--------|-----------|-----------------|------------------|--------|
| PolyMesh | AbcGeom/IPolyMesh.h | `IPolyMesh` | `OPolyMesh` (stub) | READ-ONLY |
| Xform | AbcGeom/IXform.h | `IXform` | `OXform` (stub) | READ-ONLY |
| Curves | AbcGeom/ICurves.h | `ICurves` | `OCurves` (stub) | READ-ONLY |
| Points | AbcGeom/IPoints.h | `IPoints` | `OPoints` (stub) | READ-ONLY |
| SubD | AbcGeom/ISubD.h | `ISubD` | `OSubD` (stub) | READ-ONLY |
| Camera | AbcGeom/ICamera.h | `ICamera` | `OCamera` (stub) | READ-ONLY |
| Light | AbcGeom/ILight.h | `ILight` | `OLight` (stub) | READ-ONLY |
| NuPatch | AbcGeom/INuPatch.h | `INuPatch` | `ONuPatch` (stub) | READ-ONLY |
| FaceSet | AbcGeom/IFaceSet.h | `IFaceSet` | `OFaceSet` (stub) | READ-ONLY |
| GeomBase | AbcGeom/IGeomBase.h | `IGeomBase` | `OGeomBase` (stub) | READ-ONLY |

**Critical**: All O* types are `PhantomData` stubs - geometry writing not implemented!

### 1.5 Xform Operations (`src/geom/xform.rs`)

| Operation | Reference | Status |
|-----------|-----------|--------|
| Scale | XformOp::kScaleOperation | FULL |
| Translate | XformOp::kTranslateOperation | FULL |
| RotateX/Y/Z | XformOp::kRotate*Operation | FULL |
| RotateXYZ | XformOp::kRotateOperation | FULL |
| Matrix | XformOp::kMatrixOperation | FULL |
| Matrix composition | XformSample::getMatrix() | FULL |

**File**: `src/geom/xform.rs` - 561 lines, complete implementation

### 1.6 Material Module (`src/material/`)

| Component | Reference | Status |
|-----------|-----------|--------|
| IMaterial | AbcMaterial/IMaterial.h | FULL |
| MaterialSchema | AbcMaterial/MaterialSchema.h | FULL |
| MaterialAssignment | AbcMaterial/MaterialAssignment.h | FULL |

### 1.7 Collection Module (`src/collection/`)

| Component | Reference | Status |
|-----------|-----------|--------|
| ICollections | AbcCollection/ICollections.h | FULL |
| CollectionSchema | AbcCollection/CollectionSchema.h | FULL |

---

## 2. Missing Features (vs Reference)

### 2.1 AbcCoreHDF5 - NOT PLANNED

**Reference**: `_ref/alembic/lib/Alembic/AbcCoreHDF5/`

HDF5 is the legacy backend, replaced by Ogawa. Most modern pipelines use Ogawa exclusively.

**Recommendation**: No action needed - Ogawa is the standard.

### 2.2 AbcCoreLayer - MISSING

**Reference**: `_ref/alembic/lib/Alembic/AbcCoreLayer/`

AbcCoreLayer provides:
- Archive layering (compositing multiple .abc files)
- Non-destructive overrides
- USD-style composition

**Impact**: Cannot layer/composite archives  
**Recommendation**: Consider implementing for USD interop workflows

### 2.3 Output Geometry Types - STUBS

All `O*` types in `src/geom/mod.rs` are defined as:
```rust
pub struct OPolyMesh<'a>(std::marker::PhantomData<&'a ()>);
pub struct OXform<'a>(std::marker::PhantomData<&'a ()>);
// ... etc
```

**Impact**: Cannot write geometry data to .abc files  
**Recommendation**: Implement using existing `OgawaWriter` infrastructure

---

## 3. Dead Code Analysis

### 3.1 `#[allow(dead_code)]` Markers

Found **35 instances** across the codebase:

| Module | Count | Notes |
|--------|-------|-------|
| viewer/renderer.rs | 15 | Pipeline struct fields |
| viewer/mesh_converter.rs | 8 | Internal cache fields |
| standard-surface crate | 6 | Shader parameters |
| ogawa/writer.rs | 4 | Internal state |
| abc/mod.rs | 2 | Reserved fields |

**Most Common Pattern**:
```rust
#[allow(dead_code)]
struct Pipeline {
    // Fields used only for GPU resource lifetime
    bind_group_layout: wgpu::BindGroupLayout,
    // ...
}
```

**Recommendation**: Most are legitimate (GPU resource ownership). Review viewer module fields.

### 3.2 TODO/FIXME Comments

Only **1 TODO** found in entire codebase:
```rust
// src/ogawa/reader.rs - minor optimization note
```

**Recommendation**: Codebase is clean, no action needed.

---

## 4. Viewer Module Analysis

### 4.1 Overview

The viewer module (`src/viewer/`) is **ADDITIONAL FUNCTIONALITY** not present in C++ Alembic.

| Component | Lines | Status |
|-----------|-------|--------|
| renderer.rs | 1246 | Production-ready |
| mesh_converter.rs | 882 | Production-ready |
| smooth_normals.rs | 71 | Working |
| camera.rs | ~200 | Working |
| mod.rs | 89 | Clean exports |

### 4.2 Render Pipeline

**7 GPU Pipelines**:
1. `solid_pipeline` - Standard Surface shading
2. `wireframe_pipeline` - Edge overlay
3. `line_pipeline` - Curves rendering
4. `point_pipeline` - Points rendering
5. `shadow_pipeline` - Depth-only shadow pass
6. `skybox_pipeline` - Environment background
7. `debug_pipeline` - Development aids

### 4.3 Standard Surface Shader

**Location**: `crates/standard-surface/`

MaterialX-compatible PBR shader with:
- Base color + metallic
- Roughness + specular
- Normal mapping
- Environment reflection
- Shadow receiving

### 4.4 Smooth Normals

**File**: `src/viewer/smooth_normals.rs`

```rust
pub fn compute_smooth_normals(
    positions: &[f32],
    indices: &[u32],
    angle_threshold: f32,  // radians
) -> Vec<f32>
```

- Angle-weighted normal averaging
- Configurable threshold for hard edges
- Works correctly with indexed meshes

### 4.5 Potential Issues

1. **Memory**: Large meshes may exceed GPU memory - no LOD system
2. **Animation**: Mesh cache doesn't handle time-varying topology well
3. **Dead code**: ~15 struct fields marked as dead but needed for lifetime

---

## 5. Performance Considerations

### 5.1 Strengths

- **Parallel mesh conversion**: Uses rayon for multi-threaded processing
- **Sample deduplication**: SpookyHash prevents duplicate data
- **Mesh caching**: MeshCache prevents redundant GPU uploads
- **Zero-copy reading**: Ogawa reader uses memory-mapped files

### 5.2 Potential Bottlenecks

| Area | Issue | Recommendation |
|------|-------|----------------|
| Large archives | No streaming API | Add lazy loading |
| Animation playback | Full sample read per frame | Add sample caching |
| Multi-file scenes | No AbcCoreLayer | Consider implementing |

---

## 6. API Compatibility

### 6.1 Naming Conventions - **CRITICAL ISSUE**

**PROBLEM**: Method names do NOT match C++ API. This breaks parity!

| C++ Reference | alembic-rs | Status |
|---------------|------------|--------|
| `getTop()` | `root()` | **WRONG** |
| `getName()` | `name()` | **WRONG** |
| `getFullName()` | `full_name()` | **WRONG** |
| `getNumChildren()` | `num_children()` | **WRONG** |
| `getChild()` | `child()` | **WRONG** |
| `getHeader()` | `header()` | **WRONG** |
| `getParent()` | missing | **MISSING** |
| `getArchive()` | missing | **MISSING** |

**Required Fix**: Rename ALL methods to match C++ API exactly:
```rust
// WRONG (current)
pub fn root(&self) -> IObject
pub fn name(&self) -> &str
pub fn num_children(&self) -> usize

// CORRECT (should be)
pub fn getTop(&self) -> IObject
pub fn getName(&self) -> &str  
pub fn getNumChildren(&self) -> usize
```

**Impact**: High - anyone familiar with C++ Alembic API cannot use this library intuitively.

### 6.2 Error Handling

C++ uses exceptions, Rust uses `Result<T, AlembicError>`:
```rust
pub enum AlembicError {
    Io(std::io::Error),
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidGroup,
    PropertyNotFound(String),
    // ...
}
```

Good idiomatic Rust error handling throughout.

---

## 7. Recommendations

### 7.1 CRITICAL Priority

1. **FIX API METHOD NAMES** - Must match C++ exactly:
   - `root()` → `getTop()`
   - `name()` → `getName()`
   - `full_name()` → `getFullName()`
   - `num_children()` → `getNumChildren()`
   - `child()` → `getChild()`
   - `header()` → `getHeader()`
   - Add missing: `getParent()`, `getArchive()`

### 7.2 High Priority

1. **Implement O* geometry types** - Critical for full read/write parity
2. **Review viewer dead code** - Clean up or document GPU lifetime requirements

### 7.3 Medium Priority

1. **Consider AbcCoreLayer** - Needed for composition workflows
2. **Add streaming API** - For very large archives
3. **Sample interpolation** - Currently floor-only, add lerp

### 7.4 Low Priority

1. **HDF5 support** - not needed
2. **More comprehensive tests** - Add property round-trip tests
3. **Benchmarks** - Compare with C++ implementation

---

## 8. Conclusion

**alembic-rs** is a well-structured Rust port of Alembic with:

**Strengths**:
- Complete Ogawa read support
- Clean Rust API design
- Bonus wgpu viewer
- Good error handling
- Parallel processing

**Gaps**:
- O* geometry types are stubs (no geometry writing)
- No AbcCoreLayer (no composition)
- No HDF5 (legacy, acceptable)

**Overall Assessment**: **Production-ready for reading**, partial writing support. Suitable for viewers, converters, and read-only pipelines. Not yet suitable for applications that need to author .abc geometry.

---

## Appendix A: File Inventory

```
src/
├── abc/mod.rs          (1128 lines) - High-level API
├── core/
│   ├── mod.rs          (37 lines)   - Core exports
│   └── traits.rs       (492 lines)  - Abstract traits
├── geom/
│   ├── mod.rs          (170 lines)  - Geometry exports
│   └── xform.rs        (561 lines)  - Xform implementation
├── ogawa/
│   ├── reader.rs       (~800 lines) - Ogawa reader
│   └── writer.rs       (~600 lines) - Ogawa writer
├── material/mod.rs     (40 lines)   - Material schema
├── collection/mod.rs   (~50 lines)  - Collections
├── util/
│   └── hash.rs         (~200 lines) - SpookyHash, Murmur3
├── viewer/
│   ├── mod.rs          (89 lines)   - Viewer exports
│   ├── renderer.rs     (1246 lines) - wgpu renderer
│   ├── mesh_converter.rs (882 lines)- ABC→GPU conversion
│   └── smooth_normals.rs (71 lines) - Normal calculation
└── lib.rs              (~50 lines)  - Crate root
```

## Appendix B: Reference Comparison

```
_ref/alembic/lib/Alembic/
├── Abc/                 → src/abc/           FULL
├── AbcCoreAbstract/     → src/core/          FULL  
├── AbcCoreOgawa/        → src/ogawa/         FULL
├── AbcCoreHDF5/         → (not implemented)  NOT PLANNED
├── AbcCoreLayer/        → (not implemented)  MISSING
├── AbcGeom/             → src/geom/          READ-ONLY
├── AbcMaterial/         → src/material/      FULL
└── AbcCollection/       → src/collection/    FULL
```
