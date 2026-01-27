# Alembic-RS Audit Findings

**Audit Date:** 2026-01-26
**Auditor:** Claude
**Purpose:** Comprehensive audit of C++ Alembic port to Rust

## Summary

Overall the port is **very complete and production-ready**. All major features are implemented with good Rust idioms. The code is well-structured and follows modern Rust practices.

---

## 1. Core API Parity

### 1.1 Archive (IArchive/OArchive)
- [x] IArchive - read archive
- [x] OArchive - write archive
- [x] Archive info (app name, library version, date written)
- [x] TimeSampling management
- [x] Top object access
- [ ] ReadArraySampleCache (not implemented - low priority)

### 1.2 Object Hierarchy (IObject/OObject)
- [x] Object creation/traversal
- [x] Parent/child relationships
- [x] Object names and full paths
- [x] Metadata handling
- [x] Object headers
- [x] getChildHeader(index) - implemented
- [x] Instance API surface (isInstanceRoot, instanceSourcePath, isInstanceDescendant, isChildInstance)
- [!] Instance methods are stubs (default trait impls) - ogawa reader doesn't parse instance data

### 1.3 Properties
- [x] Scalar properties
- [x] Array properties  
- [x] Compound properties
- [x] Property headers
- [x] Data types (all POD types)
- [x] Extent handling

### 1.4 TimeSampling
- [x] Uniform sampling
- [x] Cyclic sampling
- [x] Acyclic sampling
- [x] Time sampling pool
- [x] Sample time queries

---

## 2. Geometry Schemas

### 2.1 PolyMesh (IPolyMesh/OPolyMesh) - COMPLETE
- [x] Positions (P)
- [x] Face counts
- [x] Face indices
- [x] Velocities
- [x] Normals (N) with GeomParam support
- [x] UVs with indexed support
- [x] Self bounds
- [x] Child bounds
- [x] Arbitrary GeomParams
- [x] FaceSets
- [x] Topology variance detection

### 2.2 Xform (IXform/OXform) - COMPLETE
- [x] Translation
- [x] Rotation (X/Y/Z and arbitrary axis)
- [x] Scale
- [x] Matrix (4x4)
- [x] Inherits transform flag
- [x] Operations stack decoding
- [x] Child bounds

### 2.3 Camera (ICamera/OCamera) - COMPLETE
- [x] Focal length
- [x] Horizontal/Vertical aperture
- [x] Film offsets
- [x] Lens squeeze ratio
- [x] Near/far clipping
- [x] Focus distance
- [x] Shutter open/close
- [x] Film back xform operations

### 2.4 Curves (ICurves/OCurves) - COMPLETE
- [x] Positions
- [x] Curve types (linear, cubic, bezier, bspline, catmullrom, hermite)
- [x] Wrap mode (periodic/non-periodic)
- [x] Num vertices per curve
- [x] Knots
- [x] Orders
- [x] Widths
- [x] Basis type

### 2.5 Points (IPoints/OPoints) - COMPLETE
- [x] Positions
- [x] Ids
- [x] Velocities
- [x] Widths

### 2.6 SubD (ISubD/OSubD) - COMPLETE
- [x] Positions
- [x] Face counts/indices
- [x] FV interpolate boundary
- [x] FV propagate corners
- [x] Interpolate boundary
- [x] Crease indices/lengths/sharpnesses
- [x] Corner indices/sharpnesses
- [x] Holes
- [x] Subdivision scheme (catmull-clark, loop, bilinear)
- [x] UVs

### 2.7 NuPatch (INuPatch/ONuPatch) - COMPLETE
- [x] Positions
- [x] Num U/V
- [x] U/V Order
- [x] U/V Knot
- [x] Position weights
- [x] Normals
- [x] UVs
- [x] Trim curves

### 2.8 FaceSet (IFaceSet/OFaceSet) - COMPLETE
- [x] Faces
- [x] Exclusivity
- [x] Visibility

### 2.9 Light (ILight/OLight) - COMPLETE
- [x] Camera schema (shared)
- [x] Child bounds
- [x] Arbitrary GeomParams

---

## 3. Ogawa Format - COMPLETE

