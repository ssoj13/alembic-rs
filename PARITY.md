# alembic-rs Parity Report

Comprehensive comparison of alembic-rs implementation vs original C++ Alembic library.

**Legend:**
- [x] Implemented
- [ ] Not implemented
- [~] Partially implemented

---

## 1. Abc Module (High-Level API)

### IArchive (Input Archive)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IArchive()` default constructor | Yes | Yes | |
| `IArchive(fileName)` constructor | Yes | Yes | `IArchive::open()` |
| `getName()` | Yes | Yes | `name()` |
| `getTop()` / root object | Yes | Yes | `root()` |
| `getTimeSampling(index)` | Yes | Yes | `time_sampling()` |
| `getNumTimeSamplings()` | Yes | Yes | `num_time_samplings()` |
| `getMaxNumSamplesForTimeSamplingIndex()` | Yes | Yes | `max_num_samples_for_time_sampling()` |
| `getArchiveVersion()` | Yes | Yes | `archive_version()` |
| `getReadArraySampleCachePtr()` | Yes | [ ] | Array sample caching |
| `setReadArraySampleCachePtr()` | Yes | [ ] | |
| `getCoreArchive()` | Yes | [ ] | Access to underlying AbcCoreAbstract |
| `valid()` | Yes | [ ] | |
| Error handler policy | Yes | [ ] | |
| Multiple archive formats (Ogawa/HDF5) | Yes | [~] | Only Ogawa implemented |

### OArchive (Output Archive)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `OArchive()` default constructor | Yes | Yes | |
| `OArchive(fileName)` constructor | Yes | Yes | `OArchive::create()` |
| `getName()` | Yes | [ ] | |
| `getTop()` / root object | Yes | Yes | `root()` |
| `addTimeSampling()` | Yes | [ ] | |
| `getTimeSampling()` | Yes | [ ] | |
| `getNumTimeSamplings()` | Yes | [ ] | |
| `setCompressionHint()` | Yes | [ ] | |
| `getCompressionHint()` | Yes | [ ] | |
| `getCoreArchive()` | Yes | [ ] | |
| Write actual data | Yes | [ ] | Only stub |

### IObject (Input Object)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IObject()` default constructor | Yes | Yes | |
| `getHeader()` | Yes | Yes | `header()` |
| `getName()` | Yes | Yes | `name()` |
| `getFullName()` | Yes | Yes | `full_name()` |
| `getMetaData()` | Yes | Yes | `meta_data()` |
| `getArchive()` | Yes | [ ] | |
| `getParent()` | Yes | [ ] | |
| `getNumChildren()` | Yes | Yes | `num_children()` |
| `getChildHeader(index)` | Yes | Yes | `child_header()` |
| `getChild(index)` | Yes | Yes | `child()` |
| `getChild(name)` | Yes | Yes | `child_by_name()` |
| `getProperties()` | Yes | Yes | `properties()` |
| `isInstanceRoot()` | Yes | Yes | `is_instance_root()` |
| `isInstanceDescendant()` | Yes | Yes | `is_instance_descendant()` |
| `instanceSourcePath()` | Yes | Yes | `instance_source_path()` |
| `isChildInstance(index)` | Yes | Yes | `is_child_instance()` |
| `getPropertiesHash()` | Yes | Yes | `properties_hash()` |
| `getChildrenHash()` | Yes | Yes | `children_hash()` |
| Schema matching | Yes | Yes | `matches_schema()` |
| `valid()` | Yes | [ ] | |
| Error handler policy | Yes | [ ] | |

### OObject (Output Object)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `OObject()` default constructor | Yes | Yes | |
| `getName()` | Yes | Yes | Stub only |
| `getHeader()` | Yes | [ ] | |
| `getFullName()` | Yes | [ ] | |
| `getNumChildren()` | Yes | [ ] | |
| `createChild()` | Yes | [ ] | |
| `getChild()` | Yes | [ ] | |
| `getProperties()` | Yes | [ ] | |
| `getArchive()` | Yes | [ ] | |
| `getParent()` | Yes | [ ] | |
| Add child object | Yes | [ ] | |
| `valid()` | Yes | [ ] | |

