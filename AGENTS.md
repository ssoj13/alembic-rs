# alembic-rs Architecture & Dataflow

## Project Overview

Rust port of Alembic (.abc) 3D interchange format with integrated PBR viewer.

## Module Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         PUBLIC API (lib.rs)                         │
│  prelude, IArchive, OArchive, IObject, IPolyMesh, IXform, etc.     │
└─────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌───────────────┐         ┌─────────────────┐         ┌───────────────┐
│    abc/       │         │     geom/       │         │   material/   │
│  High-level   │         │   Geometry      │         │   Materials   │
│    API        │         │   Schemas       │         │   Shaders     │
└───────────────┘         └─────────────────┘         └───────────────┘
        │                         │                           │
        └─────────────────────────┼───────────────────────────┘
                                  ▼
                    ┌─────────────────────────┐
                    │        core/            │
                    │   Abstract Traits       │
                    │   TimeSampling          │
                    │   MetaData              │
                    │   Headers               │
                    └─────────────────────────┘
                                  │
                                  ▼
                    ┌─────────────────────────┐
                    │        ogawa/           │
                    │   Binary Format         │
                    │   Reader/Writer         │
                    │   Compression           │
                    │   Deduplication         │
                    └─────────────────────────┘
                                  │
                                  ▼
                    ┌─────────────────────────┐
                    │        util/            │
                    │   DataType, Error       │
                    │   POD types             │
                    │   BBox, Vec types       │
                    └─────────────────────────┘
```

## Reading Dataflow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        FILE READING PIPELINE                             │
└──────────────────────────────────────────────────────────────────────────┘

.abc file
    │
    ▼
┌──────────────────┐     ┌──────────────────┐
│  mmap/File I/O   │────▶│  Ogawa Parser    │
└──────────────────┘     │  - Magic check   │
                         │  - Version       │
                         │  - Root offset   │
                         └────────┬─────────┘
                                  │
                                  ▼
                    ┌─────────────────────────┐
                    │   OgawaArchiveReader    │
                    │   - time_samplings[]    │
                    │   - indexed_metadata[]  │
                    │   - root object         │
                    └───────────┬─────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌───────────────┐
│ OgawaObject   │     │ OgawaCompound   │     │ OgawaArray    │
│ - header      │     │ - properties[]  │     │ - samples[]   │
│ - children[]  │     │ - sub-props[]   │     │ - keys[]      │
│ - properties  │     └─────────────────┘     │ - dims[]      │
└───────────────┘                             └───────────────┘
        │
        ▼
┌───────────────────────────────────────────────────────────────┐
│                    SCHEMA INTERPRETATION                       │
│  IXform, IPolyMesh, ICurves, IPoints, ICamera, IMaterial...   │
└───────────────────────────────────────────────────────────────┘
```

## Writing Dataflow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        FILE WRITING PIPELINE                             │
└──────────────────────────────────────────────────────────────────────────┘

User Data (vertices, indices, transforms...)
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│                      OArchive::create()                        │
│   - Write Ogawa magic header                                  │
│   - Initialize time samplings (identity at index 0)           │
│   - Initialize deduplication map                              │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│              add_object() / add_property()                     │
│   - Build hierarchy                                           │
│   - Write samples with deduplication                          │
│   - Compress data if hint >= 0                                │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│                      OArchive::close()                         │
│   - Write time samplings                                      │
│   - Write indexed metadata                                    │
│   - Write object hierarchy                                    │
│   - Update root position in header                            │
│   - Set frozen flag                                           │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
.abc file
```

## Writer Parity Notes (Ogawa / AbcCoreOgawa)

```
Object Write Order (reference)
  1) Child objects written first (hashes available)
  2) Property sample data written
  3) Property groups written in reverse creation order
  4) Object headers written (include data-hash + child-hash suffix)
  5) Property headers written
  6) Object group frozen (properties + children + headers)

Archive Finalization Order
  - Version data
  - Library version data
  - Root object group
  - Archive metadata
  - Time samplings (max samples + stored times)
  - Indexed metadata table

Time Sampling Notes (reference)
  - Constant properties contribute maxSamples = 1 (even if repeated samples exist)
  - _ai_AlembicVersion is always set from library version/build time