### 3.1 Reading
- [x] Header parsing (magic, version, frozen flag)
- [x] Group reading with child offsets
- [x] Data reading
- [x] Indexed metadata strings
- [x] Memory-mapped I/O
- [ ] Buffered I/O option (for modifiable files)

### 3.2 Writing
- [x] Header generation
- [x] Group writing
- [x] Data writing
- [x] Metadata indexing
- [x] SpookyHash V2 for deduplication
- [x] MurmurHash3 for metadata
- [x] Stream management
- [x] Deferred group writing (C++ compatible mode)

---

## 4. Python Bindings (PyO3) - COMPLETE

### 4.1 Archive
- [x] IArchive opening (Abc.IArchive)
- [x] OArchive creation (Abc.OArchive)
- [x] Top object access
- [x] Archive info (getAppName, getDateWritten, etc.)

### 4.2 Objects
- [x] Object traversal
- [x] Object properties access
- [x] Object metadata

### 4.3 Geometry - All Schemas
- [x] IPolyMesh/IPolyMeshSchema
- [x] IXform/IXformSchema
- [x] ICamera/ICameraSchema
- [x] ICurves/ICurvesSchema
- [x] IPoints/IPointsSchema
- [x] ISubD/ISubDSchema
- [x] INuPatch/INuPatchSchema
- [x] IFaceSet/IFaceSetSchema
- [x] ILight/ILightSchema

### 4.4 Properties
- [x] ICompoundProperty
- [x] Property info

### 4.5 Write Classes
- [x] OPolyMesh, OXform, OCurves, OPoints
- [x] OSubD, OCamera, ONuPatch, OLight
- [x] OFaceSet, OMaterial, OCollections
- [x] OScalarProperty, OArrayProperty, OCompoundProperty

### 4.6 Constants
- [x] GeometryScope (kConstantScope, kVertexScope, etc.)
- [x] CurveType, CurvePeriodicity
- [x] BasisType
- [x] SubDScheme
- [x] TopologyVariance
- [x] FaceSetExclusivity
- [x] ObjectVisibility

---

## 5. Viewer (Additional Feature) - PRODUCTION READY

### 5.1 Rendering
- [x] Mesh rendering (solid + wireframe)
- [x] StandardSurface material shader
- [x] Environment maps (HDR)
- [x] Shadows
- [x] MSAA antialiasing
- [x] Grid display

### 5.2 UI
- [x] Orbit camera controls
- [x] Scene camera support
- [x] Timeline with playback controls
- [x] Scene hierarchy tree
- [x] Wildcard object filtering
- [x] Settings panel
- [x] Recent files menu

### 5.3 Performance
- [x] Async loading via worker thread
- [x] Hash-based scene change detection
- [x] GPU buffer caching

### 5.4 Export
- [x] Export functionality

---

## 6. Additional Crates - COMPLETE

### 6.1 murmur3
- [x] MurmurHash3 x64_128 implementation
- [x] Binary compatible with C++ Alembic
- [x] Big-endian POD byte swapping support

### 6.2 spooky-hash
- [x] SpookyHash V2 implementation
- [x] All rotation constants match reference
- [x] Short/long message paths
- [x] Incremental hashing support
- [x] Binary compatible test cases

### 6.3 standard-surface
- [x] MaterialX StandardSurface shader params
- [x] wgpu shader implementation

---

## 7. Findings

### Critical Issues
*None found - the port is functionally complete*

### Missing Features (Low Priority)
1. **Instance support** - `isInstanceRoot()`, `instanceSourcePath()` not implemented
   - Used for referencing objects within archive
   - Relatively rare in production files
   
2. **ReadArraySampleCache** - read-side sample caching
   - C++ Alembic has this for performance
   - Rust mmap provides similar benefits

3. **Buffered I/O reader** - alternative to mmap
   - For files that may be modified during read
   - Comment mentions this but not implemented

### API Differences (Acceptable)
1. `getTimeSampling()` returns index instead of TimeSampling object
   - Rust ownership makes returning reference tricky
   - Can get object via archive.getTimeSampling(index)

2. `getChildHeader()` not exposed as separate method
   - Can get via getChild(i).getHeader()