### ICompoundProperty
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ICompoundProperty()` default | Yes | Yes | |
| `getHeader()` | Yes | Yes | `header()` |
| `getNumProperties()` | Yes | Yes | `num_properties()` |
| `getPropertyHeader(index)` | Yes | [ ] | |
| `getPropertyHeader(name)` | Yes | [ ] | |
| `getProperty(index)` | Yes | Yes | `property()` |
| `getProperty(name)` | Yes | Yes | `property_by_name()` |
| `getParent()` | Yes | [ ] | |
| `getScalarProperty()` | Yes | [~] | Via `as_scalar()` |
| `getArrayProperty()` | Yes | [~] | Via `as_array()` |
| `getCompoundProperty()` | Yes | [~] | Via `as_compound()` |
| `valid()` | Yes | [ ] | |

### OCompoundProperty
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `OCompoundProperty()` default | Yes | Yes | Stub only |
| Create scalar property | Yes | [ ] | |
| Create array property | Yes | [ ] | |
| Create compound property | Yes | [ ] | |
| All write operations | Yes | [ ] | |

### IScalarProperty
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IScalarProperty()` default | Yes | Yes | |
| `getHeader()` | Yes | Yes | `header()` |
| `getNumSamples()` | Yes | Yes | `num_samples()` |
| `isConstant()` | Yes | Yes | `is_constant()` |
| `getTimeSampling()` | Yes | [ ] | |
| `get(sample, selector)` | Yes | Yes | `read_sample()` |
| `getParent()` | Yes | [ ] | |
| `valid()` | Yes | [ ] | |
| Typed property access | Yes | [ ] | `ITypedScalarProperty<T>` |

### IArrayProperty
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IArrayProperty()` default | Yes | Yes | |
| `getHeader()` | Yes | Yes | `header()` |
| `getNumSamples()` | Yes | Yes | `num_samples()` |
| `isConstant()` | Yes | Yes | `is_constant()` |
| `isScalarLike()` | Yes | [ ] | |
| `getTimeSampling()` | Yes | [ ] | |
| `get(sample, selector)` | Yes | Yes | `read_sample_vec()` |
| `getAs(sample, pod)` | Yes | [ ] | Type conversion |
| `getKey(key, selector)` | Yes | [ ] | Array sample key |
| `getDimensions(dims, selector)` | Yes | [ ] | |
| `getParent()` | Yes | [ ] | |
| `valid()` | Yes | [ ] | |
| Typed property access | Yes | [ ] | `ITypedArrayProperty<T>` |

### OScalarProperty / OArrayProperty
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| All output property features | Yes | [ ] | Stubs only |

### ISampleSelector
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| Index-based selection | Yes | Yes | `SampleSelector::Index` |
| Time-based selection (floor) | Yes | [~] | Enum variant exists |
| Time-based selection (ceil) | Yes | [~] | Enum variant exists |
| Time-based selection (nearest) | Yes | [~] | Enum variant exists |
| Actual time resolution | Yes | Yes | `get_index()`, `get_sample_interp()` |

---

## 2. AbcGeom Module (Geometry Schemas)

### IXform / OXform
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IXformSchema` | Yes | Yes | `IXform` |
| `getNumSamples()` | Yes | Yes | |
| `isConstant()` | Yes | [~] | Via sample |
| `isConstantIdentity()` | Yes | [~] | Via sample |
| `getTimeSampling()` | Yes | [ ] | |
| `get(sample, selector)` | Yes | Yes | `get_sample()` |
| `getValue(selector)` | Yes | Yes | |
| `getInheritsXforms()` | Yes | Yes | In sample |
| `getNumOps()` | Yes | Yes | In sample |
| `getChildBoundsProperty()` | Yes | [ ] | |
| `getArbGeomParams()` | Yes | Partial | `has_arb_geom_params()`, `arb_geom_param_names()` |
| `getUserProperties()` | Yes | Partial | `has_user_properties()`, `user_property_names()` |
| `OXformSchema` | Yes | [ ] | Stub only |
| XformOp types (all 12) | Yes | Yes | Translate, Scale, Rotate*, Matrix |
| XformOp hints | Yes | Yes | isXAnimated, isYAnimated, etc. |