```

## Viewer Pipeline

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        VIEWER RENDERING PIPELINE                         │
└──────────────────────────────────────────────────────────────────────────┘

.abc file
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│                    collect_scene_cached()                      │
│   - Sequential file reads                                     │
│   - Build mesh/curves/points tasks                            │
│   - Collect cameras, lights, materials                        │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│              Parallel Conversion (Rayon)                       │
│   convert_polymesh() - triangulation, normals                 │
│   convert_curves()   - line strips                            │
│   convert_points()   - point sprites                          │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│                    GPU Upload (wgpu)                           │
│   Renderer::add_mesh() - vertex/index buffers                 │
│   Renderer::add_curves() - line buffers                       │
│   Renderer::add_points() - point buffers                      │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
┌───────────────────────────────────────────────────────────────┐
│                    Render Loop                                 │
│   1. Shadow depth pass                                        │
│   2. Skybox pass (if HDR loaded)                              │
│   3. Grid pass (line pipeline)                                │
│   4. Opaque mesh pass (Standard Surface shader)               │
│   5. Transparent mesh pass (sorted back-to-front)             │
│   6. Curves pass (line pipeline)                              │
│   7. Points pass (point pipeline)                             │
└───────────────────────────────────────────────────────────────┘
    │
    ▼
Screen
```

## Viewer Frame Update (Current)

```
UI Thread (eframe::App::update)
    |
    v
process_worker_results()  <-- non-blocking recv
    |
    v
apply_scene(frame, CollectedScene)
    |
    |-- update stats + bounds + floor + scene lights/cameras
    |-- retain meshes/curves by path (points NOT retained)
    |-- meshes:
    |     update transform
    |     if vertex_hash changed -> recreate GPU buffers
    |     else -> reuse
    |-- curves:
    |     add_curves() always creates new buffers + bind groups
    |-- points:
    |     add_points() always creates new buffers + bind groups
    |
    v
Renderer::render() -> GPU passes
```

## Camera Input Mapping (Current vs Desired)

```
Current (viewport.rs):
  LMB drag  -> orbit
  MMB drag  -> pan
  Shift+LMB -> pan
  Wheel     -> zoom

Desired (Houdini/Maya-like):
  LMB drag  -> orbit
  MMB drag  -> pan
  RMB drag  -> zoom
  Wheel     -> (optional) zoom
```

## Key Data Structures

```
IArchive
├── name: String
├── time_samplings: Vec<TimeSampling>
├── root: IObject
│   ├── header: ObjectHeader
│   │   ├── name
│   │   ├── full_name
│   │   └── meta_data
│   ├── children: Vec<IObject>
│   └── properties: ICompoundProperty
│       ├── scalar_properties
│       ├── array_properties
│       └── compound_properties
└── metadata: MetaData
```

## Dependencies

```
alembic-rs
├── murmur3 (internal crate) - hash for dedup
├── spooky-hash (internal crate) - SpookyHash V2
├── standard-surface (internal crate) - PBR shader
│   └── wgpu - GPU rendering
├── glam - linear algebra
├── half - f16 support
├── memmap2 - memory-mapped files
├── flate2 - zlib compression
├── rayon - parallel processing
├── parking_lot - fast mutex
├── smallvec - stack-allocated vectors
└── bytemuck - safe POD casting
```

## BUGHUNT Status (2026-01-22)

### Fixed Issues

| Issue | Status |
|-------|--------|
| PhantomData stubs in geom/mod.rs | ✅ Fixed - re-exports from ogawa/writer.rs |
| abc::OArchive stub | ✅ Fixed - write_archive() delegates to ogawa |
| Dead O* stubs in abc/mod.rs | ✅ Fixed - removed |
| Code duplication (~60 lines) | ✅ Fixed - shared helpers in OProperty |

### Current Analysis (2026-01-22)

| Category | Count | Severity |
|----------|-------|----------|
| Critical Bugs (unwrap/panic) | 3 | HIGH |
| Logic Bugs | 14 | MEDIUM |
| Dead code markers | 37 | LOW |
| Unused Functions | 17 | MEDIUM |
| Error Handling Issues | 18 | HIGH |
| Interface Inconsistencies | 8 | MEDIUM |
| Code Duplication | 12 | LOW |
| Clippy Warnings | 22 | LOW |

### Critical Bugs Found

1. **abc_impl.rs:247** - `unwrap()` on Option in `findObject()` can panic
2. **writer/property.rs:280,296,314** - `panic!()` in public API methods
3. **writer/archive/properties.rs:169-170** - `unwrap()` on slice conversion

### Tests Status

- **Unit tests**: 109 passed ✅
- **Integration test**: 1 failed (missing test data file `gears_out.abc`)
- **Clippy**: 22 warnings (all fixable)

### See Also

- [PLAN1.md](./PLAN1.md) - Full bug hunt report with recommendations
- [DIAGRAMS.md](./DIAGRAMS.md) - Architecture diagrams
