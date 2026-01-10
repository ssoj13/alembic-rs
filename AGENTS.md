# AGENTS.md - Alembic-rs Architecture & Dataflow

## Project Overview

**alembic-rs** is a pure Rust implementation of the Alembic (.abc) 3D interchange format used in VFX pipelines.

- **Reading:** Fully implemented for Ogawa binary format
- **Writing:** Implemented with deduplication support
- **Python Bindings:** Available via PyO3

---

## Module Architecture

```
alembic-rs/
├── src/
│   ├── lib.rs              # Library root, module declarations, prelude
│   ├── main.rs             # CLI tool (info, tree, stats commands)
│   │
│   ├── util/               # Fundamental types
│   │   ├── pod.rs          # PlainOldDataType enum (14 types)
│   │   ├── data_type.rs    # DataType (POD + extent)
│   │   ├── error.rs        # Error/Result types
│   │   ├── math.rs         # glam re-exports (Vec3, Mat4, etc.)
│   │   └── dimensions.rs   # Dimension helpers
│   │
│   ├── core/               # Abstract layer
│   │   ├── traits.rs       # ArchiveReader/Writer, ObjectReader/Writer, PropertyReader/Writer
│   │   ├── header.rs       # ObjectHeader, PropertyHeader
│   │   ├── time_sampling.rs# TimeSampling (Identity/Uniform/Cyclic/Acyclic)
│   │   ├── sample.rs       # SampleSelector, GeometryScope
│   │   ├── metadata.rs     # MetaData key-value storage
│   │   ├── cache.rs        # ArraySampleCache (LRU-ish)
│   │   └── compression.rs  # zlib decompression
│   │
│   ├── ogawa/              # Binary format implementation
│   │   ├── format.rs       # Constants (magic, flags, version)
│   │   ├── reader.rs       # IStreams (mmap or buffered)
│   │   ├── writer.rs       # OArchive, OObject, OPolyMesh, etc.
│   │   ├── abc_impl.rs     # OgawaArchiveReader (implements core traits)
│   │   └── read_util.rs    # Binary parsing utilities
│   │
│   ├── abc/                # High-level API
│   │   └── mod.rs          # IArchive, OArchive, IObject, OObject, Properties
│   │
│   ├── geom/               # Geometry schemas
│   │   ├── polymesh.rs     # IPolyMesh, PolyMeshSample
│   │   ├── xform.rs        # IXform, XformSample, XformOp
│   │   ├── curves.rs       # ICurves, CurvesSample
│   │   ├── points.rs       # IPoints, PointsSample
│   │   ├── subd.rs         # ISubD, SubDSample
│   │   ├── camera.rs       # ICamera, CameraSample
│   │   ├── faceset.rs      # IFaceSet, FaceSetSample
│   │   ├── nupatch.rs      # INuPatch, NuPatchSample
│   │   ├── light.rs        # ILight, LightSample
│   │   ├── visibility.rs   # ObjectVisibility, get_visibility()
│   │   ├── geom_param.rs   # IGeomParam templates
│   │   └── mod.rs          # Schema constants, re-exports
│   │
│   ├── material/           # Material support
│   │   ├── mod.rs          # Constants
│   │   └── schema.rs       # IMaterial
│   │
│   ├── collection/         # Collection support
│   │   ├── mod.rs          # Constants
│   │   └── schema.rs       # ICollections
│   │
│   └── python/             # PyO3 bindings
│       ├── mod.rs          # Module registration
│       ├── archive.rs      # PyIArchive, PyOArchive
│       ├── object.rs       # PyIObject
│       ├── geom.rs         # Py*Sample classes
│       ├── properties.rs   # PyICompoundProperty, PyIScalarProperty
│       ├── time_sampling.rs# PyTimeSampling
│       ├── write.rs        # PyO* write classes
│       └── materials.rs    # PyIMaterial
│
├── tests/
│   ├── read_files.rs       # Integration tests for reading
│   └── write_tests.rs      # Round-trip tests
│
└── data/                   # Test files
    ├── chess3.abc
    ├── chess4.abc
    └── bmw.abc
```