### IPolyMesh / OPolyMesh
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IPolyMeshSchema` | Yes | Yes | `IPolyMesh` |
| `getNumSamples()` | Yes | Yes | |
| `isConstant()` | Yes | [~] | |
| `getTopologyVariance()` | Yes | Yes | `topology_variance()` |
| `getTimeSampling()` | Yes | [ ] | |
| `get(sample, selector)` | Yes | Yes | `get_sample()` |
| Positions (`P`) | Yes | Yes | `positions()` |
| Face indices (`.faceIndices`) | Yes | Yes | `face_indices()` |
| Face counts (`.faceCounts`) | Yes | Yes | `face_counts()` |
| Velocities (`v`) | Yes | Yes | `velocities` field |
| UVs param | Yes | [~] | `uvs()` - basic support |
| Normals param | Yes | [~] | `normals()` - basic support |
| Self bounds | Yes | Yes | `self_bounds` field |
| `getFaceSetNames()` | Yes | Yes | `face_set_names()` |
| `getFaceSet(name)` | Yes | [~] | `face_set()` - architectural limitation |
| `hasFaceSet(name)` | Yes | Yes | `has_face_set()` |
| `getArbGeomParams()` | Yes | Partial | `has_arb_geom_params()`, `arb_geom_param_names()` |
| `getUserProperties()` | Yes | Partial | `has_user_properties()`, `user_property_names()` |
| `OPolyMeshSchema` | Yes | [ ] | |

### ICurves / OCurves
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ICurvesSchema` | Yes | Yes | `ICurves` |
| `getNumSamples()` | Yes | Yes | |
| `isConstant()` | Yes | [~] | |
| `getTopologyVariance()` | Yes | Yes | `topology_variance()` |
| `get(sample, selector)` | Yes | Yes | `get_sample()` |
| Positions | Yes | Yes | |
| NumVertices per curve | Yes | Yes | `num_vertices()` |
| Curve type | Yes | Yes | `curve_type()` |
| Curve periodicity (wrap) | Yes | Yes | `periodicity()` |
| Basis type | Yes | Yes | `basis()` |
| Orders (variable order curves) | Yes | Yes | `orders()` |
| Knots | Yes | Yes | `knots()` |
| Position weights | Yes | Yes | `position_weights()` |
| Velocities | Yes | Yes | `velocities` field |
| UVs param | Yes | [~] | Basic via `uvs` field |
| Normals param | Yes | [~] | Basic via `normals` field |
| Widths param | Yes | Yes | `widths` field |
| Self bounds | Yes | Yes | `self_bounds` field |
| `OCurvesSchema` | Yes | [ ] | |

### IPoints / OPoints
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IPointsSchema` | Yes | Yes | `IPoints` |
| `getNumSamples()` | Yes | Yes | |
| `isConstant()` | Yes | [~] | |
| `get(sample, selector)` | Yes | Yes | `get_sample()` |
| Positions | Yes | Yes | |
| IDs | Yes | Yes | |
| Velocities | Yes | Yes | `velocities` field |
| Widths param | Yes | Yes | `widths` field |
| Self bounds | Yes | Yes | `self_bounds` field |
| `OPointsSchema` | Yes | [ ] | |

### ISubD / OSubD
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ISubDSchema` | Yes | Yes | `ISubD` |
| `getNumSamples()` | Yes | Yes | |
| `isConstant()` | Yes | [~] | |
| `getTopologyVariance()` | Yes | Yes | `topology_variance()` |
| `get(sample, selector)` | Yes | Yes | `get_sample()` |
| Positions | Yes | Yes | |
| Face indices | Yes | Yes | |
| Face counts | Yes | Yes | |
| Subdivision scheme | Yes | Yes | |
| Face varying interpolate boundary | Yes | Yes | |
| Face varying propagate corners | Yes | Yes | |
| Interpolate boundary | Yes | Yes | |
| Crease indices | Yes | Yes | |
| Crease lengths | Yes | Yes | |
| Crease sharpnesses | Yes | Yes | |
| Corner indices | Yes | Yes | |
| Corner sharpnesses | Yes | Yes | |
| Holes | Yes | Yes | |
| Velocities | Yes | [ ] | |
| UVs param | Yes | [ ] | |
| FaceSet support | Yes | [ ] | |
| `OSubDSchema` | Yes | [ ] | |