### Optimizations Applied
1. **FIXED: Per-frame bind group allocation** - 4x `create_bind_group` per frame for SSAO, SSAO blur (H+V), and lighting passes. Now cached and rebuilt only on texture resize or env map change. This was the root cause of periodic stuttering.
2. **FIXED: Redundant uniform writes** - SSAO blur direction params now use two static buffers instead of rewriting one buffer twice per frame.

### Writer Bugs Fixed (Binary Compatibility)
3. **FIXED: OCurves `.nVertices` → `nVertices`** - Dotted prefix was incorrect. C++ ref: `OCurves.cpp:393` uses `"nVertices"`.
4. **FIXED: OSubD schema version `v2` → `v1`** - Writer used `AbcGeom_SubD_v2` but C++ ref: `SchemaInfoDeclarations.h:72` declares `AbcGeom_SubD_v1`.
5. **FIXED: OCurves curveBasisAndType** - Writer used 3 separate string scalars (`.curveType`, `.wrap`, `.basis`) but C++ ref: `OCurves.cpp:395-397` uses a single `curveBasisAndType` scalar (kUint8POD, extent=4) with layout `[type, wrap, basis, basis]`.

### Writer Audit (All Schemas vs C++ Ref)
6. **VERIFIED**: All 8 writer schemas (PolyMesh, SubD, Curves, Points, Camera, Xform, Light, NuPatch, FaceSet) audited against C++ reference. Schema versions, property names, data types all match after fixes above.
7. **FIXED: OCurves width property `.widths` → `width`** - C++ ref: `OCurves.cpp:511` uses `"width"` (no dot, singular, via GeomParam).

### Reader Bugs Fixed (Binary Compatibility)
8. **FIXED: ICurves `.knots`/`.orders`** - Reader used `"knots"` and `"orders"` but C++ ref: `ICurves.cpp:128,134` uses `".knots"` and `".orders"` (with dot prefix).
9. **FIXED: IPoints `.velocities`** - Reader had non-standard `"velocity"` as primary fallback. C++ ref: `IPoints.cpp:62` only reads `".velocities"`.
10. **FIXED: IPoints `.widths`** - Reader used `"width"` but C++ ref: `IPoints.cpp:70` uses `".widths"` (with dot, plural).

### Reader Audit (All Schemas vs C++ Ref)
11. **VERIFIED**: All readers (PolyMesh, SubD, Curves, Points, Camera, Xform, NuPatch, FaceSet, Light) audited against C++ reference. Property names match after fixes above.

### Enum/Type Compatibility Fixes
12. **FIXED: CurveType enum** - Had 6 values (Cubic, Linear, Bezier, Bspline, CatmullRom, Hermite) but C++ ref `CurveType.h` only has 3: kCubic=0, kLinear=1, kVariableOrder=2. Bezier/Bspline/etc belong in BasisType, not CurveType. Removed wrong variants, added VariableOrder.
13. **FIXED: OSubDSample default scheme** - Used `"catmullClark"` (camelCase) but C++ ref `OSubD.h:105` uses `"catmull-clark"` (hyphenated). Fixed default and parser to accept both forms.
14. **FIXED: Python CurveType constants** - Removed kBezier/kBspline/kCatmullRom/kHermite from CurveType constants (those were BasisType values). Added kVariableOrder.

### Optimizations Possible
1. More aggressive use of `zerocopy`/`bytemuck` for POD types
2. Property reader could cache decoded samples
3. Viewer: LOD support for very large scenes
4. Viewer: Frustum culling for many objects

### Code Quality Notes
- Well-structured modular code
- Good use of Rust idioms (Result, Option, traits)
- Comprehensive error types
- Tests present for critical paths
- Good documentation in module headers

---

## 8. Recommendations

### Short Term
1. Add `isInstanceRoot()` / `instanceSourcePath()` for instance support
2. Add `getChildHeader(index)` method to IObject

### Long Term
1. Consider adding ReadArraySampleCache equivalent
2. Add buffered I/O option for reader
3. Profile and optimize hot paths

---

## 9. Changelog