---

## Data Flow Diagrams

### Reading Pipeline

```
┌─────────────────┐
│  .abc File      │
│  (Ogawa Format) │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ IStreams (src/ogawa/reader.rs)                                  │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Memory-mapped I/O (mmap feature) OR Buffered file I/O       │ │
│ │ - read_bytes(offset, len) -> Vec<u8>                        │ │
│ │ - size() -> u64                                             │ │
│ └─────────────────────────────────────────────────────────────┘ │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ OgawaArchiveReader (src/ogawa/abc_impl.rs)                      │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Implements: ArchiveReader trait                             │ │
│ │ - Parses file header (magic, version, root offset)          │ │
│ │ - Loads time samplings                                      │ │
│ │ - Creates root ObjectReader                                 │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ OgawaObjectReader                                               │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Implements: ObjectReader trait                              │ │
│ │ - Navigates child hierarchy                                 │ │
│ │ - Provides property access                                  │ │
│ │ - Lazy loading via OnceLock                                 │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ OgawaPropertyReader                                             │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Implements: ScalarPropertyReader, ArrayPropertyReader       │ │
│ │ - Reads property data blocks                                │ │
│ │ - Handles decompression (zlib)                              │ │
│ │ - Sample caching                                            │ │
│ └─────────────────────────────────────────────────────────────┘ │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ IArchive / IObject / IProperty (src/abc/mod.rs)                 │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ User-facing API wrapping trait objects                      │ │
│ │ - IArchive::open(path) -> Result<IArchive>                  │ │
│ │ - archive.root() -> IObject                                 │ │
│ │ - object.child_by_name(name) -> Option<IObject>             │ │
│ │ - object.properties() -> ICompoundProperty                  │ │
│ └─────────────────────────────────────────────────────────────┘ │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ Geometry Schemas (src/geom/*.rs)                                │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ IPolyMesh::new(&object) -> Option<IPolyMesh>                │ │
│ │ polymesh.get_sample(index) -> Result<PolyMeshSample>        │ │
│ │                                                             │ │
│ │ Sample contains:                                            │ │
│ │   positions: Vec<Vec3>                                      │ │
│ │   face_counts: Vec<i32>                                     │ │
│ │   face_indices: Vec<i32>                                    │ │
│ │   normals: Option<Vec<Vec3>>                                │ │
│ │   uvs: Option<Vec<Vec2>>                                    │ │
│ │   self_bounds: Option<BBox3d>                               │ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Writing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│ User Code                                                       │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ let mut archive = OArchive::create("output.abc")?;          │ │
│ │ let mut mesh = OPolyMesh::new("myMesh");                    │ │
│ │ mesh.add_sample(&OPolyMeshSample::new(pos, counts, idx));   │ │
│ │ let mut root = OObject::new("");                            │ │
│ │ root.add_child(mesh.build());                               │ │
│ │ archive.write_archive(&root)?;                              │ │
│ └─────────────────────────────────────────────────────────────┘ │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ OArchive (src/ogawa/writer.rs)                                  │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ write_archive(&root):                                       │ │
│ │   1. Write time samplings                                   │ │
│ │   2. Recursively write object hierarchy                     │ │
│ │   3. Write property data with deduplication                 │ │
│ │   4. Write header (magic, version, root offset)             │ │
│ │   5. Mark as frozen                                         │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ Deduplication:                                                  │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ - MD5 hash of array data                                    │ │
│ │ - HashMap<[u8;16], u64> stores hash -> offset               │ │
│ │ - Identical samples share storage                           │ │
│ └─────────────────────────────────────────────────────────────┘ │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ .abc File (Ogawa Binary Format)                                 │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Header (16 bytes):                                          │ │
│ │   [0-4]  Magic: "Ogawa"                                     │ │
│ │   [5]    Frozen flag (0xFF when complete)                   │ │
│ │   [6-7]  Version (u16 LE)                                   │ │
│ │   [8-15] Root group offset (u64 LE)                         │ │
│ ├─────────────────────────────────────────────────────────────┤ │
│ │ Groups: Hierarchical containers                             │ │
│ │   - Child count (u64)                                       │ │
│ │   - Child offsets[] (u64 each, MSB=type flag)               │ │
│ │     MSB=0: GROUP, MSB=1: DATA                               │ │
│ ├─────────────────────────────────────────────────────────────┤ │
│ │ Data Blocks: Property values                                │ │
│ │   - Size (u64)                                              │ │
│ │   - Raw bytes (optionally zlib compressed)                  │ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Type Relationships

```
                    ┌─────────────────────────────────┐
                    │         ArchiveReader           │
                    │  (trait in src/core/traits.rs)  │
                    └────────────────┬────────────────┘
                                     │ implements
                    ┌────────────────▼────────────────┐
                    │      OgawaArchiveReader         │
                    │  (src/ogawa/abc_impl.rs)        │
                    │                                 │
                    │  - streams: Arc<IStreams>       │
                    │  - time_samplings: Vec<TS>      │
                    │  - root_data: OgawaObjectData   │
                    └────────────────┬────────────────┘
                                     │ wraps
                    ┌────────────────▼────────────────┐
                    │          IArchive               │
                    │     (src/abc/mod.rs)            │
                    │                                 │
                    │  reader: Box<dyn ArchiveReader> │
                    └────────────────┬────────────────┘
                                     │ provides
                    ┌────────────────▼────────────────┐
                    │          IObject                │
                    │     (src/abc/mod.rs)            │
                    │                                 │
                    │  reader: IObjectReader enum     │
                    │    - Borrowed(&dyn ObjectReader)│
                    │    - Owned(Box<dyn ObjectReader>│
                    └────────────────┬────────────────┘
                                     │ schema check
          ┌──────────────────────────┼──────────────────────────┐
          │                          │                          │
          ▼                          ▼                          ▼
┌─────────────────┐       ┌─────────────────┐        ┌─────────────────┐
│   IPolyMesh     │       │    IXform       │        │   ICurves       │
│(src/geom/       │       │(src/geom/       │        │(src/geom/       │
│ polymesh.rs)    │       │ xform.rs)       │        │ curves.rs)      │
│                 │       │                 │        │                 │
│ object: &IObject│       │ object: &IObject│        │ object: &IObject│
└────────┬────────┘       └────────┬────────┘        └────────┬────────┘
         │ get_sample()            │ get_sample()             │ get_sample()
         ▼                         ▼                          ▼
┌─────────────────┐       ┌─────────────────┐        ┌─────────────────┐
│PolyMeshSample   │       │  XformSample    │        │  CurvesSample   │
│                 │       │                 │        │                 │
│positions: Vec3[]│       │ops: XformOp[]   │        │positions: Vec3[]│
│face_counts: i32[]       │inherits: bool   │        │num_vertices: i32│
│face_indices: i32│       │                 │        │basis: CurveBasis│
│normals: Option  │       │matrix() -> Mat4 │        │type: CurveType  │
│uvs: Option      │       │translation()    │        │                 │
│self_bounds: Opt │       │scale()          │        │                 │
└─────────────────┘       └─────────────────┘        └─────────────────┘
```

---

## Time Sampling System

```
TimeSamplingType enum:
┌────────────────────────────────────────────────────────────────┐
│ Identity                                                       │
│   - Static data (sample 0 only)                               │
│   - time_per_cycle = 1.0, start_time = 0.0                    │
├────────────────────────────────────────────────────────────────┤
│ Uniform { time_per_cycle, start_time }                        │
│   - Regular intervals (e.g., 24 fps = 1/24 per cycle)         │
│   - sample_time(n) = start_time + n * time_per_cycle          │
├────────────────────────────────────────────────────────────────┤
│ Cyclic { time_per_cycle, times: Vec<f64> }                    │
│   - Repeating pattern within cycle                            │
│   - times = offsets within each cycle                         │
├────────────────────────────────────────────────────────────────┤
│ Acyclic { times: Vec<f64> }                                   │
│   - Arbitrary sample times                                    │
│   - times = absolute sample times                             │
└────────────────────────────────────────────────────────────────┘

SampleSelector:
┌────────────────────────────────────────────────────────────────┐
│ Index(usize)        - Direct sample index                     │
│ Time(f64)           - Time in seconds                         │
│ TimeFloor(f64)      - Nearest earlier sample                  │
│ TimeCeil(f64)       - Nearest later sample                    │
│ TimeNearest(f64)    - Closest sample                          │
└────────────────────────────────────────────────────────────────┘
```

---

## Property System

```
Property Hierarchy:
┌────────────────────────────────────────────────────────────────┐
│ CompoundProperty (container)                                   │
│   └─ child properties:                                        │
│       ├─ ScalarProperty (single value per sample)             │
│       │    - i32, f32, Vec3, Mat4, etc.                       │
│       │    - is_constant() - same value all samples?          │
│       │    - read_sample(index, &mut buf)                     │
│       │                                                        │
│       ├─ ArrayProperty (array of values per sample)           │
│       │    - Vec<i32>, Vec<f32>, Vec<Vec3>, etc.              │
│       │    - sample_len(index) - elements in sample           │
│       │    - read_sample_vec(index) -> Vec<u8>                │
│       │    - sample_key(index) - MD5 for dedup                │
│       │                                                        │
│       └─ CompoundProperty (nested)                            │
│            - Recursive structure                              │
└────────────────────────────────────────────────────────────────┘

Standard Properties in .geom compound:
┌────────────────────────────────────────────────────────────────┐
│ "P"              - positions (Vec3 array)                     │
│ ".faceCounts"    - vertices per face (i32 array)              │
│ ".faceIndices"   - vertex indices (i32 array)                 │
│ "N"              - normals (Vec3 array)                       │
│ ".velocities"    - velocity vectors (Vec3 array)              │
│ ".selfBnds"      - bounding box (6 x f64: min, max)           │
│ ".childBnds"     - child bounds (6 x f64)                     │
│ ".arbGeomParams" - arbitrary geometry parameters              │
│ ".userProperties"- user-defined properties                    │
└────────────────────────────────────────────────────────────────┘
```

---

## Schema Identification

```rust
// Schema constants (src/geom/mod.rs)
pub const XFORM_SCHEMA: &str = "AbcGeom_Xform_v3";
pub const POLYMESH_SCHEMA: &str = "AbcGeom_PolyMesh_v1";
pub const SUBD_SCHEMA: &str = "AbcGeom_SubD_v1";
pub const CURVES_SCHEMA: &str = "AbcGeom_Curves_v1";
pub const POINTS_SCHEMA: &str = "AbcGeom_Points_v1";
pub const CAMERA_SCHEMA: &str = "AbcGeom_Camera_v1";
pub const LIGHT_SCHEMA: &str = "AbcGeom_Light_v1";
pub const FACESET_SCHEMA: &str = "AbcGeom_FaceSet_v1";
pub const NUPATCH_SCHEMA: &str = "AbcGeom_NuPatch_v1";

// Check: object.matches_schema(POLYMESH_SCHEMA)
// or:    object.header().meta_data.get("schema")
```

---

## Python Bindings Architecture

```
Python Module Structure:
┌────────────────────────────────────────────────────────────────┐
│ alembic_rs (PyModule)                                         │
│   ├── Abc (submodule)                                         │
│   │     ├── IArchive      → PyIArchive                        │
│   │     ├── OArchive      → PyOArchive                        │
│   │     ├── IObject       → PyIObject                         │
│   │     ├── TimeSampling  → PyTimeSampling                    │
│   │     └── ICompoundProperty, IScalarProperty, IArrayProperty│
│   │                                                            │
│   └── AbcGeom (submodule)                                     │
│         ├── PolyMeshSample    → PyPolyMeshSample              │
│         ├── XformSample       → PyXformSample                 │
│         ├── CurvesSample      → PyCurvesSample                │
│         ├── PointsSample      → PyPointsSample                │
│         ├── CameraSample      → PyCameraSample                │
│         ├── SubDSample        → PySubDSample                  │
│         ├── ObjectVisibility  → PyObjectVisibility            │
│         └── OPolyMesh, OXform, OCurves... (write support)     │
└────────────────────────────────────────────────────────────────┘

Python Object Ownership:
┌────────────────────────────────────────────────────────────────┐
│ PyIArchive                                                    │
│   inner: Arc<OgawaIArchive>  ← Shared ownership               │
│                                                                │
│ PyIObject                                                     │
│   archive: Arc<OgawaIArchive>  ← Keeps archive alive          │
│   path: Vec<String>            ← Path from root               │
│                                                                │
│ Access Pattern:                                                │
│   with_object(|obj| { ... })   ← Closure-based traversal      │
│   - Traverses path on each call                               │
│   - No dangling references                                    │
│   - Thread-safe via Arc                                       │
└────────────────────────────────────────────────────────────────┘
```

---

## Error Handling

```rust
// src/util/error.rs
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid format: {0}")]
    Invalid(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Mmap failed: {0}")]
    MmapFailed(String),

    #[error("Other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Usage patterns:
// - Propagation: function()?
// - Context: .map_err(|e| Error::invalid(format!("...: {}", e)))?
// - Options: .ok_or_else(|| Error::not_found("..."))?
```

---

## Caching Strategy

```
ArraySampleCache (src/core/cache.rs):
┌────────────────────────────────────────────────────────────────┐
│ Key: ArraySampleKey { path, property, index }                 │
│ Value: CachedSample { data: Arc<Vec<u8>>, size }              │
│                                                                │
│ - Default max size: 256 MB                                    │
│ - Eviction: Remove ~50% when full (random, not LRU)           │
│ - Thread-safe: RwLock + atomic size tracking                  │
│                                                                │
│ Usage:                                                         │
│   let cache = ArraySampleCache::new(256 * 1024 * 1024);       │
│   cache.get(&key) -> Option<Arc<Vec<u8>>>                     │
│   cache.insert(key, data) -> ()                               │
└────────────────────────────────────────────────────────────────┘
```

---

## Build Configuration

```toml
# Cargo.toml features
[features]
default = ["mmap"]
mmap = []          # Enable memory-mapped I/O
python = ["pyo3"]  # Build Python bindings

# Build commands
cargo build                    # Library only
cargo build --features python  # With Python bindings
maturin develop               # Python dev build
maturin build --release       # Python wheel
```

---

## Test Data

| File | Description | Size | Objects |
|------|-------------|------|---------|
| chess3.abc | Chess scene v3 | ~2 MB | ~40 meshes |
| chess4.abc | Chess scene v4 | ~2 MB | ~40 meshes |
| bmw.abc | BMW car model | ~15 MB | ~200 meshes, ~50k verts |

---

## Known Limitations

1. **HDF5 format not supported** - Only Ogawa binary format
2. **No layer/LOD support** - Single representation per object
3. **Parent tracking not implemented** - Cannot traverse up hierarchy
4. **Cache eviction primitive** - Not LRU, just random eviction
5. **Some Python bindings incomplete** - See plan1.md for details

---

## Performance Considerations

- **Memory mapping preferred** for large files (>10 MB)
- **Sample caching** reduces repeated reads
- **Deduplication** on write reduces file size significantly
- **Lazy loading** - objects created on demand
- **OnceLock** for thread-safe lazy initialization