### ICamera / OCamera
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ICameraSchema` | Yes | Yes | `ICamera` |
| `getNumSamples()` | Yes | Yes | |
| `isConstant()` | Yes | [~] | |
| `get(sample, selector)` | Yes | Yes | `get_sample()` |
| Focal length | Yes | Yes | |
| Horizontal aperture | Yes | Yes | |
| Horizontal film offset | Yes | Yes | |
| Vertical aperture | Yes | Yes | |
| Vertical film offset | Yes | Yes | |
| Lens squeeze ratio | Yes | Yes | |
| Over scan left/right/top/bottom | Yes | Yes | |
| F-stop | Yes | Yes | |
| Focus distance | Yes | Yes | |
| Shutter open/close | Yes | Yes | |
| Near/far clipping plane | Yes | Yes | |
| Film back transform ops | Yes | [ ] | |
| Core properties (16 doubles) | Yes | Yes | |
| `getChildBoundsProperty()` | Yes | [ ] | |
| `getArbGeomParams()` | Yes | Partial | `has_arb_geom_params()`, `arb_geom_param_names()` |
| `getUserProperties()` | Yes | Partial | `has_user_properties()`, `user_property_names()` |
| FOV calculations | Yes | Yes | `fov_horizontal()`, `fov_vertical()` |
| `OCameraSchema` | Yes | [ ] | |

### INuPatch / ONuPatch
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `INuPatchSchema` | Yes | Yes | `INuPatch` |
| All NuPatch features | Yes | Yes | U/V knots, orders, positions, weights |
| Trim curves | Yes | [~] | Parsed in sample |
| `ONuPatchSchema` | Yes | [ ] | |

### ILight / OLight
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ILightSchema` | Yes | Yes | `ILight` |
| Camera schema (embedded) | Yes | Yes | `camera_sample()` |
| Child bounds | Yes | [~] | Via child bounds property |
| `OLightSchema` | Yes | [ ] | |

### IFaceSet / OFaceSet
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IFaceSetSchema` | Yes | Yes | `IFaceSet` |
| Face indices | Yes | Yes | `faces` field |
| FaceSet exclusivity | Yes | Yes | `exclusivity` field |
| `OFaceSetSchema` | Yes | [ ] | |

### IGeomParam / OGeomParam
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ITypedGeomParam<T>` template | Yes | Yes | `IGeomParam` |
| Indexed geometry params | Yes | Yes | `is_indexed()` |
| `getIndexed()` | Yes | Yes | `get_sample()` |
| `getExpanded()` | Yes | Yes | `get_expanded_sample()` |
| Geometry scope | Yes | Yes | `GeometryScope` enum |
| All typed variants (IV2fGeomParam, etc.) | Yes | Yes | Type aliases |
| `OTypedGeomParam<T>` | Yes | [ ] | |