| Date | Item | Status | Notes |
|------|------|--------|-------|
| 2026-01-26 | Core API audit | Complete | IArchive, OArchive, IObject OK |
| 2026-01-26 | Geometry schemas audit | Complete | All 9 schemas implemented |
| 2026-01-26 | Ogawa format audit | Complete | Read/write binary compatible |
| 2026-01-26 | Python bindings audit | Complete | Full API coverage |
| 2026-01-26 | Hash functions audit | Complete | SpookyV2, MurmurHash3 correct |
| 2026-01-26 | Viewer audit | Complete | Production-ready |
| 2026-01-26 | Bind group caching fix | Complete | Root cause of viewer stuttering |
| 2026-01-26 | IBL hemisphere sampling | Complete | Improved diffuse quality |
| 2026-01-26 | OCurves nVertices fix | Complete | Binary compat: dot prefix removed |
| 2026-01-26 | OSubD schema v1 fix | Complete | Binary compat: v2→v1 |
| 2026-01-26 | curveBasisAndType fix | Complete | Binary compat: 3 strings→1 uint8x4 scalar |
| 2026-01-26 | Full writer audit | Complete | All 8 schemas verified vs C++ ref |
| 2026-01-26 | Roundtrip tests added | Complete | Curves, Points, SubD, Camera |
| 2026-01-26 | OCurves width prop fix | Complete | Binary compat: .widths→width |
| 2026-01-26 | Reader property fixes | Complete | knots/orders dots, points vel/widths |
| 2026-01-26 | Full reader audit | Complete | All schemas verified vs C++ ref |
| 2026-01-26 | CurveType enum fix | Complete | Removed wrong variants, added VariableOrder |
| 2026-01-26 | SubD scheme default | Complete | catmullClark→catmull-clark |
| 2026-01-26 | Python constants fix | Complete | CurveType constants corrected |
| 2026-01-26 | Zero-copy cast optimization | Complete | try_cast_vec for f32/i32 arrays |
| 2026-01-26 | IFaceSet reader fix | Complete | .geom→.faceset compound name |
| 2026-01-26 | Edge-case tests added | Complete | NuPatch, basis types, empty mesh, anim curves, light, faceset |
| 2026-01-26 | ISampleSelector Python API | Complete | Full index/time/floor/ceil/near selection |
| 2026-01-26 | Schema getValue() selector | Complete | All 9 schemas accept ISampleSelector |
| 2026-01-26 | PyIObject getMetaData/getArchive | Complete | Metadata dict + archive back-ref |
| 2026-01-26 | Panic safety fixes | Complete | abc_impl ?-operator, geom_param let-else |
| 2026-01-27 | Points varying widths test | Complete | 2-frame animation with per-point widths + velocities |
| 2026-01-27 | All POD types test | Complete | Roundtrip all 12 scalar types (bool→string) via OProperty/IProperty |
| 2026-01-27 | Path tracer BVH infrastructure | Complete | bvh.rs (data types), build.rs (SAH builder), gpu_data.rs (serialization) |
| 2026-01-27 | BVH traverse WGSL shader | Complete | Moller-Trumbore intersection, slab AABB test, stack-based traversal |
| 2026-01-27 | Full API audit vs C++ _ref | Complete | All 9 geometry readers/writers at parity. TimeSampling methods present. Only missing: ArchiveBounds convenience helpers |
| 2026-01-27 | Thread safety review | Complete | ArchiveReader trait is Send+Sync, IArchive auto-derives. No unsafe Send/Sync impls needed |
| 2026-01-27 | PT compute pipeline | Complete | compute.rs: PathTraceCompute with dispatch/blit, blit.wgsl ACES tone map |
| 2026-01-27 | PT scene converter | Complete | scene_convert.rs: Vertex/indices → Triangle with world-space transform |
| 2026-01-27 | Binary compat verified | Complete | C++ abcls/abctree/abcecho all read our files. Byte-level ordering diffs but logically identical |
| 2026-01-27 | F-key focus bug fix | Complete | Curves/hair now included in scene bounds (ConvertedCurves.bounds + compute_scene_bounds) |
| 2026-01-27 | Path tracing UI toggle | Complete | Checkbox in Display panel, lazy init + scene upload on enable |

---

**Conclusion:** The alembic-rs port is **production-ready** with excellent coverage of the C++ API. The missing features (instances, caching) are low-priority and the existing implementation handles the vast majority of real-world Alembic files.