### Visibility
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ObjectVisibility` enum | Yes | Yes | `ObjectVisibility` |
| `IVisibilityProperty` | Yes | Yes | `IVisibilityProperty` |
| `OVisibilityProperty` | Yes | [ ] | |
| `GetVisibilityProperty()` | Yes | Yes | `get_visibility()` |
| `GetVisibility()` | Yes | Yes | `IVisibilityProperty::get()` |
| `IsAncestorInvisible()` | Yes | [~] | Can be computed |
| `CreateVisibilityProperty()` | Yes | [ ] | |

### Other AbcGeom Features
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `GeometryScope` enum | Yes | Yes | In `core::sample` |
| `MeshTopologyVariance` enum | Yes | Yes | `TopologyVariance` in core |
| `ArchiveBounds` | Yes | [ ] | Archive-level bounds |
| `FilmBackXformOp` | Yes | [ ] | Camera film back transforms |

---

## 3. AbcCoreAbstract Module (Core Interfaces)

### DataType
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `DataType` class | Yes | Yes | |
| `getPod()` | Yes | Yes | `pod()` |
| `getExtent()` | Yes | Yes | `extent()` |
| `getNumBytes()` | Yes | Yes | `num_bytes()` |
| `setPod()` | Yes | [ ] | |
| `setExtent()` | Yes | [ ] | |
| Comparison operators | Yes | Yes | |

### PlainOldDataType
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| All POD types | Yes | Yes | Bool through Wstring |
| `PODName()` | Yes | Yes | `name()` |
| `PODNumBytes()` | Yes | Yes | `num_bytes()` |
| `PODFromName()` | Yes | [ ] | |

### TimeSampling
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `TimeSampling` class | Yes | Yes | |
| Uniform sampling | Yes | Yes | |
| Cyclic sampling | Yes | [~] | Parsed but not fully used |
| Acyclic sampling | Yes | [~] | Parsed but not fully used |
| `getNumStoredTimes()` | Yes | Yes | |
| `getStoredTimes()` | Yes | Yes | |
| `getTimeSamplingType()` | Yes | Yes | |
| `getSampleTime(index)` | Yes | Yes | `sample_time()` |
| `getFloorIndex(time)` | Yes | Yes | `floor_index()` |
| `getCeilIndex(time)` | Yes | Yes | `ceil_index()` |
| `getNearIndex(time)` | Yes | Yes | `near_index()` |

### TimeSamplingType
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| Uniform type | Yes | Yes | |
| Cyclic type | Yes | Yes | |
| Acyclic type | Yes | Yes | |
| `isUniform()` | Yes | [~] | |
| `isCyclic()` | Yes | [~] | |
| `isAcyclic()` | Yes | [~] | |
| `getNumSamplesPerCycle()` | Yes | [ ] | |
| `getTimePerCycle()` | Yes | [ ] | |

### ObjectHeader
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ObjectHeader` struct | Yes | Yes | |
| `getName()` | Yes | Yes | `name` field |
| `getFullName()` | Yes | Yes | `full_name` field |
| `getMetaData()` | Yes | Yes | `meta_data` field |

### PropertyHeader
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `PropertyHeader` struct | Yes | Yes | |
| `getName()` | Yes | Yes | `name` field |
| `getPropertyType()` | Yes | Yes | |
| `isScalar()` | Yes | Yes | |
| `isArray()` | Yes | Yes | |
| `isCompound()` | Yes | Yes | |
| `getDataType()` | Yes | Yes | `data_type` field |
| `getMetaData()` | Yes | Yes | `meta_data` field |
| `getTimeSamplingIndex()` | Yes | [~] | |

### MetaData
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `MetaData` class | Yes | [~] | HashMap<String, String> |
| `get(key)` | Yes | Yes | |
| `set(key, value)` | Yes | [ ] | |
| `getAll()` | Yes | [ ] | |
| `matches(other)` | Yes | [ ] | |
| Serialization | Yes | [~] | Basic parsing |

### ArraySample / ScalarSample
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ArraySample` class | Yes | [ ] | |
| `ArraySampleKey` | Yes | [ ] | For deduplication |
| `ScalarSample` class | Yes | [ ] | |
| Type-safe accessors | Yes | [ ] | |

### Reader/Writer Interfaces
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ArchiveReader` trait | Yes | Yes | |
| `ObjectReader` trait | Yes | Yes | |
| `CompoundPropertyReader` trait | Yes | Yes | |
| `ScalarPropertyReader` trait | Yes | Yes | |
| `ArrayPropertyReader` trait | Yes | Yes | |
| `ArchiveWriter` trait | Yes | [ ] | |
| `ObjectWriter` trait | Yes | [ ] | |
| `CompoundPropertyWriter` trait | Yes | [ ] | |
| `ScalarPropertyWriter` trait | Yes | [ ] | |
| `ArrayPropertyWriter` trait | Yes | [ ] | |

### ReadArraySampleCache
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ReadArraySampleCache` | Yes | [ ] | Caching for array samples |
| Cache lookup | Yes | [ ] | |
| Cache insertion | Yes | [ ] | |
| Thread-safe caching | Yes | [ ] | |

---

## 4. AbcMaterial Module

| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IMaterialSchema` | Yes | [ ] | Not implemented |
| `OMaterialSchema` | Yes | [ ] | |
| `IMaterial` object | Yes | [ ] | |
| `OMaterial` object | Yes | [ ] | |
| Shader targets | Yes | [ ] | |
| Shader types | Yes | [ ] | |
| Shader parameters | Yes | [ ] | |
| Network nodes | Yes | [ ] | Shader networks |
| Network connections | Yes | [ ] | |
| Network terminals | Yes | [ ] | |
| Interface parameters | Yes | [ ] | |
| Material assignment | Yes | [ ] | |
| Material flattening | Yes | [ ] | |

---

## 5. AbcCollection Module

| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ICollectionsSchema` | Yes | [ ] | Not implemented |
| `OCollectionsSchema` | Yes | [ ] | |
| `ICollections` object | Yes | [ ] | |
| `OCollections` object | Yes | [ ] | |
| `getNumCollections()` | Yes | [ ] | |
| `getCollection(index)` | Yes | [ ] | |
| `getCollection(name)` | Yes | [ ] | |
| `getCollectionName(index)` | Yes | [ ] | |

---

## 6. AbcCoreLayer Module

| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| Archive layering | Yes | [ ] | Not implemented |
| `ReadArchive` | Yes | [ ] | |
| Merge multiple archives | Yes | [ ] | |
| Override properties | Yes | [ ] | |
| Layer composition | Yes | [ ] | |

---

## 7. AbcCoreOgawa Module (Ogawa Backend)

| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ReadArchive` | Yes | Yes | |
| `WriteArchive` | Yes | [~] | Stub only |
| Object reading | Yes | Yes | |
| Object writing | Yes | [ ] | |
| Property reading | Yes | Yes | |
| Property writing | Yes | [ ] | |
| Data reading | Yes | Yes | |
| Data writing | Yes | [~] | Stub only |
| Compression support | Yes | [ ] | |
| Thread safety | Yes | [~] | Basic with parking_lot |

---

## 8. Ogawa Module (Low-Level Format)

### IStreams (Input Streams)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IStreams` class | Yes | Yes | |
| Memory-mapped I/O | Yes | Yes | |
| Buffered I/O fallback | Yes | Yes | |
| `isValid()` | Yes | Yes | |
| `isFrozen()` | Yes | Yes | |
| `version()` | Yes | Yes | |
| `size()` | Yes | Yes | |
| `read(pos, size)` | Yes | Yes | `read_bytes()` |
| Header parsing | Yes | Yes | |

### IGroup (Input Group)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IGroup` class | Yes | Yes | |
| `numChildren()` | Yes | Yes | |
| `isChildGroup(index)` | Yes | Yes | |
| `isChildData(index)` | Yes | Yes | |
| `childGroup(index)` | Yes | Yes | `group()` |
| `childData(index)` | Yes | Yes | `data()` |
| Light mode | Yes | Yes | |
| Child iteration | Yes | Yes | `children()` |

### IData (Input Data)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `IData` class | Yes | Yes | |
| `size()` | Yes | Yes | |
| `read()` | Yes | Yes | `read_all()` |
| `readInto(buffer)` | Yes | Yes | `read_into()` |
| String reading | Yes | Yes | `read_string()` |
| Slice access (mmap) | Yes | Yes | `slice()` |

### OStream / OGroup / OData (Output)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `OStream` | Yes | [~] | Basic implementation |
| `OGroup` | Yes | [~] | Basic implementation |
| `OData` | Yes | [~] | Basic implementation |
| Full write support | Yes | [ ] | |

---

## 9. Util Module

### Error Handling
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `ErrorHandler` class | Yes | [ ] | C++ error policy system |
| `ErrorHandler::Policy` | Yes | [ ] | |
| Exception types | Yes | Yes | `Error` enum |
| Error propagation | Yes | Yes | `Result<T>` |

### Types
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `bool_t` | Yes | Yes | |
| `uint8_t` - `uint64_t` | Yes | Yes | |
| `int8_t` - `int64_t` | Yes | Yes | |
| `float16_t` | Yes | [~] | As u16 |
| `float32_t`, `float64_t` | Yes | Yes | |
| `string` | Yes | Yes | |
| `wstring` | Yes | [~] | As String |
| `chrono_t` (time) | Yes | Yes | f64 |
| `index_t` | Yes | Yes | usize |

### Dimensions
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| `Dimensions` class | Yes | [ ] | Multi-dimensional array support |
| Rank | Yes | [ ] | |
| Size per dimension | Yes | [ ] | |

---

## 10. HDF5 Backend (AbcCoreHDF5)

| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| HDF5 file format support | Yes | [ ] | Not implemented, legacy format |

---

## Summary Statistics

### By Module

| Module | Total Features | Implemented | Partial | Not Implemented |
|--------|---------------|-------------|---------|-----------------|
| Abc (Archives/Objects) | ~60 | ~25 | ~5 | ~30 |
| Abc (Properties) | ~40 | ~15 | ~5 | ~20 |
| AbcGeom (Schemas) | ~150 | ~60 | ~20 | ~70 |
| AbcCoreAbstract | ~50 | ~20 | ~10 | ~20 |
| AbcMaterial | ~20 | 0 | 0 | ~20 |
| AbcCollection | ~10 | 0 | 0 | ~10 |
| AbcCoreLayer | ~10 | 0 | 0 | ~10 |
| Ogawa | ~30 | ~20 | ~5 | ~5 |
| Util | ~20 | ~10 | ~5 | ~5 |

### Overall

- **Total C++ Features**: ~390
- **Fully Implemented**: ~150 (38%)
- **Partially Implemented**: ~50 (13%)
- **Not Implemented**: ~190 (49%)

### Key Gaps

1. **Writing support**: Only stubs exist for OArchive, OObject, OProperties
2. **Material system**: AbcMaterial module not implemented
3. **Collections**: AbcCollection module not implemented
4. **Archive layering**: AbcCoreLayer not implemented
5. **HDF5 backend**: Not implemented (legacy, low priority)
6. **Instance support**: Object instances not implemented
7. **IGeomParam**: Typed geometry parameters not implemented
8. **Visibility**: Visibility property system not implemented
9. **INuPatch**: NURBS surfaces not implemented
10. **ILight**: Light schema not implemented
11. **IFaceSet**: Face sets not implemented
12. **Time-based sample selection**: Only index-based works
13. **Array sample caching**: Not implemented
14. **Error handler policies**: Uses Rust Result instead

### Strengths

1. **Core reading**: Solid implementation of archive/object/property reading
2. **Main geometry schemas**: IXform, IPolyMesh, ICurves, IPoints, ISubD, ICamera
3. **Ogawa format**: Complete reading support
4. **Memory mapping**: Efficient mmap-based file access
5. **Type safety**: Rust's type system provides better guarantees
6. **Error handling**: Idiomatic Rust error handling with Result

---

*Generated by comparing C++ Alembic 1.8.x headers with alembic-rs implementation.*
